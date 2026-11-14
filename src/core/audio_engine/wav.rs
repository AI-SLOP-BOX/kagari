use super::*;

// ── WAV loading & waveform extraction ───────────────────────────────────────

fn stereo_mix_weights(channel: symphonia::core::audio::Channels) -> Option<[f32; 2]> {
    use symphonia::core::audio::Channels as Ch;

    let center = std::f32::consts::FRAC_1_SQRT_2;
    let surround = std::f32::consts::FRAC_1_SQRT_2;
    let top = 0.5;
    Some(if channel == Ch::FRONT_LEFT {
        [1.0, 0.0]
    } else if channel == Ch::FRONT_RIGHT {
        [0.0, 1.0]
    } else if channel == Ch::FRONT_CENTRE {
        [center, center]
    } else if channel == Ch::LFE1 || channel == Ch::LFE2 {
        [0.316_227_76, 0.316_227_76]
    } else if matches!(
        channel,
        Ch::REAR_LEFT
            | Ch::SIDE_LEFT
            | Ch::FRONT_LEFT_CENTRE
            | Ch::REAR_LEFT_CENTRE
            | Ch::FRONT_LEFT_WIDE
            | Ch::FRONT_LEFT_HIGH
    ) {
        [surround, 0.0]
    } else if matches!(
        channel,
        Ch::REAR_RIGHT
            | Ch::SIDE_RIGHT
            | Ch::FRONT_RIGHT_CENTRE
            | Ch::REAR_RIGHT_CENTRE
            | Ch::FRONT_RIGHT_WIDE
            | Ch::FRONT_RIGHT_HIGH
    ) {
        [0.0, surround]
    } else if matches!(
        channel,
        Ch::REAR_CENTRE | Ch::TOP_CENTRE | Ch::TOP_FRONT_CENTRE | Ch::TOP_REAR_CENTRE
    ) {
        [top, top]
    } else if channel == Ch::TOP_FRONT_LEFT || channel == Ch::TOP_REAR_LEFT {
        [top, 0.0]
    } else if channel == Ch::TOP_FRONT_RIGHT || channel == Ch::TOP_REAR_RIGHT {
        [0.0, top]
    } else {
        return None;
    })
}

fn downmix_interleaved_to_stereo(
    samples: &[f32],
    layout: symphonia::core::audio::Channels,
    output: &mut Vec<f32>,
) -> Result<(), String> {
    let channels = layout.count();
    if channels <= 2 || !samples.len().is_multiple_of(channels) {
        return Err("multichannel audio has an invalid interleaved layout".into());
    }
    let weights: Vec<[f32; 2]> = layout
        .iter()
        .map(|channel| {
            stereo_mix_weights(channel)
                .ok_or_else(|| format!("unsupported audio channel position: {channel}"))
        })
        .collect::<Result<_, _>>()?;
    let frames = samples.len() / channels;
    let output_len = frames
        .checked_mul(2)
        .ok_or_else(|| "downmixed audio size overflows address space".to_string())?;
    output.clear();
    output
        .try_reserve(output_len)
        .map_err(|error| format!("cannot allocate downmixed audio: {error}"))?;
    for frame in samples.chunks_exact(channels) {
        let mut left = 0.0f32;
        let mut right = 0.0f32;
        for (sample, [left_weight, right_weight]) in frame.iter().zip(&weights) {
            left += sample * left_weight;
            right += sample * right_weight;
        }
        output.extend_from_slice(&[left, right]);
    }
    Ok(())
}

