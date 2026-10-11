use super::*;

/// Mix all audio sources in the composition for the given frame range and write
/// to a temporary WAV file. Returns the path to the mixed WAV.
pub fn mix_composition_to_wav(
    comp: &Composition,
    start_frame: u32,
    end_frame: u32,
    sample_rate: u32,
    mixer: Option<&[crate::core::audio_types::MixerChannel]>,
    dsp: &MasterDspParams,
) -> std::io::Result<Option<std::path::PathBuf>> {
    use std::io::{BufWriter, Error, ErrorKind, Write};

    let fps = comp.fps.max(1);
    if end_frame <= start_frame {
        return Ok(None);
    }
    let sample_boundary =
        |frame: u32| -> u64 { u64::from(frame) * u64::from(sample_rate) / u64::from(fps) };
    let sample_count = sample_boundary(end_frame).saturating_sub(sample_boundary(start_frame));
    let data_len = sample_count
        .checked_mul(4)
        .and_then(|bytes| u32::try_from(bytes).ok())
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "mixed WAV exceeds RIFF size limit"))?;
    let riff_size = 36u32
        .checked_add(data_len)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "mixed WAV exceeds RIFF size limit"))?;
    let byte_rate = sample_rate
        .checked_mul(4)
        .ok_or_else(|| Error::new(ErrorKind::InvalidInput, "audio sample rate is too large"))?;
    let block_align = 4u16;
    let bits_per_sample = 16u16;

    static MIX_FILE_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let sequence = MIX_FILE_COUNTER.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
    let path = std::env::temp_dir().join(format!(
        "kagari_mix_{}_{}.wav",
        std::process::id(),
        sequence
    ));
    let f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)?;
    let mut has_loaded_source = false;
    let result = (|| -> std::io::Result<()> {
        let mut writer = BufWriter::new(f);
        writer.write_all(b"RIFF")?;
        writer.write_all(&riff_size.to_le_bytes())?;
        writer.write_all(b"WAVE")?;
        writer.write_all(b"fmt ")?;
        writer.write_all(&16u32.to_le_bytes())?;
        writer.write_all(&1u16.to_le_bytes())?;
        writer.write_all(&2u16.to_le_bytes())?;
        writer.write_all(&sample_rate.to_le_bytes())?;
        writer.write_all(&byte_rate.to_le_bytes())?;
        writer.write_all(&block_align.to_le_bytes())?;
        writer.write_all(&bits_per_sample.to_le_bytes())?;
        writer.write_all(b"data")?;
        writer.write_all(&data_len.to_le_bytes())?;

        let mut dsp_state = MasterDspState::default();
        let mut encoded = Vec::new();
        for frame in start_frame..end_frame {
            let frame_samples = sample_boundary(frame + 1).saturating_sub(sample_boundary(frame));
            let frame_samples = usize::try_from(frame_samples)
                .map_err(|_| Error::new(ErrorKind::InvalidInput, "audio frame is too large"))?;
            let (stereo_buf, meter) = mix_audio_sources_for_frame_with_state(
                comp,
                frame,
                sample_rate,
                frame_samples,
                mixer,
                dsp,
                &mut dsp_state,
            );
            has_loaded_source |= meter.loaded_sources > 0;
            if !meter.source_errors.is_empty() {
                let details = meter
                    .source_errors
                    .iter()
                    .map(|issue| format!("{} ({}): {}", issue.layer, issue.path, issue.reason))
                    .collect::<Vec<_>>()
                    .join("; ");
                return Err(Error::new(
                    ErrorKind::InvalidData,
                    format!("audio mix has unreadable source(s): {details}"),
                ));
            }
            encoded.clear();
            encoded.reserve(stereo_buf.len().saturating_mul(2));
            for sample in stereo_buf {
                let value = (sample.clamp(-1.0, 1.0) * 32767.0) as i16;
                encoded.extend_from_slice(&value.to_le_bytes());
            }
            writer.write_all(&encoded)?;
        }
        writer.flush()
    })();
    if let Err(error) = result {
        let _ = std::fs::remove_file(&path);
        return Err(error);
    }

    // A valid export range without any active, readable audio source should
    // not produce a silent WAV that callers mistake for a prepared mix.
    if !has_loaded_source {
        let _ = std::fs::remove_file(&path);
        return Ok(None);
    }

    Ok(Some(path))
}
