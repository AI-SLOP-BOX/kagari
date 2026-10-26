use super::*;

// ── WAV loading & waveform extraction ───────────────────────────────────────

impl AudioBuffer {
    /// Parses a RIFF/WAVE file (PCM 8/16/24-bit, mono or stereo).
    /// Returns an error with a human-readable reason for unsupported formats.
    pub fn load_wav(path: &std::path::Path) -> Result<Self, String> {
        const MAX_WAV_BYTES: u64 = 512 * 1024 * 1024;
        let metadata = std::fs::metadata(path).map_err(|e| format!("cannot stat WAV: {}", e))?;
        if metadata.len() > MAX_WAV_BYTES {
            return Err("WAV file exceeds the 512 MiB limit".into());
        }
        let data = std::fs::read(path).map_err(|e| format!("cannot read WAV: {}", e))?;
        if data.len() < 44 || &data[0..4] != b"RIFF" || &data[8..12] != b"WAVE" {
            return Err("not a RIFF/WAVE file".into());
        }

        // Walk chunks to find fmt and data
        let mut pos = 12usize;
        let mut format_tag = 1u16;
        let mut channels = 0u16;
        let mut sample_rate = 0u32;
        let mut bits = 0u16;
        let mut audio_data: Option<&[u8]> = None;

        while pos + 8 <= data.len() {
            let id = &data[pos..pos + 4];
            let size =
                u32::from_le_bytes([data[pos + 4], data[pos + 5], data[pos + 6], data[pos + 7]])
                    as usize;
            let body_start = pos + 8;
            let body_end = body_start.saturating_add(size).min(data.len());
            match id {
                b"fmt " => {
                    let b = &data[body_start..body_end];
                    if b.len() >= 16 {
                        format_tag = u16::from_le_bytes([b[0], b[1]]);
                        channels = u16::from_le_bytes([b[2], b[3]]);
                        sample_rate = u32::from_le_bytes([b[4], b[5], b[6], b[7]]);
                        bits = u16::from_le_bytes([b[14], b[15]]);
                    }
                }
                b"data" => audio_data = Some(&data[body_start..body_end]),
                _ => {}
            }
            // Chunks are word-aligned
            pos = body_start + size + (size & 1);
        }

        if audio_data.is_none() {
            return Err("WAV has no data chunk".into());
        }
        if channels == 0 || channels > 32 || sample_rate == 0 || sample_rate > 384_000 || bits == 0
        {
            return Err("WAV has invalid fmt chunk".into());
        }
        if format_tag != 1 {
            return Err(format!(
                "unsupported WAV format tag {} (need PCM)",
                format_tag
            ));
        }
        let raw = audio_data.unwrap();
        let bytes_per_sample = (bits / 8) as usize;
        let frame_bytes = bytes_per_sample * channels as usize;

        let mut samples = Vec::with_capacity(raw.len() / frame_bytes * channels as usize);
        let mut i = 0usize;
        while i + frame_bytes <= raw.len() {
            for ch in 0..channels as usize {
                let base = i + ch * bytes_per_sample;
                let v = match bits {
                    8 => (raw[base] as f32 - 128.0) / 128.0,
                    16 => i16::from_le_bytes([raw[base], raw[base + 1]]) as f32 / 32768.0,
                    24 => {
                        let b0 = raw[base] as i32;
                        let b1 = raw[base + 1] as i32;
                        let b2 = raw[base + 2] as i32;
                        (((b2 << 16) | (b1 << 8) | b0) << 8) as f32 / 2147483648.0
                    }
                    _ => return Err(format!("unsupported bit depth {}", bits)),
                };
                samples.push(v.clamp(-1.0, 1.0));
            }
            i += frame_bytes;
        }

        Ok(Self {
            samples,
            sample_rate,
            channels,
        })
    }

    /// Decodes a supported audio container in-process into normalized f32 PCM.
    /// WAV keeps the strict PCM parser above; compressed and non-WAV formats use
    /// Symphonia so preview, waveform analysis, and audio keyframes share one path.
    pub fn load_audio(path: &std::path::Path) -> Result<Self, String> {
        const MAX_AUDIO_BYTES: u64 = 512 * 1024 * 1024;
        const MAX_DECODED_SAMPLES: usize = MAX_DECODED_AUDIO_BYTES / std::mem::size_of::<f32>();

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
        let mut samples = Vec::new();
        let mut sample_rate = 0u32;
        let mut channels = 0u16;

        loop {
            let packet = match format.next_packet() {
                Ok(packet) => packet,
                Err(SymphoniaError::ResetRequired) => break,
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
                Err(SymphoniaError::DecodeError(_)) => continue,
                Err(SymphoniaError::IoError(_)) => continue,
                Err(error) => return Err(format!("cannot decode audio packet: {}", error)),
            };
            let spec = *decoded.spec();
            let packet_channels = spec.channels.count();
            if packet_channels == 0 || packet_channels > u16::MAX as usize {
                return Err("audio track has an invalid channel count".into());
            }
            if sample_buffer.is_none() {
                let duration = decoded.capacity() as usize;
                if duration.saturating_mul(packet_channels) > MAX_DECODED_SAMPLES {
                    return Err("decoded audio exceeds the 512 MiB limit".into());
                }
                sample_buffer = Some(SampleBuffer::<f32>::new(decoded.capacity() as u64, spec));
                sample_rate = spec.rate;
                channels = packet_channels as u16;
            }
            let buffer = sample_buffer
                .as_mut()
                .expect("sample buffer initialized above");
            buffer.copy_interleaved_ref(decoded);
            let decoded_samples = buffer.samples();
            if samples.len().saturating_add(decoded_samples.len()) > MAX_DECODED_SAMPLES {
                return Err("decoded audio exceeds the 512 MiB limit".into());
            }
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
        let start_sample = ((time_sec - window_sec * 0.5).max(0.0) * self.sample_rate as f32)
            as usize
            * self.channels as usize;
        let end_sample = (((time_sec + window_sec * 0.5) * self.sample_rate as f32) as usize)
            .saturating_mul(self.channels as usize)
            .min(self.samples.len());
        let mut peak = 0.0f32;
        for s in &self.samples[start_sample.min(self.samples.len())..end_sample] {
            peak = peak.max(s.abs());
        }
        peak
    }

    /// Downsamples the buffer into `bins` peak values — ready for waveform drawing.
    pub fn waveform_peaks(&self, bins: usize) -> Vec<f32> {
        let bins = bins.max(1);
        let total = self.samples.len();
        let per_bin = (total / bins).max(1);
        let mut peaks = Vec::with_capacity(bins);
        for b in 0..bins {
            let range = b * per_bin..((b + 1) * per_bin).min(total);
            peaks.push(range.clone().next().map_or(0.0, |_| {
                self.samples[range]
                    .iter()
                    .fold(0.0f32, |m, s| m.max(s.abs()))
            }));
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