impl AudioBuffer {
    /// Parses integer PCM RIFF/WAVE files and downmixes WAVE_FORMAT_EXTENSIBLE
    /// multichannel layouts with a recognized speaker mask to stereo.
    /// Returns an error with a human-readable reason for unsupported formats.
    pub fn load_wav(path: &std::path::Path) -> Result<Self, String> {
        use std::io::{Read, Seek, SeekFrom};
        const MAX_WAV_BYTES: u64 = 512 * 1024 * 1024;
        const INPUT_CHUNK_BYTES: usize = 64 * 1024;
        let mut file = std::fs::File::open(path).map_err(|e| format!("cannot open WAV: {}", e))?;
        let metadata = file
            .metadata()
            .map_err(|e| format!("cannot stat WAV: {}", e))?;
        if metadata.len() > MAX_WAV_BYTES {
            return Err("WAV file exceeds the 512 MiB limit".into());
        }
        let mut header = [0u8; 12];
        file.read_exact(&mut header)
            .map_err(|error| format!("cannot read WAV header: {error}"))?;
        if &header[0..4] != b"RIFF" || &header[8..12] != b"WAVE" {
            return Err("not a RIFF/WAVE file".into());
        }
        let riff_size = u32::from_le_bytes([header[4], header[5], header[6], header[7]]) as u64;
        let riff_end = 8u64
            .checked_add(riff_size)
            .filter(|end| *end <= metadata.len() && *end >= 12)
            .ok_or_else(|| "WAV has an invalid RIFF size".to_string())?;

        // Walk chunks to find fmt and data
        let mut pos = 12u64;
        let mut format_tag = 1u16;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits = 0u16;
        let mut block_align = 0u16;
        let mut has_format = false;
        let mut channel_layout = None;
        let mut audio_data: Option<(u64, usize)> = None;

        while pos.checked_add(8).is_some_and(|end| end <= riff_end) {
            file.seek(SeekFrom::Start(pos))
                .map_err(|error| format!("cannot seek WAV chunks: {error}"))?;
            let mut chunk_header = [0u8; 8];
            file.read_exact(&mut chunk_header)
                .map_err(|error| format!("cannot read WAV chunk header: {error}"))?;
            let id = &chunk_header[..4];
            let size = u32::from_le_bytes([
                chunk_header[4],
                chunk_header[5],
                chunk_header[6],
                chunk_header[7],
            ]) as u64;
            let body_start = pos + 8;
            let body_end = body_start
                .checked_add(size)
                .filter(|end| *end <= riff_end)
                .ok_or_else(|| "WAV contains a truncated chunk".to_string())?;
            match id {
                b"fmt " => {
                    if size < 16 {
                        return Err("WAV has a truncated fmt chunk".into());
                    }
                    let mut b = [0u8; 40];
                    let fmt_bytes = (size as usize).min(b.len());
                    file.read_exact(&mut b[..fmt_bytes])
                        .map_err(|error| format!("cannot read WAV fmt chunk: {error}"))?;
                    format_tag = u16::from_le_bytes([b[0], b[1]]);
                    channels = u16::from_le_bytes([b[2], b[3]]);
                    sample_rate = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
                    block_align = u16::from_le_bytes([b[12], b[13]]);
                    bits = u16::from_le_bytes([b[14], b[15]]);
                    if format_tag == 0xfffe {
                        if size < 40 {
                            return Err("WAV has a truncated extensible fmt chunk".into());
                        }
                        let extension_size = u16::from_le_bytes([b[16], b[17]]) as u64;
                        if extension_size < 22 || 18 + extension_size > size {
                            return Err("WAV has an invalid extensible fmt size".into());
                        }
                        let valid_bits = u16::from_le_bytes([b[18], b[19]]);
                        if valid_bits == 0 || valid_bits > bits {
                            return Err("WAV has an invalid valid-bits-per-sample value".into());
                        }
                        let channel_mask = u32::from_le_bytes([b[20], b[21], b[22], b[23]]);
                        let subformat = u32::from_le_bytes([b[24], b[25], b[26], b[27]]);
                        let pcm_guid_tail = [
                            0x00, 0x00, 0x10, 0x00, 0x80, 0x00, 0x00, 0xaa, 0x00, 0x38, 0x9b, 0x71,
                        ];
                        if subformat != 1 || b[28..40] != pcm_guid_tail {
                            return Err("unsupported WAVE_FORMAT_EXTENSIBLE subformat".into());
                        }
                        format_tag = 1;
                        if channel_mask != 0 {
                            let layout = symphonia::core::audio::Channels::from_bits(channel_mask)
                                .filter(|layout| layout.count() == channels as usize)
                                .ok_or_else(|| "WAV has an invalid channel mask".to_string())?;
                            channel_layout = Some(layout);
                        }
                    }
                    has_format = true;
                }
                b"data" => audio_data = Some((body_start, size as usize)),
                _ => {}
            }
            // Chunks are word-aligned
            pos = body_end
                .checked_add(size & 1)
                .filter(|end| *end <= riff_end)
                .ok_or_else(|| "WAV contains an incomplete chunk padding byte".to_string())?;
        }
        if pos != riff_end {
            return Err("WAV ends with an incomplete chunk header".into());
        }

        if !has_format {
            return Err("WAV has no fmt chunk".into());
        }
        if audio_data.is_none() {
            return Err("WAV has no data chunk".into());
        }
        if channels == 0 || sample_rate == 0 || sample_rate > 384_000 || bits == 0 {
            return Err("WAV has invalid fmt chunk".into());
        }
        if channels > 2 && channel_layout.is_none() {
            return Err(format!(
                "WAV has {channels} channels but no speaker layout; multichannel PCM needs WAVE_FORMAT_EXTENSIBLE speaker metadata."
            ));
        }
        if format_tag != 1 {
            return Err(format!(
                "unsupported WAV format tag {} (need PCM)",
                format_tag
            ));
        }
        let (audio_data_offset, audio_data_len) = audio_data.unwrap();
        if audio_data_len == 0 {
            return Err("WAV data chunk is empty".into());
        }
        if !matches!(bits, 8 | 16 | 24 | 32) {
            return Err(format!("unsupported bit depth {}", bits));
        }
        let bytes_per_sample = (bits / 8) as usize;
        let frame_bytes = bytes_per_sample
            .checked_mul(channels as usize)
            .ok_or_else(|| "WAV frame size overflows address space".to_string())?;
        if block_align as usize != frame_bytes {
            return Err("WAV block alignment does not match its sample format".into());
        }
        if !audio_data_len.is_multiple_of(frame_bytes) {
            return Err("WAV data chunk ends in a partial sample frame".into());
        }
        let output_channels = if channels > 2 { 2 } else { channels as usize };
        let sample_count = (audio_data_len / frame_bytes)
            .checked_mul(output_channels)
            .filter(|count| *count <= MAX_DECODED_AUDIO_BYTES / std::mem::size_of::<f32>())
            .ok_or_else(|| "decoded WAV exceeds the 512 MiB limit".to_string())?;

        let channel_weights = if channels > 2 {
            let layout = channel_layout.expect("multichannel layout validated above");
            Some(
                layout
                    .iter()
                    .map(|channel| {
                        stereo_mix_weights(channel)
                            .ok_or_else(|| format!("unsupported WAV speaker position: {channel}"))
                    })
                    .collect::<Result<Vec<_>, _>>()?,
            )
        } else {
            None
        };

        let mut samples = Vec::new();
        samples
            .try_reserve_exact(sample_count)
            .map_err(|error| format!("cannot allocate decoded WAV samples: {error}"))?;
        file.seek(SeekFrom::Start(audio_data_offset))
            .map_err(|error| format!("cannot seek WAV audio data: {error}"))?;
        let frames_per_chunk = (INPUT_CHUNK_BYTES / frame_bytes).max(1);
        let chunk_bytes = frames_per_chunk * frame_bytes;
        let mut raw = vec![0u8; chunk_bytes];
        let mut remaining = audio_data_len;
        while remaining > 0 {
            let bytes_to_read = remaining.min(chunk_bytes);
            file.read_exact(&mut raw[..bytes_to_read])
                .map_err(|error| format!("cannot read WAV audio data: {error}"))?;
            for frame in raw[..bytes_to_read].chunks_exact(frame_bytes) {
                let mut left = 0.0f32;
                let mut right = 0.0f32;
                for ch in 0..channels as usize {
                    let base = ch * bytes_per_sample;
                    let v = match bits {
                        8 => (frame[base] as f32 - 128.0) / 128.0,
                        16 => i16::from_le_bytes([frame[base], frame[base + 1]]) as f32 / 32768.0,
                        24 => {
                            let b0 = frame[base] as i32;
                            let b1 = frame[base + 1] as i32;
                            let b2 = frame[base + 2] as i32;
                            (((b2 << 16) | (b1 << 8) | b0) << 8) as f32 / 2147483648.0
                        }
                        32 => {
                            i32::from_le_bytes([
                                frame[base],
                                frame[base + 1],
                                frame[base + 2],
                                frame[base + 3],
                            ]) as f32
                                / 2147483648.0
                        }
                        _ => unreachable!("bit depth was validated above"),
                    };
                    let v = v.clamp(-1.0, 1.0);
                    if let Some(weights) = &channel_weights {
                        left += v * weights[ch][0];
                        right += v * weights[ch][1];
                    } else {
                        samples.push(v);
                    }
                }
                if channel_weights.is_some() {
                    samples.extend_from_slice(&[left, right]);
                }
            }
            remaining -= bytes_to_read;
        }

        Ok(Self {
            samples,
            sample_rate,
            channels: output_channels as u16,
        })
    }

