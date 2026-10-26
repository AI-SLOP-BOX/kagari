#![allow(dead_code)]
use crate::core::timeline::{Composition, Layer, LayerType};
use serde::{Deserialize, Serialize};

mod dynamic_link;
pub use dynamic_link::*;
mod otio;
pub use otio::*;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_otio_serialization() {
        let mut comp = Composition::new(
            "test_comp".to_string(),
            "test_comp".to_string(),
            1920,
            1080,
            30,
            120,
        );
        let layer = Layer::new(
            "layer_1".to_string(),
            "Solid Layer".to_string(),
            LayerType::Solid {
                color: [1.0, 0.0, 0.0, 1.0],
            },
            120,
        );
        comp.add_layer(layer);

        let otio = OtioTimeline::from_composition(&comp);
        assert_eq!(otio.name, "test_comp");
        assert_eq!(otio.tracks[0].items[0].name, "Solid Layer");

        let json = otio.to_json().unwrap();
        let parsed_otio = OtioTimeline::from_json(&json).unwrap();
        assert_eq!(parsed_otio.name, "test_comp");

        let parsed_comp = parsed_otio.to_composition().unwrap();
        assert_eq!(parsed_comp.layers.len(), 1);
        assert_eq!(parsed_comp.layers[0].name, "Solid Layer");
    }

    #[test]
    fn otio_clip_v2_resolves_only_its_active_media_reference() {
        let mut document = serde_json::json!({
            "OTIO_SCHEMA": "Timeline.1",
            "name": "Clip v2",
            "global_start_time": {
                "OTIO_SCHEMA": "RationalTime.1",
                "value": 0,
                "rate": 24
            },
            "tracks": {
                "OTIO_SCHEMA": "Stack.1",
                "children": [{
                    "OTIO_SCHEMA": "Track.1",
                    "name": "V1",
                    "kind": "Video",
                    "children": [{
                        "OTIO_SCHEMA": "Clip.2",
                        "name": "Shot A",
                        "source_range": {
                            "OTIO_SCHEMA": "TimeRange.1",
                            "start_time": {"OTIO_SCHEMA": "RationalTime.1", "value": 0, "rate": 24},
                            "duration": {"OTIO_SCHEMA": "RationalTime.1", "value": 24, "rate": 24}
                        },
                        "active_media_reference_key": "proxy",
                        "media_references": {
                            "DEFAULT_MEDIA": {"OTIO_SCHEMA": "MissingReference.1"},
                            "proxy": {
                                "OTIO_SCHEMA": "ExternalReference.1",
                                "target_url": "file:///tmp/shot-a.png",
                                "available_range": {
                                    "OTIO_SCHEMA": "TimeRange.1",
                                    "start_time": {"OTIO_SCHEMA": "RationalTime.1", "value": 0, "rate": 24},
                                    "duration": {"OTIO_SCHEMA": "RationalTime.1", "value": 48, "rate": 24}
                                }
                            }
                        }
                    }]
                }]
            }
        });
        document["tracks"]["children"][0]["children"][0]["source_range"] = serde_json::Value::Null;

        let timeline = OtioTimeline::from_json(&document.to_string()).unwrap();
        assert_eq!(
            timeline.tracks[0].items[0].media_reference.as_deref(),
            Some("/tmp/shot-a.png")
        );
        assert_eq!(timeline.tracks[0].items[0].source_range, [0, 48]);
        let composition = timeline.to_composition().unwrap();
        assert!(matches!(
            &composition.layers[0].layer_type,
            LayerType::Image { path } if path == "/tmp/shot-a.png"
        ));

        document["tracks"]["children"][0]["children"][0]["active_media_reference_key"] =
            serde_json::json!("missing");
        let error = OtioTimeline::from_json(&document.to_string()).unwrap_err();
        assert!(error
            .to_string()
            .contains("active media reference 'missing'"));
    }

    #[test]
    fn otio_clip_without_any_time_range_is_rejected() {
        let mut document = serde_json::json!({
            "OTIO_SCHEMA": "Timeline.1",
            "name": "Missing range",
            "tracks": {
                "OTIO_SCHEMA": "Stack.1",
                "children": [{
                    "OTIO_SCHEMA": "Track.1",
                    "name": "V1",
                    "kind": "Video",
                    "children": [{
                        "OTIO_SCHEMA": "Clip.2",
                        "name": "Shot without range",
                        "active_media_reference_key": "DEFAULT_MEDIA",
                        "media_references": {
                            "DEFAULT_MEDIA": {
                                "OTIO_SCHEMA": "ExternalReference.1",
                                "target_url": "file:///tmp/shot.png"
                            }
                        }
                    }]
                }]
            }
        });

        let error = OtioTimeline::from_json(&document.to_string()).unwrap_err();
        assert!(error
            .to_string()
            .contains("OTIO clip 'Shot without range' has no source start time"));

        document["tracks"]["children"][0]["children"][0]["OTIO_SCHEMA"] =
            serde_json::json!("Gap.1");
        let error = OtioTimeline::from_json(&document.to_string()).unwrap_err();
        assert!(error
            .to_string()
            .contains("OTIO gap on track 'V1' has no duration"));
    }

    #[test]
    fn test_dynamic_link_handshake() {
        let mut server = DynamicLinkServer::new(9000);
        assert!(!server.is_connected);

        let handshake_req =
            r#"{"type":"handshake","client_name":"Kdenlive","protocol_version":"1.0.0"}"#;
        let resp = server.handle_message(handshake_req);

        assert!(server.is_connected);
        assert_eq!(server.connected_app.unwrap(), "Kdenlive");

        if let Some(DynamicLinkMessage::Handshake { client_name, .. }) = resp {
            assert_eq!(client_name, "Kagari VFX");
        } else {
            panic!("Expected handshake response");
        }
    }

    #[test]
    fn otio_rejects_overflowing_source_range() {
        let timeline = OtioTimeline {
            name: "overflow".into(),
            global_start_time: 0,
            fps: 30.0,
            width: 1920,
            height: 1080,
            duration_frames: 300,
            tracks: vec![OtioTrack {
                name: "V1".into(),
                kind: "video".into(),
                items: vec![OtioItem {
                    name: "bad range".into(),
                    source_range: [u32::MAX, 1],
                    timeline_start_frame: None,
                    native_layer: None,
                    media_reference: None,
                }],
            }],
        };

        let error = timeline.to_composition().unwrap_err();
        assert!(error.contains("bad range"));
        assert!(error.contains("source range"));
    }

    #[test]
    fn otio_rejects_corrupt_kagari_video_reference() {
        let timeline = OtioTimeline {
            name: "corrupt media".into(),
            global_start_time: 0,
            fps: 30.0,
            width: 1920,
            height: 1080,
            duration_frames: 300,
            tracks: vec![OtioTrack {
                name: "V1".into(),
                kind: "video".into(),
                items: vec![OtioItem {
                    name: "broken clip".into(),
                    source_range: [0, 30],
                    timeline_start_frame: None,
                    native_layer: None,
                    media_reference: Some("video-json:{broken".into()),
                }],
            }],
        };

        let error = timeline.to_composition().unwrap_err();
        assert!(error.contains("broken clip"));
        assert!(error.contains("invalid Kagari video reference"));
    }

    #[test]
    fn otio_roundtrip_preserves_audio_and_video_layers() {
        let mut comp = Composition::new(
            "media_comp".to_string(),
            "Media Comp".to_string(),
            1280,
            720,
            24,
            240,
        );
        comp.add_layer(Layer::new(
            "video_1".to_string(),
            "Footage".to_string(),
            LayerType::Video {
                source: "/media/scene.mov".to_string(),
                frames_dir: "/cache/scene".to_string(),
                frame_count: 240,
                audio_wav: Some("/cache/scene.wav".to_string()),
                audio_volume: crate::core::property::Animatable::new_constant(0.0),
                speed: 1.0,
            },
            240,
        ));
        comp.add_layer(Layer::new(
            "audio_1".to_string(),
            "Music".to_string(),
            LayerType::Audio {
                path: "/audio/music.wav".to_string(),
                volume: crate::core::property::Animatable::new_constant(1.0),
            },
            240,
        ));

        let otio = OtioTimeline::from_composition(&comp);
        assert_eq!(otio.tracks.len(), 2);
        assert_eq!(otio.tracks[0].kind, "video");
        assert_eq!(otio.tracks[1].kind, "audio");

        let round_tripped = otio.to_composition().unwrap();
        assert_eq!(round_tripped.width, 1280);
        assert_eq!(round_tripped.height, 720);
        assert_eq!(round_tripped.duration_frames, 240);
        assert!(matches!(
            round_tripped.layers[0].layer_type,
            LayerType::Video { .. }
        ));
        assert!(matches!(
            round_tripped.layers[1].layer_type,
            LayerType::Audio { .. }
        ));
        if let LayerType::Video {
            source,
            frames_dir,
            frame_count,
            audio_wav,
            ..
        } = &round_tripped.layers[0].layer_type
        {
            assert_eq!(source, "/media/scene.mov");
            assert_eq!(frames_dir, "/cache/scene");
            assert_eq!(*frame_count, 240);
            assert_eq!(audio_wav.as_deref(), Some("/cache/scene.wav"));
        } else {
            panic!("video layer did not survive OTIO roundtrip");
        }
        if let LayerType::Audio { path, .. } = &round_tripped.layers[1].layer_type {
            assert_eq!(path, "/audio/music.wav");
        } else {
            panic!("audio layer did not survive OTIO roundtrip");
        }
    }

    #[test]
    fn otio_roundtrip_preserves_model3d_layers() {
        let mut comp = Composition::new(
            "model_comp".to_string(),
            "Model Comp".to_string(),
            1920,
            1080,
            30,
            120,
        );
        comp.add_layer(Layer::new(
            "model_1".to_string(),
            "Hero OBJ".to_string(),
            LayerType::Model3D {
                path: "/assets/hero.obj".to_string(),
            },
            120,
        ));

        let otio = OtioTimeline::from_composition(&comp);
        assert_eq!(
            otio.tracks[0].items[0].media_reference.as_deref(),
            Some("model3d:/assets/hero.obj")
        );

        let json = otio.to_json().unwrap();
        let parsed = OtioTimeline::from_json(&json).unwrap();
        let round_tripped = parsed.to_composition().unwrap();
        assert!(matches!(
            &round_tripped.layers[0].layer_type,
            LayerType::Model3D { path } if path == "/assets/hero.obj"
        ));
    }

    #[test]
    fn test_real_tcp_server_sync() {
        use std::io::{BufRead, BufReader, Write};
        use std::net::TcpStream;
        let (frame_tx, frame_rx) = latest_frame_channel();
        let (conn_tx, conn_rx) =
            std::sync::mpsc::sync_channel(DYNAMIC_LINK_CONNECTION_QUEUE_CAPACITY);

        // Start server on dynamic OS port 0
        let port = match start_sync_server(0, frame_tx, conn_tx) {
            Ok(p) => p,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(e) => panic!("start_sync_server failed: {e:?}"),
        };

        // Connect client
        let mut stream = match TcpStream::connect(format!("127.0.0.1:{}", port)) {
            Ok(s) => s,
            Err(e) if e.kind() == std::io::ErrorKind::PermissionDenied => return,
            Err(e) => panic!("Failed to connect to test TCP server: {e:?}"),
        };
        let mut reader = BufReader::new(stream.try_clone().unwrap());

        // Send Handshake Request
        let handshake_req =
            r#"{"type":"handshake","client_name":"Kdenlive","protocol_version":"1.0.0"}"#;
        stream.write_all(handshake_req.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();

        // Read Handshake Response
        let mut line = String::new();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("Kagari VFX"));

        // Verify connection state channel
        let conn_state = conn_rx.recv().unwrap();
        assert_eq!(conn_state, Some("Kdenlive".to_string()));

        // Repeated handshakes from one connected client must not queue redundant UI events.
        stream.write_all(handshake_req.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();
        line.clear();
        reader.read_line(&mut line).unwrap();
        assert!(line.contains("Kagari VFX"));
        assert!(matches!(
            conn_rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));

        // Send Playhead Sync
        let sync_msg = r#"{"type":"sync_playhead","frame":123}"#;
        stream.write_all(sync_msg.as_bytes()).unwrap();
        stream.write_all(b"\n").unwrap();
        stream.flush().unwrap();

        // Verify frame sync channel
        let frame = frame_rx
            .recv_timeout(std::time::Duration::from_secs(2))
            .unwrap();
        assert_eq!(frame, 123);

        drop(reader);
        drop(stream);
        assert_eq!(
            conn_rx
                .recv_timeout(std::time::Duration::from_secs(2))
                .unwrap(),
            None
        );
    }

    #[test]
    fn latest_frame_mailbox_coalesces_updates_without_stale_queue_growth() {
        let (tx, rx) = latest_frame_channel();
        for frame in 0..10_000 {
            tx.publish(frame);
        }
        assert_eq!(rx.try_recv(), Ok(9_999));
        assert!(matches!(
            rx.try_recv(),
            Err(std::sync::mpsc::TryRecvError::Empty)
        ));
    }
}
