#[test]
fn test_video_texture_byte_budget_is_sane() {
    let bytes = crate::core::renderer::MAX_VIDEO_FRAME_TEXTURE_CACHE_BYTES as u64;
    assert!(
        bytes <= 512 * 1024 * 1024,
        "budget {} bytes too large",
        bytes
    );
    let frame_bytes = 1920u64 * 1080 * 8;
    assert!(bytes / frame_bytes >= 30, "1080p residency is too small");
}
