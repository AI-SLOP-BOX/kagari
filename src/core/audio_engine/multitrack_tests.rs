use super::*;
use crate::core::keyframe::Keyframe;
use crate::core::property::Animatable;
use crate::core::timeline::{Composition, Layer, LayerType};

fn write_wav(path: &std::path::Path, samples: &[f32], rate: u32) {
    use std::io::Write;
    let mut f = std::fs::File::create(path).unwrap();
    let data_len = (samples.len() * 2) as u32;
    f.write_all(b"RIFF").unwrap();
    f.write_all(&(36 + data_len).to_le_bytes()).unwrap();
    f.write_all(b"WAVE").unwrap();
    f.write_all(b"fmt ").unwrap();
    f.write_all(&16u32.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&1u16.to_le_bytes()).unwrap();
    f.write_all(&rate.to_le_bytes()).unwrap();
    f.write_all(&(rate * 2).to_le_bytes()).unwrap();
    f.write_all(&2u16.to_le_bytes()).unwrap();
    f.write_all(&16u16.to_le_bytes()).unwrap();
    f.write_all(b"data").unwrap();
    f.write_all(&data_len.to_le_bytes()).unwrap();
    for s in samples {
        f.write_all(&((s.clamp(-1.0, 1.0) * 32767.0) as i16).to_le_bytes())
            .unwrap();
    }
}

