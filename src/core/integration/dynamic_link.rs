use super::*;

// --- 2. Dynamic Link Server (Real-time synchronization protocol) ---

pub const DYNAMIC_LINK_FRAME_QUEUE_CAPACITY: usize = 1;
pub const DYNAMIC_LINK_CONNECTION_QUEUE_CAPACITY: usize = 8;

#[derive(Clone)]
pub struct LatestFrameSender {
    latest: std::sync::Arc<std::sync::atomic::AtomicU32>,
    wake: std::sync::mpsc::SyncSender<()>,
}

pub struct LatestFrameReceiver {
    latest: std::sync::Arc<std::sync::atomic::AtomicU32>,
    wake: std::sync::mpsc::Receiver<()>,
}

pub fn latest_frame_channel() -> (LatestFrameSender, LatestFrameReceiver) {
    let latest = std::sync::Arc::new(std::sync::atomic::AtomicU32::new(0));
    let (wake, receiver) = std::sync::mpsc::sync_channel(DYNAMIC_LINK_FRAME_QUEUE_CAPACITY);
    (
        LatestFrameSender {
            latest: latest.clone(),
            wake,
        },
        LatestFrameReceiver {
            latest,
            wake: receiver,
        },
    )
}

impl LatestFrameSender {
    pub fn publish(&self, frame: u32) {
        self.latest
            .store(frame, std::sync::atomic::Ordering::Release);
        let _ = self.wake.try_send(());
    }
}

impl LatestFrameReceiver {
    pub fn try_recv(&self) -> Result<u32, std::sync::mpsc::TryRecvError> {
        self.wake.try_recv()?;
        Ok(self.latest.load(std::sync::atomic::Ordering::Acquire))
    }

