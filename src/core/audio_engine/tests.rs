use super::cache::{AudioCacheKey, DecodedAudioCache};
use super::*;
use std::sync::Arc;

#[test]
fn test_mix_audio_for_frame_empty_comp() {
    let comp = Composition::new("c1".into(), "Comp 1".into(), 1920, 1080, 30, 300);
    let (buf, meter) = mix_audio_for_frame(&comp, 0, 44100, 512, &MasterDspParams::default());
    assert_eq!(buf.len(), 1024);
    assert!(
        meter.peak_db_left < -100.0,
        "silent comp should have very low peak"
    );
}

#[test]
fn decoded_audio_cache_enforces_byte_budget_and_lru_order() {
    let mut cache = DecodedAudioCache::new(16);
    let make_buffer = |value| {
        Arc::new(AudioBuffer {
            samples: vec![value, value],
            sample_rate: 48_000,
            channels: 2,
        })
    };
    let key = |path: &str| AudioCacheKey {
        path: path.to_owned(),
        sample_rate: 48_000,
        file_len: 10,
        modified_ns: Some(1),
    };

    cache.insert(key("a.wav"), make_buffer(0.1));
    cache.insert(key("b.wav"), make_buffer(0.2));
    assert_eq!(cache.bytes, 16);
    assert!(cache.get(&key("a.wav")).is_some());

    cache.insert(key("c.wav"), make_buffer(0.3));
    assert!(cache.get(&key("a.wav")).is_some());
    assert!(cache.get(&key("b.wav")).is_none());
    assert!(cache.get(&key("c.wav")).is_some());
    assert!(cache.bytes <= cache.max_bytes);
}

#[test]
fn decoded_audio_cache_key_includes_requested_sample_rate() {
    let mut cache = DecodedAudioCache::new(32);
    let buffer = Arc::new(AudioBuffer {
        samples: vec![0.25, 0.5],
        sample_rate: 44_100,
        channels: 2,
    });
    cache.insert(
        AudioCacheKey {
            path: "same.wav".into(),
            sample_rate: 44_100,
            file_len: 12,
            modified_ns: Some(7),
        },
        buffer.clone(),
    );
    assert!(cache
        .get(&AudioCacheKey {
            path: "same.wav".into(),
            sample_rate: 48_000,
            file_len: 12,
            modified_ns: Some(7),
        })
        .is_none());
}

#[test]
fn decoded_audio_cache_key_includes_source_file_revision() {
    let mut cache = DecodedAudioCache::new(32);
    let source_key = AudioCacheKey {
        path: "same.wav".into(),
        sample_rate: 48_000,
        file_len: 100,
        modified_ns: Some(10),
    };
    cache.insert(
        source_key,
        Arc::new(AudioBuffer {
            samples: vec![0.25, 0.5],
            sample_rate: 48_000,
            channels: 2,
        }),
    );
    assert!(cache
        .get(&AudioCacheKey {
            path: "same.wav".into(),
            sample_rate: 48_000,
            file_len: 101,
            modified_ns: Some(11),
        })
        .is_none());
}

#[test]
fn resample_rejects_output_that_exceeds_memory_limit() {
    let source = AudioBuffer {
        samples: vec![0.25],
        sample_rate: 1,
        channels: 1,
    };

    assert!(source.resample(u32::MAX).is_err());
}