#[test]
fn test_multitrack_mix_sums_two_wavs() {
    let dir = std::env::temp_dir().join(format!("kagari_mix_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let rate = 48000u32;
    let wav_a = dir.join("a.wav");
    let wav_b = dir.join("b.wav");
    // Track A: constant 0.5, Track B: constant 0.25
    let samples_a: Vec<f32> = vec![0.5; rate as usize];
    let samples_b: Vec<f32> = vec![0.25; rate as usize];
    write_wav(&wav_a, &samples_a, rate);
    write_wav(&wav_b, &samples_b, rate);

    let mut comp = Composition::new("c".into(), "Mix".into(), 64, 64, 30, 30);
    let mut la = Layer::new(
        "a".into(),
        "TrackA".into(),
        LayerType::Video {
            source: "a".into(),
            frames_dir: "/tmp/na".into(),
            frame_count: 10,
            audio_wav: Some(wav_a.to_string_lossy().to_string()),
            speed: 1.0,
        },
        30,
    );
    la.in_frame = 0;
    la.out_frame = 30;
    let mut lb = Layer::new(
        "b".into(),
        "TrackB".into(),
        LayerType::Video {
            source: "b".into(),
            frames_dir: "/tmp/nb".into(),
            frame_count: 10,
            audio_wav: Some(wav_b.to_string_lossy().to_string()),
            speed: 1.0,
        },
        30,
    );
    lb.in_frame = 0;
    lb.out_frame = 30;
    comp.layers.push(la);
    comp.layers.push(lb);

    let bypass_dsp = MasterDspParams {
        eq_highpass: 0.0,
        eq_lowpass: 24000.0,
        eq_mid_gain: 0.0,
        eq_mid_freq: 1000.0,
        comp_threshold: 0.0,
        comp_ratio: 1.0,
        comp_attack: 0.1,
        comp_release: 10.0,
        comp_makeup: 0.0,
        ..MasterDspParams::bypass()
    };
    let (mix, meter) = mix_audio_sources_for_frame(&comp, 0, rate, 480, None, &bypass_dsp);
    // Sum of 0.5 + 0.25 at sample 0 ≈ 0.75
    assert!((mix[0] - 0.75).abs() < 0.01, "mix[0] = {}", mix[0]);
    // Meter reflects the combined level
    assert!(meter.peak_db_left > -5.0, "peak {} dB", meter.peak_db_left);
    assert_eq!(meter.track_peaks.len(), 2);
    assert!((meter.track_peaks[0].0 - 0.5).abs() < 0.01);
    assert!((meter.track_peaks[1].0 - 0.25).abs() < 0.01);
    let _ = Animatable::<f32>::new_constant;
    let _ = Keyframe::new(0, 0.0f32, crate::core::keyframe::InterpolationType::Linear);

    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn mixed_wav_uses_composition_fps_for_audio_duration() {
    let dir = std::env::temp_dir().join(format!("kagari_mix_duration_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.wav");
    write_wav(&source, &vec![0.25; 48_000], 48_000);
    let mut comp = Composition::new("c".into(), "Duration".into(), 64, 64, 30, 30);
    comp.layers.push(Layer::new(
        "audio".into(),
        "Audio".into(),
        LayerType::Audio {
            path: source.to_string_lossy().into_owned(),
            volume: Animatable::new_constant(1.0),
        },
        30,
    ));
    let path = mix_composition_to_wav(&comp, 0, 30, 48_000, None, &MasterDspParams::bypass())
        .expect("audio mix should succeed")
        .expect("composition audio should produce a mixed WAV");
    let audio = AudioBuffer::load_wav(&path).expect("mixed WAV must be readable");
    assert_eq!(audio.sample_rate, 48_000);
    assert_eq!(audio.samples.len(), 30 * 1_600 * 2);
    let _ = std::fs::remove_file(path);
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn empty_composition_does_not_emit_a_silent_mix_file() {
    let comp = Composition::new("c".into(), "Empty".into(), 64, 64, 30, 30);
    assert!(
        mix_composition_to_wav(&comp, 0, 30, 48_000, None, &MasterDspParams::bypass())
            .expect("empty mix request is valid")
            .is_none()
    );
}

#[test]
fn concurrent_mixes_receive_distinct_output_paths() {
    let dir = std::env::temp_dir().join(format!("kagari_mix_concurrent_{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let source = dir.join("source.wav");
    write_wav(&source, &vec![0.25; 48_000], 48_000);
    let mut comp = Composition::new("c".into(), "Concurrent mix".into(), 64, 64, 30, 2);
    comp.layers.push(Layer::new(
        "audio".into(),
        "Audio".into(),
        LayerType::Audio {
            path: source.to_string_lossy().into_owned(),
            volume: Animatable::new_constant(1.0),
        },
        2,
    ));
    let handles: Vec<_> = (0..2)
        .map(|_| {
            let comp = comp.clone();
            std::thread::spawn(move || {
                mix_composition_to_wav(&comp, 0, 2, 48_000, None, &MasterDspParams::bypass())
                    .expect("concurrent mix should succeed")
                    .expect("concurrent mix must produce a WAV")
            })
        })
        .collect();
    let paths: Vec<_> = handles
        .into_iter()
        .map(|handle| handle.join().expect("mix worker must not panic"))
        .collect();

    assert_ne!(paths[0], paths[1]);
    for path in paths {
        assert!(path.is_file());
        let _ = std::fs::remove_file(path);
    }
    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn standalone_audio_layer_is_included_in_mix() {
    let dir = std::env::temp_dir().join(format!("kagari_audio_layer_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let wav = dir.join("voice.wav");
    write_wav(&wav, &vec![0.4; 48_000], 48_000);

    let mut comp = Composition::new("c".into(), "Audio layer".into(), 64, 64, 30, 30);
    let mut layer = Layer::new(
        "voice".into(),
        "Voice".into(),
        LayerType::Audio {
            path: wav.to_string_lossy().into_owned(),
            volume: Animatable::new_constant(0.0),
        },
        30,
    );
    layer.in_frame = 0;
    layer.out_frame = 30;
    comp.layers.push(layer);

    let (mix, _) =
        mix_audio_sources_for_frame(&comp, 0, 48_000, 1_600, None, &MasterDspParams::bypass());
    assert!((mix[0] - 0.4).abs() < 0.01, "mix[0] = {}", mix[0]);
    let _ = std::fs::remove_dir_all(&dir);
}

#[test]
fn video_audio_follows_positive_playback_speed() {
    let dir = std::env::temp_dir().join(format!("kagari_audio_speed_{}", std::process::id()));
    let _ = std::fs::create_dir_all(&dir);
    let wav = dir.join("speed.wav");
    let samples: Vec<f32> = (0..1_000).map(|index| index as f32 / 1_000.0).collect();
    write_wav(&wav, &samples, 1_000);

    let mut comp = Composition::new("c".into(), "Audio speed".into(), 64, 64, 10, 10);
    let mut layer = Layer::new(
        "video".into(),
        "Video with audio".into(),
        LayerType::Video {
            source: "video".into(),
            frames_dir: dir.to_string_lossy().into_owned(),
            frame_count: 10,
            audio_wav: Some(wav.to_string_lossy().into_owned()),
            speed: 2.0,
        },
        10,
    );
    layer.in_frame = 0;
    layer.out_frame = 10;
    comp.layers.push(layer);

    let (mix, _) =
        mix_audio_sources_for_frame(&comp, 1, 1_000, 1, None, &MasterDspParams::bypass());
    assert!(
        (mix[0] - 0.2).abs() < 0.02,
        "speed-adjusted sample = {}",
        mix[0]
    );
    let _ = std::fs::remove_dir_all(&dir);
}