    /// Decodes a supported audio container in-process into normalized f32 PCM.
    /// WAV keeps the strict PCM parser above; compressed and non-WAV formats use
    /// Symphonia so preview, waveform analysis, and audio keyframes share one path.
    pub fn load_audio(path: &std::path::Path) -> Result<Self, String> {
        const MAX_AUDIO_BYTES: u64 = 512 * 1024 * 1024;
        const MAX_DECODED_SAMPLES: usize = MAX_DECODED_AUDIO_BYTES / std::mem::size_of::<f32>();
        const MAX_DECODED_PACKET_BYTES: usize = 64 * 1024 * 1024;
        const MAX_DECODED_PACKET_SAMPLES: usize =
            MAX_DECODED_PACKET_BYTES / std::mem::size_of::<f32>();

        let path_str = path.to_string_lossy();
        validate_audio_path(&path_str)?;
        let metadata =
            std::fs::metadata(path).map_err(|e| format!("cannot stat audio file: {}", e))?;
        if metadata.len() > MAX_AUDIO_BYTES {
            return Err("audio file exceeds the 512 MiB limit".into());
        }
        if path
            .extension()
            .and_then(|ext| ext.to_str())
            .is_some_and(|ext| ext.eq_ignore_ascii_case("wav"))
        {
            return Self::load_wav(path);
        }

        use symphonia::core::audio::SampleBuffer;
        use symphonia::core::codecs::{DecoderOptions, CODEC_TYPE_NULL};
        use symphonia::core::errors::Error as SymphoniaError;
        use symphonia::core::formats::FormatOptions;
        use symphonia::core::io::MediaSourceStream;
        use symphonia::core::meta::MetadataOptions;
        use symphonia::core::probe::Hint;

        let file =
            std::fs::File::open(path).map_err(|e| format!("cannot open audio file: {}", e))?;
        let mss = MediaSourceStream::new(Box::new(file), Default::default());
        let mut hint = Hint::new();
        if let Some(ext) = path.extension().and_then(|ext| ext.to_str()) {
            hint.with_extension(ext);
        }
        let probed = symphonia::default::get_probe()
            .format(
                &hint,
                mss,
                &FormatOptions::default(),
                &MetadataOptions::default(),
            )
            .map_err(|e| format!("cannot probe audio file: {}", e))?;
        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|track| track.codec_params.codec != CODEC_TYPE_NULL)
            .ok_or_else(|| "audio file has no supported audio track".to_string())?;
        let track_id = track.id;
        let mut decoder = symphonia::default::get_codecs()
            .make(&track.codec_params, &DecoderOptions::default())
            .map_err(|e| format!("cannot create audio decoder: {}", e))?;
        let mut sample_buffer: Option<SampleBuffer<f32>> = None;
        let mut sample_spec: Option<(u32, symphonia::core::audio::Channels)> = None;
        let mut downmixed_samples = Vec::new();
        let mut samples = Vec::new();
        let mut sample_rate = 0u32;
        let mut channels = 0u16;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::ResetRequired) => {
                    return Err(
                        "audio stream changed configuration and cannot be decoded continuously"
                            .into(),
                    )
                }
                Err(SymphoniaError::IoError(error))
                    if error.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break
                }
                Err(error) => return Err(format!("cannot read audio packet: {}", error)),
            };
            if packet.track_id() != track_id {
                continue;
            }
            let decoded = match decoder.decode(&packet) {
                Ok(decoded) => decoded,
                Err(SymphoniaError::DecodeError(error)) => {
                    return Err(format!("cannot decode audio packet: {error}"))
                }
                Err(SymphoniaError::IoError(error)) => {
                    return Err(format!("cannot read audio packet: {error}"))
                }
                Err(error) => return Err(format!("cannot decode audio packet: {}", error)),
            };
            let spec = *decoded.spec();
            let packet_channels = spec.channels.count();
            if packet_channels == 0 || packet_channels > u16::MAX as usize {
                return Err("audio track has an invalid channel count".into());
            }
            let packet_capacity = decoded.capacity() as usize;
            if packet_capacity.saturating_mul(packet_channels) > MAX_DECODED_PACKET_SAMPLES {
                return Err("decoded audio packet exceeds the 64 MiB limit".into());
            }
            if sample_spec.is_some_and(|(rate, channel_layout)| {
                rate != spec.rate || channel_layout != spec.channels
            }) {
                return Err("audio track changes sample rate or channel layout mid-stream".into());
            }
            if sample_spec.is_none() {
                sample_spec = Some((spec.rate, spec.channels));
                sample_rate = spec.rate;
                channels = if packet_channels > 2 {
                    2
                } else {
                    packet_channels as u16
                };
            }
            if sample_buffer
                .as_ref()
                .is_none_or(|buffer| buffer.capacity() < packet_capacity * packet_channels)
            {
                sample_buffer = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
            }
            let buffer = sample_buffer
                .as_mut()
                .expect("sample buffer initialized above");
            buffer.clear();
            buffer.copy_interleaved_ref(decoded);
            let decoded_samples = buffer.samples();
            let decoded_samples = if packet_channels > 2 {
                downmix_interleaved_to_stereo(
                    decoded_samples,
                    spec.channels,
                    &mut downmixed_samples,
                )?;
                downmixed_samples.as_slice()
            } else {
                decoded_samples
            };
            let new_sample_count = samples
                .len()
                .checked_add(decoded_samples.len())
                .filter(|count| *count <= MAX_DECODED_SAMPLES)
                .ok_or_else(|| "decoded audio exceeds the 512 MiB limit".to_string())?;
            samples
                .try_reserve(new_sample_count - samples.len())
                .map_err(|error| format!("cannot allocate decoded audio samples: {error}"))?;
            samples.extend_from_slice(decoded_samples);
        }

        if samples.is_empty() || sample_rate == 0 || channels == 0 {
            return Err("audio file contains no decodable samples".into());
        }
        Ok(Self {
            samples,
            sample_rate,
            channels,
        })
    }

    /// Peak amplitude in [0, 1] at a given time offset (± window/2 seconds).
    pub fn peak_at(&self, time_sec: f32, window_sec: f32) -> f32 {
        if !time_sec.is_finite()
            || !window_sec.is_finite()
            || window_sec < 0.0
            || self.sample_rate == 0
            || self.channels == 0
        {
            return 0.0;
        }
        let channels = usize::from(self.channels);
        let frame_count = self.samples.len() / channels;
        let half_window = f64::from(window_sec) * 0.5;
        let sample_rate = f64::from(self.sample_rate);
        let max_frame = frame_count as f64;
        let start_frame = ((f64::from(time_sec) - half_window).max(0.0) * sample_rate)
            .clamp(0.0, max_frame) as usize;
        let end_frame = ((f64::from(time_sec) + half_window).max(0.0) * sample_rate)
            .clamp(0.0, max_frame) as usize;
        let start_sample = start_frame.saturating_mul(channels).min(self.samples.len());
        let end_sample = end_frame.saturating_mul(channels).min(self.samples.len());
        let mut peak = 0.0f32;
        for s in &self.samples[start_sample..end_sample.max(start_sample)] {
            peak = peak.max(s.abs());
        }
        peak
    }

    /// Downsamples the buffer into `bins` peak values — ready for waveform drawing.
    pub fn waveform_peaks(&self, bins: usize) -> Vec<f32> {
        let total = self.samples.len();
        const MAX_WAVEFORM_BINS: usize = 1_000_000;
        let bins = bins.max(1).min(total.max(1)).min(MAX_WAVEFORM_BINS);
        let mut peaks = Vec::new();
        if peaks.try_reserve_exact(bins).is_err() {
            return peaks;
        }
        for b in 0..bins {
            let range_start = (b as u128 * total as u128 / bins as u128) as usize;
            let range_end = ((b + 1) as u128 * total as u128 / bins as u128) as usize;
            let peak = self.samples[range_start..range_end]
                .iter()
                .fold(0.0f32, |maximum, sample| maximum.max(sample.abs()));
            peaks.push(peak);
        }
        peaks
    }
}

