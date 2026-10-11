pub(crate) type RenderKey = (u64, u64, u32, u32, u32, (u32, u32));
#[derive(Clone, PartialEq, Eq, Hash)]
pub(crate) struct TextTextureKey {
    pub(crate) layer_id: String,
    pub(crate) text: String,
    pub(crate) font_size: u32,
    pub(crate) color: [u32; 4],
    pub(crate) font_family: String,
    pub(crate) tracking: u32,
    pub(crate) leading: u32,
    pub(crate) box_width: u32,
    pub(crate) align: usize,
    pub(crate) stroke_color: [u32; 4],
    pub(crate) stroke_width: u32,
    pub(crate) faux_bold: bool,
    pub(crate) faux_italic: bool,
    pub(crate) all_caps: bool,
    pub(crate) small_caps: bool,
}
pub(crate) type TextTextureCache = std::collections::HashMap<
    TextTextureKey,
    (wgpu::Texture, std::sync::Arc<wgpu::BindGroup>, u32, u32),
>;
pub(crate) type VideoFrameKey = (String, String, u32);
struct VideoFrameEntry {
    _texture: std::sync::Arc<wgpu::Texture>,
    bind_group: std::sync::Arc<wgpu::BindGroup>,
    width: u32,
    height: u32,
    byte_size: usize,
    last_used: u64,
}

pub(crate) struct VideoFrameCache {
    entries: std::collections::HashMap<VideoFrameKey, VideoFrameEntry>,
    current_bytes: usize,
    max_bytes: usize,
    clock: u64,
}

impl VideoFrameCache {
    pub(crate) const DEFAULT_MAX_BYTES: usize = 512 * 1024 * 1024;

    pub(crate) fn new() -> Self {
        Self {
            entries: std::collections::HashMap::new(),
            current_bytes: 0,
            max_bytes: Self::DEFAULT_MAX_BYTES,
            clock: 0,
        }
    }

    pub(crate) fn get(
        &mut self,
        key: &VideoFrameKey,
    ) -> Option<(std::sync::Arc<wgpu::BindGroup>, u32, u32)> {
        self.clock = self.clock.wrapping_add(1);
        let entry = self.entries.get_mut(key)?;
        entry.last_used = self.clock;
        Some((entry.bind_group.clone(), entry.width, entry.height))
    }

    pub(crate) fn insert(
        &mut self,
        key: VideoFrameKey,
        texture: std::sync::Arc<wgpu::Texture>,
        bind_group: std::sync::Arc<wgpu::BindGroup>,
        width: u32,
        height: u32,
    ) {
        let Some(byte_size) = (width as usize)
            .checked_mul(height as usize)
            .and_then(|pixels| pixels.checked_mul(8))
        else {
            return;
        };
        if byte_size > self.max_bytes {
            return;
        }
        if let Some(old) = self.entries.remove(&key) {
            self.current_bytes = self.current_bytes.saturating_sub(old.byte_size);
        }
        self.clock = self.clock.wrapping_add(1);
        self.current_bytes = self.current_bytes.saturating_add(byte_size);
        self.entries.insert(
            key,
            VideoFrameEntry {
                _texture: texture,
                bind_group,
                width,
                height,
                byte_size,
                last_used: self.clock,
            },
        );
        while self.current_bytes > self.max_bytes {
            let Some(oldest) = self
                .entries
                .iter()
                .min_by_key(|(_, entry)| entry.last_used)
                .map(|(key, _)| key.clone())
            else {
                break;
            };
            if let Some(removed) = self.entries.remove(&oldest) {
                self.current_bytes = self.current_bytes.saturating_sub(removed.byte_size);
            }
        }
    }

    pub(crate) fn clear(&mut self) {
        self.entries.clear();
        self.current_bytes = 0;
    }
}
