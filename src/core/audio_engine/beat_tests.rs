use super::*;

fn write_test_wav(path: &std::path::Path, sr: u32, clicks_at_sec: &[f32]) {
    let total = (sr as f32 * 2.0) as usize;
    let mut samples = vec![0i16; total];
    for &t in clicks_at_sec {
        let idx = (t * sr as f32) as usize;
        let click_len = sr as usize / 50;
        for k in 0..click_len {
            if idx + k < total {
                samples[idx + k] = ((k as f32 / click_len as f32) * 30000.0) as i16;
            }
        }
    }
    // Minimal 44-byte RIFF/WAVE header, mono 16-bit
    let data_len = (samples.len() * 2) as u32;
    let mut bytes: Vec<u8> = Vec::with_capacity(44 + data_len as usize);
    bytes.extend_from_slice(b"RIFF");
    bytes.extend_from_slice(&(36 + data_len).to_le_bytes());
    bytes.extend_from_slice(b"WAVEfmt ");
    bytes.extend_from_slice(&16u32.to_le_bytes());
    bytes.extend_from_slice(&1u16.to_le_bytes()); // PCM
    bytes.extend_from_slice(&1u16.to_le_bytes()); // mono
    bytes.extend_from_slice(&sr.to_le_bytes());
    bytes.extend_from_slice(&(sr * 2).to_le_bytes()); // byte rate
    bytes.extend_from_slice(&2u16.to_le_bytes()); // block align
    bytes.extend_from_slice(&16u16.to_le_bytes()); // bits
    bytes.extend_from_slice(b"data");
    bytes.extend_from_slice(&data_len.to_le_bytes());
    for s in samples {
        bytes.extend_from_slice(&s.to_le_bytes());
    }
    std::fs::write(path, bytes).unwrap();
}

#[test]
fn test_detect_beat_frames_finds_clicks() {
    let dir = std::env::temp_dir().join("kagari_beat_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("clicks.wav");
    write_test_wav(&path, 44100, &[0.2, 0.8, 1.4]);
    let beats = detect_beat_frames(&path, 60, 30.0);
    assert!(
        beats.contains(&6),
        "expected onset near frame 6, got {:?}",
        beats
    );
    assert!(
        beats.contains(&24),
        "expected onset near frame 24, got {:?}",
        beats
    );
    assert!(
        beats.len() >= 3 && beats.len() <= 12,
        "sane beat count, got {:?}",
        beats
    );
    let _ = std::fs::remove_file(&path);
}

#[test]
fn test_detect_beat_frames_silence_is_empty() {
    let dir = std::env::temp_dir().join("kagari_beat_test");
    std::fs::create_dir_all(&dir).unwrap();
    let path = dir.join("silence.wav");
    write_test_wav(&path, 22050, &[]);
    let beats = detect_beat_frames(&path, 60, 30.0);
    assert!(
        beats.is_empty(),
        "silence should have no beats, got {:?}",
        beats
    );
    let _ = std::fs::remove_file(&path);
}