#[cfg(test)]
mod wav_tests {
    use super::*;

    /// Writes a minimal 16-bit PCM WAV file for testing.
    fn write_test_wav(path: &std::path::Path, samples: &[f32], rate: u32) {
        use std::io::Write;
        let mut f = std::fs::File::create(path).unwrap();
        let data_len = (samples.len() * 2) as u32;
        f.write_all(b"RIFF").unwrap();
        f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
        f.write_all(b"WAVE").unwrap();
        f.write_all(b"fmt ").unwrap();
        f.write_all(&16u32.to_le_bytes()).unwrap();
        f.write_all(&1u16.to_le_bytes()).unwrap(); // PCM
        f.write_all(&1u16.to_le_bytes()).unwrap(); // mono
        f.write_all(&rate.to_le_bytes()).unwrap();
        f.write_all(&(rate * 2).to_le_bytes()).unwrap(); // byte rate
        f.write_all(&2u16.to_le_bytes()).unwrap(); // block align
        f.write_all(&16u16.to_le_bytes()).unwrap();
        f.write_all(b"data").unwrap();
        f.write_all(&data_len.to_le_bytes()).unwrap();
        for s in samples {
            let v = (s.clamp(-1.0, 1.0) * 32767.0) as i16;
            f.write_all(&v.to_le_bytes()).unwrap();
        }
    }

