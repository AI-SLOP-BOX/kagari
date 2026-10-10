use crate::core::timeline::Composition;

#[derive(Debug, Clone)]
pub struct AepHeader {
    pub magic: [u8; 4],
    pub version: u32,
    pub items: Vec<AepItem>,
}

#[derive(Debug, Clone)]
pub struct AepItem {
    pub item_type: AepItemType,
    pub name: String,
    pub start_frame: u32,
    pub duration_frames: u32,
    pub width: u32,
    pub height: u32,
    pub fps: f32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum AepItemType {
    Composition,
    Footage,
    Folder,
    Unknown(u32),
}

pub fn parse_aep(data: &[u8]) -> Result<AepHeader, String> {
    if data.len() < 16 {
        return Err("File too small for AEP header".into());
    }
    Err("After Effects .aep project parsing is not supported; no project data was imported".into())
}

#[cfg(test)]
fn is_printable_ascii(s: &str) -> bool {
    s.bytes().all(|b| (0x20..0x7F).contains(&b))
}

pub fn aep_to_compositions(header: &AepHeader) -> Vec<Composition> {
    header
        .items
        .iter()
        .map(|item| {
            Composition::new(
                item.name.clone(),
                item.name.clone(),
                item.width,
                item.height,
                item.fps as u32,
                item.duration_frames.max(1),
            )
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_aep_too_small() {
        let data = vec![0u8; 4];
        assert!(parse_aep(&data).is_err());
    }

    #[test]
    fn test_parse_aep_empty_items() {
        let mut data = vec![0xCF, 0xFA, 0xED, 0xFE, 0, 0, 0, 0];
        data.extend_from_slice(&[0u8; 64]);
        assert!(parse_aep(&data).unwrap_err().contains("not supported"));
    }

    #[test]
    fn test_aep_item_type_eq() {
        assert_eq!(AepItemType::Composition, AepItemType::Composition);
        assert_ne!(AepItemType::Composition, AepItemType::Footage);
    }

    #[test]
    fn test_is_printable_ascii() {
        assert!(is_printable_ascii("Hello World"));
        assert!(!is_printable_ascii("Hello\x00World"));
    }
}
