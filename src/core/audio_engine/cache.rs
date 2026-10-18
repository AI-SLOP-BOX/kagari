use super::*;
use std::collections::{HashMap, VecDeque};
use std::sync::{Arc, Mutex, OnceLock};

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub(super) struct AudioCacheKey {
    pub(super) path: String,
    pub(super) sample_rate: u32,
    pub(super) file_len: u64,
    pub(super) modified_ns: Option<u128>,
}

pub(super) struct DecodedAudioCache {
    entries: HashMap<AudioCacheKey, Arc<AudioBuffer>>,
    least_to_most_recent: VecDeque<AudioCacheKey>,
    pub(super) bytes: usize,
    pub(super) max_bytes: usize,
}

impl DecodedAudioCache {
    pub(super) fn new(max_bytes: usize) -> Self {
        Self {
            entries: HashMap::new(),
            least_to_most_recent: VecDeque::new(),
            bytes: 0,
            max_bytes,
        }
    }

    pub(super) fn get(&mut self, key: &AudioCacheKey) -> Option<Arc<AudioBuffer>> {
        let value = self.entries.get(key)?.clone();
        if let Some(index) = self
            .least_to_most_recent
            .iter()
            .position(|entry| entry == key)
        {
            self.least_to_most_recent.remove(index);
        }
        self.least_to_most_recent.push_back(key.clone());
        Some(value)
    }

    pub(super) fn insert(&mut self, key: AudioCacheKey, value: Arc<AudioBuffer>) {
        let bytes = value
            .samples
            .len()
            .saturating_mul(std::mem::size_of::<f32>());
        if bytes > self.max_bytes {
            return;
        }
        self.remove(&key);
        while self.entries.len() >= MAX_AUDIO_CACHE_ENTRIES
            || self.bytes.saturating_add(bytes) > self.max_bytes
        {
            let Some(oldest) = self.least_to_most_recent.pop_front() else {
                break;
            };
            self.remove(&oldest);
        }
        self.bytes = self.bytes.saturating_add(bytes);
        self.least_to_most_recent.push_back(key.clone());
        self.entries.insert(key, value);
    }

    fn remove(&mut self, key: &AudioCacheKey) {
        if let Some(value) = self.entries.remove(key) {
            self.bytes = self.bytes.saturating_sub(
                value
                    .samples
                    .len()
                    .saturating_mul(std::mem::size_of::<f32>()),
            );
        }
        self.least_to_most_recent.retain(|entry| entry != key);
    }
}

fn audio_cache() -> &'static Mutex<DecodedAudioCache> {
    static CACHE: OnceLock<Mutex<DecodedAudioCache>> = OnceLock::new();
    CACHE.get_or_init(|| Mutex::new(DecodedAudioCache::new(MAX_AUDIO_CACHE_BYTES)))
}

pub(super) fn load_cached_audio(path: &str, sample_rate: u32) -> Result<Arc<AudioBuffer>, String> {
    validate_audio_path(path)?;
    let metadata = std::fs::metadata(path)
        .map_err(|error| format!("could not inspect audio source: {error}"))?;
    let modified_ns = metadata
        .modified()
        .ok()
        .and_then(|modified| modified.duration_since(std::time::UNIX_EPOCH).ok())
        .map(|duration| duration.as_nanos());
    let key = AudioCacheKey {
        path: path.to_owned(),
        sample_rate,
        file_len: metadata.len(),
        modified_ns,
    };
    if let Some(buffer) = audio_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .get(&key)
    {
        return Ok(buffer);
    }
    let buffer = AudioBuffer::load_audio(std::path::Path::new(path))
        .map_err(|error| format!("could not decode audio source: {error}"))?
        .resample(sample_rate)?;
    let buffer = Arc::new(buffer);
    audio_cache()
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner())
        .insert(key, buffer.clone());
    Ok(buffer)
}