    #[test]
    fn test_load_wav_roundtrip_and_peaks() {
        let path = std::env::temp_dir().join("kagari_test_tone.wav");
        let rate = 48000u32;
        // 1-second sine at 440 Hz with amplitude 0.5
        let samples: Vec<f32> = (0..rate as usize)
            .map(|i| 0.5 * (2.0 * std::f32::consts::PI * 440.0 * i as f32 / rate as f32).sin())
            .collect();
        write_test_wav(&path, &samples, rate);

        let buf = AudioBuffer::load_wav(&path).expect("wav must parse");
        assert_eq!(buf.sample_rate, rate);
        assert_eq!(buf.channels, 1);
        assert_eq!(buf.samples.len(), samples.len());

        // Peak near the max of the sine should be close to 0.5
        let peak = buf.peak_at(0.25, 0.01);
        assert!(peak > 0.45 && peak <= 0.51, "unexpected peak {}", peak);

        // Waveform bins cover the file and stay normalized
        let peaks = buf.waveform_peaks(100);
        assert_eq!(peaks.len(), 100);
        assert!(peaks.iter().all(|p| (0.0..=1.0).contains(p)));

        let _ = std::fs::remove_file(&path);
    }

    #[test]
    fn resample_preserves_mono_channel_layout() {
        let input = AudioBuffer {
            samples: vec![0.0, 0.5, 1.0, 0.5],
            sample_rate: 4,
            channels: 1,
        };
        let output = input.resample(8).unwrap();
        assert_eq!(output.channels, 1);
        assert_eq!(output.samples.len(), 8);
        assert!((output.samples[2] - 0.5).abs() < 0.001);
        assert!((output.samples[4] - 1.0).abs() < 0.001);
    }

