#[cfg(test)]
#[allow(clippy::module_inception)]
mod tests {
    use crate::core::mlt_export::*;
    use crate::core::timeline::{Composition, Layer, LayerType};

    #[test]
    fn test_mlt_export_structure() {
        let mut comp = Composition::new("c1".into(), "MLTTest".into(), 640, 360, 30, 90);
        let bg = Layer::new(
            "bg".into(),
            "BG".into(),
            LayerType::Solid { color: [0.2; 4] },
            30,
        );
        comp.layers.push(bg);
        let img = Layer::new(
            "img".into(),
            "Logo <v1>".into(),
            LayerType::Image {
                path: "/tmp/art&work.png".into(),
            },
            30,
        );
        comp.layers.push(img);

        let xml = MltExporter::export_to_xml(&comp).unwrap();
        assert!(xml.starts_with("<?xml"));
        assert!(xml.contains("<mlt "));
        assert!(xml.contains("width=\"640\" height=\"360\""), "profile dims");
        assert!(xml.contains("frame_rate_num=\"30\""));
        // XML escaping
        assert!(
            xml.contains("/tmp/art&amp;work.png"),
            "special chars escaped"
        );
        assert!(xml.contains("Logo &lt;v1&gt;"), "names escaped");
        // Producer and entry present
        assert!(xml.contains("id=\"producer1\""));
        assert!(xml.contains("<entry producer=\"producer1\""));
        assert!(xml.contains("<track producer=\"playlist0\"/>"));
    }

    #[test]
    fn test_mlt_solid_exports_actual_color_not_label() {
        let mut comp = Composition::new("c".into(), "C".into(), 320, 180, 30, 30);
        comp.layers.push(Layer::new(
            "s".into(),
            "Green".into(),
            LayerType::Solid {
                color: [0.0, 1.0, 0.0, 1.0],
            },
            30,
        ));
        let xml = MltExporter::export_to_xml(&comp).unwrap();
        assert!(
            xml.contains("color:00ff00"),
            "solid must export its own color, got:\n{}",
            xml
        );
    }

    #[test]
    fn test_mlt_video_layer_resource_is_source_file() {
        let mut comp = Composition::new("c".into(), "V".into(), 320, 180, 30, 30);
        let vid = Layer::new(
            "v".into(),
            "Clip".into(),
            crate::core::timeline::LayerType::Video {
                source: "/media/clip.mp4".into(),
                frames_dir: "/tmp/frames".into(),
                frame_count: 100,
                audio_wav: None,
                speed: 1.0,
            },
            30,
        );
        comp.layers.push(vid);
        let xml = MltExporter::export_to_xml(&comp).unwrap();
        assert!(
            xml.contains("<property name=\"resource\">/media/clip.mp4</property>"),
            "video source should be the original file for NLE round-trip"
        );
    }

    const SAMPLE: &str = r#"<?xml version="1.0"?>
<mlt version="7.0.0">
  <profile width="640" height="360" frame_rate_num="30"/>
  <producer id="black" in="00:00:00.000" out="00:99:00.000">
    <property name="resource">0</property>
    <property name="mlt_service">color</property>
  </producer>
  <producer id="producer0" in="00:00:00.000" out="00:00:03.000">
    <property name="resource">/media/clip.mp4</property>
    <property name="mlt_service">avformat</property>
    <property name="name">My &amp; Clip</property>
  </producer>
  <playlist id="playlist0">
    <entry producer="black" in="00:00:00.000" out="00:00:00.000"/>
    <entry producer="producer0" in="00:00:00.000" out="00:00:03.000">
      <property name="name">Renamed</property>
    </entry>
  </playlist>
  <tractor id="tractor0"><track producer="playlist0"/></tractor>
</mlt>"#;

    #[test]
    fn test_import_recovers_clips_in_order() {
        let clips = import_from_mlt(SAMPLE, 30).expect("must parse");
        assert_eq!(clips.len(), 1, "black producer skipped");
        let clip = &clips[0];
        assert_eq!(clip.resource, "/media/clip.mp4");
        assert_eq!(clip.name, "Renamed");
        // MLT's inclusive frame 90 becomes Kagari's exclusive frame 91.
        assert_eq!(clip.out_frame, 91);
        assert!(!clip.is_audio);
    }

    #[test]
    fn test_import_rejects_non_mlt() {
        assert!(import_from_mlt("<html></html>", 30).is_err());
        assert!(import_from_mlt("", 30).is_err());
    }

    #[test]
    fn test_roundtrip_export_then_import() {
        let mut comp = Composition::new("c".into(), "RT".into(), 320, 180, 30, 60);
        let vid = Layer::new(
            "v".into(),
            "Clip".into(),
            LayerType::Video {
                source: "/media/clip.mp4".into(),
                frames_dir: "/tmp/f".into(),
                frame_count: 100,
                audio_wav: None,
                speed: 1.0,
            },
            30,
        );
        comp.layers.push(vid);
        let xml = MltExporter::export_to_xml(&comp).unwrap();
        let clips = import_from_mlt(&xml, 30).expect("roundtrip must parse");
        assert_eq!(clips.len(), 1);
        assert_eq!(clips[0].resource, "/media/clip.mp4");
        assert_eq!(clips[0].name, "Clip");
    }

    #[test]
    fn test_openshot_export_json_structure() {
        let mut comp = Composition::new("c".into(), "OS".into(), 1920, 1080, 30, 60);
        let vid = Layer::new(
            "v".into(),
            "Shot".into(),
            LayerType::Video {
                source: "/path/video.mp4".into(),
                frames_dir: "/tmp/f".into(),
                frame_count: 60,
                audio_wav: None,
                speed: 1.0,
            },
            30,
        );
        comp.layers.push(vid);
        let json = OpenShotExporter::export_to_json(&comp).expect("OpenShot export succeeds");
        assert!(json.contains("\"width\": 1920"));
        assert!(json.contains("/path/video.mp4"));
        assert!(json.contains("\"media_type\": \"video\""));
    }
}
