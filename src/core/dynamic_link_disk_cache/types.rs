//! Data structures and messages for dynamic link & persistent disk cache.

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct DiskCacheMetadata {
    pub cache_key: u64,
    pub comp_id: String,
    pub frame: u32,
    pub width: u32,
    pub height: u32,
    pub size_bytes: usize,
    pub last_access_epoch: u64,
}

pub struct DiskCacheWrite {
    pub cache_key: u64,
    pub comp_id: String,
    pub frame: u32,
    pub width: u32,
    pub height: u32,
    pub data: Vec<u8>,
    pub current_epoch: u64,
}

/// Dynamic Link live bidirectional message protocol.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub enum DynamicLinkMessage {
    Ping,
    Pong,
    SyncCompositionSettings {
        comp_id: String,
        width: u32,
        height: u32,
        fps: u32,
        duration_frames: u32,
    },
    RequestRenderFrame {
        comp_id: String,
        frame: u32,
    },
    FrameRenderResult {
        comp_id: String,
        frame: u32,
        width: u32,
        height: u32,
        rgba_png: Vec<u8>,
    },
    InvalidateCache {
        comp_id: String,
    },
}