    #[test]
    fn test_invalid_wav_files_error_cleanly() {
        let dir = std::env::temp_dir().join(format!("kagari_wav_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);

        // Missing file
        assert!(AudioBuffer::load_wav(&dir.join("nope.wav")).is_err());

        // Garbage bytes
        let bad = dir.join("bad.wav");
        std::fs::write(&bad, b"totally not a wav file at all").unwrap();
        assert!(AudioBuffer::load_wav(&bad).is_err());

        // Truncated header
        let trunc = dir.join("trunc.wav");
        std::fs::write(&trunc, b"RIFF----WAVEfmt ").unwrap();
        assert!(AudioBuffer::load_wav(&trunc).is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn generic_audio_loader_keeps_wav_compatibility() {
        let dir = std::env::temp_dir().join(format!("kagari_audio_loader_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("tone.wav");
        write_test_wav(&path, &[0.0, 0.25, -0.25, 0.5], 8000);

        let buf = AudioBuffer::load_audio(&path).expect("generic loader must accept WAV");
        assert_eq!(buf.sample_rate, 8000);
        assert_eq!(buf.channels, 1);
        assert_eq!(buf.samples.len(), 4);

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn unsupported_compressed_audio_returns_error_cleanly() {
        let dir = std::env::temp_dir().join(format!("kagari_audio_bad_{}", std::process::id()));
        let _ = std::fs::create_dir_all(&dir);
        let path = dir.join("broken.mp3");
        std::fs::write(&path, b"not an MP3").unwrap();

        let result = AudioBuffer::load_audio(&path);
        assert!(result.is_err());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