    pub fn recv_timeout(
        &self,
        timeout: std::time::Duration,
    ) -> Result<u32, std::sync::mpsc::RecvTimeoutError> {
        self.wake.recv_timeout(timeout)?;
        Ok(self.latest.load(std::sync::atomic::Ordering::Acquire))
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum DynamicLinkMessage {
    /// Synchronize playhead position
    SyncPlayhead {
        frame: u32,
    },
    /// Notify or request asset addition
    AddAsset {
        name: String,
        path: String,
    },
    /// Trigger frame render request from external NLE
    TriggerRender {
        comp_id: String,
        frame: u32,
    },
    /// Connection handshake / status
    Handshake {
        client_name: String,
        protocol_version: String,
    },
    Ping,
    Pong,
    /// Confirmation of asset addition
    AssetAdded {
        name: String,
        path: String,
    },
    /// Acknowledgment or result of render trigger
    RenderTriggered {
        comp_id: String,
        frame: u32,
    },
    Error {
        message: String,
    },
}

pub struct DynamicLinkServer {
    pub is_connected: bool,
    pub connected_app: Option<String>, // e.g. "Kdenlive" or "Shotcut"
    pub sync_port: u16,
}

impl Default for DynamicLinkServer {
    fn default() -> Self {
        Self {
            is_connected: false,
            connected_app: None,
            sync_port: 8080,
        }
    }
}

fn dynamic_link_version_compatible(version: &str) -> bool {
    let mut parts = version.split('.');
    let Some(major) = parts.next().and_then(|part| part.parse::<u32>().ok()) else {
        return false;
    };
    parts
        .next()
        .and_then(|part| part.parse::<u32>().ok())
        .is_some()
        && parts
            .next()
            .and_then(|part| part.parse::<u32>().ok())
            .is_some()
        && parts.next().is_none()
        && major == 1
}

impl DynamicLinkServer {
    pub fn new(port: u16) -> Self {
        Self {
            is_connected: false,
            connected_app: None,
            sync_port: port,
        }
    }

    /// Process an incoming message from the WebSocket link and return a response message if necessary
    pub fn handle_message(&mut self, raw_msg: &str) -> Option<DynamicLinkMessage> {
        let msg: Result<DynamicLinkMessage, serde_json::Error> = serde_json::from_str(raw_msg);

        match msg {
            Ok(DynamicLinkMessage::Handshake {
                client_name,
                protocol_version,
            }) => {
                if !dynamic_link_version_compatible(&protocol_version) {
                    return Some(DynamicLinkMessage::Error {
                        message: format!(
                            "Incompatible Dynamic Link protocol version '{protocol_version}'; supported major version is 1"
                        ),
                    });
                }
                self.is_connected = true;
                self.connected_app = Some(client_name.clone());
                Some(DynamicLinkMessage::Handshake {
                    client_name: "Kagari VFX".to_string(),
                    protocol_version: "1.0.0".to_string(),
                })
            }
            Ok(DynamicLinkMessage::Ping) => Some(DynamicLinkMessage::Pong),
            Ok(DynamicLinkMessage::SyncPlayhead { .. }) => {
                // Return None, application layer will handle playhead update
                None
            }
            Ok(DynamicLinkMessage::AddAsset { name, path }) => {
                if std::path::Path::new(&path).exists() {
                    Some(DynamicLinkMessage::AssetAdded { name, path })
                } else {
                    Some(DynamicLinkMessage::Error {
                        message: format!("Asset file does not exist: {path}"),
                    })
                }
            }
            Ok(DynamicLinkMessage::TriggerRender { comp_id, frame }) => {
                Some(DynamicLinkMessage::RenderTriggered { comp_id, frame })
            }
            Err(error) => Some(DynamicLinkMessage::Error {
                message: format!("Invalid Dynamic Link message: {error}"),
            }),
            Ok(
                DynamicLinkMessage::Pong
                | DynamicLinkMessage::AssetAdded { .. }
                | DynamicLinkMessage::RenderTriggered { .. }
                | DynamicLinkMessage::Error { .. },
            ) => None,
        }
    }
}

/// Spawns a background thread running a TCP synchronization server.
pub fn start_sync_server(
    port: u16,
    frame_tx: LatestFrameSender,
    connection_tx: std::sync::mpsc::SyncSender<Option<String>>,
) -> Result<u16, std::io::Error> {
    use std::io::Write;
    use std::net::TcpListener;
    use std::sync::atomic::{AtomicBool, Ordering};
    use std::sync::Arc;
    use std::thread;

    let listener = TcpListener::bind(format!("127.0.0.1:{}", port))?;
    let bound_port = listener.local_addr()?.port();
    log::info!("Dynamic Link Sync Server listening on port {}", bound_port);
    let client_active = Arc::new(AtomicBool::new(false));

    thread::spawn(move || {
        for stream in listener.incoming() {
            let stream = match stream {
                Ok(s) => s,
                Err(_) => continue,
            };
            if client_active
                .compare_exchange(false, true, Ordering::AcqRel, Ordering::Acquire)
                .is_err()
            {
                let response = DynamicLinkMessage::Error {
                    message: "Only one Dynamic Link client can be connected at a time".into(),
                };
                if let Ok(json) = serde_json::to_vec(&response) {
                    let mut stream = stream;
                    let _ = stream.write_all(&json);
                    let _ = stream.write_all(b"\n");
                    let _ = stream.flush();
                }
                continue;
            }
            let frame_tx = frame_tx.clone();
            let connection_tx = connection_tx.clone();
            let client_active_for_worker = client_active.clone();
            if let Err(error) = thread::Builder::new()
                .name("kagari-dynamic-link-client".into())
                .spawn(move || {
                    handle_sync_client(stream, frame_tx, connection_tx);
                    client_active_for_worker.store(false, Ordering::Release);
                })
            {
                client_active.store(false, Ordering::Release);
                log::error!("Could not start Dynamic Link client worker: {error}");
            }
        }
    });

    Ok(bound_port)
}

fn handle_sync_client(
    stream: std::net::TcpStream,
    frame_tx: LatestFrameSender,
    connection_tx: std::sync::mpsc::SyncSender<Option<String>>,
) {
    use std::io::{BufRead, BufReader, Read};
    const MAX_MESSAGE_BYTES: usize = 65_536;

    let stream_clone = match stream.try_clone() {
        Ok(stream) => stream,
        Err(error) => {
            log::warn!("Could not monitor Dynamic Link connection: {error}");
            return;
        }
    };
    let _ = stream.set_read_timeout(Some(std::time::Duration::from_secs(10)));
    let mut reader = BufReader::new(stream_clone);
    let mut writer = stream;
    let mut active_client: Option<String> = None;
    let mut line = Vec::with_capacity(MAX_MESSAGE_BYTES.min(4096));
    loop {
        line.clear();
        let read_result = reader
            .by_ref()
            .take((MAX_MESSAGE_BYTES + 1) as u64)
            .read_until(b'\n', &mut line);
        match read_result {
            Ok(0) => break,
            Ok(_) => {
                if line.len() > MAX_MESSAGE_BYTES {
                    log::warn!("Dynamic Link message exceeded the line limit; closing client");
                    break;
                }
                let trimmed = match std::str::from_utf8(&line) {
                    Ok(value) => value.trim(),
                    Err(_) => {
                        log::warn!("Dynamic Link message was not valid UTF-8");
                        continue;
                    }
                };
                if trimmed.is_empty() {
                    continue;
                }
                let Ok(message) = serde_json::from_str::<DynamicLinkMessage>(trimmed) else {
                    log::warn!("Ignoring malformed Dynamic Link message");
                    continue;
                };
                match message {
                    DynamicLinkMessage::Handshake {
                        client_name,
                        protocol_version,
                    } => {
                        if !dynamic_link_version_compatible(&protocol_version) {
                            write_dynamic_link_message(
                                &mut writer,
                                &DynamicLinkMessage::Error {
                                    message: format!(
                                        "Incompatible Dynamic Link protocol version '{protocol_version}'; supported major version is 1"
                                    ),
                                },
                            );
                            break;
                        }
                        if active_client.as_deref() != Some(client_name.as_str()) {
                            active_client = Some(client_name.clone());
                            if connection_tx.try_send(Some(client_name)).is_err() {
                                log::warn!("Dynamic Link connection notification queue is full");
                            }
                        }
                        write_dynamic_link_message(
                            &mut writer,
                            &DynamicLinkMessage::Handshake {
                                client_name: "Kagari VFX".into(),
                                protocol_version: "1.0.0".into(),
                            },
                        );
                    }
                    DynamicLinkMessage::Ping => {
                        write_dynamic_link_message(&mut writer, &DynamicLinkMessage::Pong);
                    }
                    DynamicLinkMessage::SyncPlayhead { frame } => frame_tx.publish(frame),
                    DynamicLinkMessage::AddAsset { name, path } => {
                        if std::path::Path::new(&path).exists() {
                            write_dynamic_link_message(
                                &mut writer,
                                &DynamicLinkMessage::AssetAdded { name, path },
                            );
                        } else {
                            write_dynamic_link_message(
                                &mut writer,
                                &DynamicLinkMessage::Error {
                                    message: format!("Asset file does not exist: {path}"),
                                },
                            );
                        }
                    }
                    DynamicLinkMessage::TriggerRender { comp_id, frame } => {
                        write_dynamic_link_message(
                            &mut writer,
                            &DynamicLinkMessage::RenderTriggered { comp_id, frame },
                        );
                    }
                    DynamicLinkMessage::Pong
                    | DynamicLinkMessage::AssetAdded { .. }
                    | DynamicLinkMessage::RenderTriggered { .. }
                    | DynamicLinkMessage::Error { .. } => {}
                }
            }
            Err(error)
                if matches!(
                    error.kind(),
                    std::io::ErrorKind::TimedOut | std::io::ErrorKind::WouldBlock
                ) =>
            {
                break;
            }
            Err(_) => break,
        }
    }
    if active_client.is_some() && connection_tx.try_send(None).is_err() {
        log::warn!("Dynamic Link disconnect notification queue is full");
    }
}

fn write_dynamic_link_message(writer: &mut std::net::TcpStream, message: &DynamicLinkMessage) {
    use std::io::Write;
    if let Ok(json) = serde_json::to_vec(message) {
        let _ = writer.write_all(&json);
        let _ = writer.write_all(b"\n");
        let _ = writer.flush();
    }
}
