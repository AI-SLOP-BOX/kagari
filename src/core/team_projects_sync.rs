//! Experimental packet model for a future team editing system.

#![allow(dead_code)]

use serde::{Deserialize, Serialize};

/// Granular operations representing atomic project mutations for collaborative editing.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ProjectChangeOp {
    AddLayer {
        layer_id: String,
        name: String,
        layer_type: String,
    },
    RemoveLayer {
        layer_id: String,
    },
    UpdatePosition {
        layer_id: String,
        frame: u32,
        position: [f32; 2],
        version: u64,
    },
    UpdateOpacity {
        layer_id: String,
        frame: u32,
        opacity: f32,
        version: u64,
    },
    AddMarker {
        frame: u32,
        comment: String,
        user_id: String,
    },
}

/// A versioned change packet sent across network clients.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SyncPacket {
    pub client_id: String,
    pub timestamp_ms: u64,
    pub sequence_num: u64,
    pub operations: Vec<ProjectChangeOp>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TeamSyncError {
    NotImplemented,
    SequenceExhausted,
}

impl std::fmt::Display for TeamSyncError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NotImplemented => f.write_str("team project synchronization is not implemented"),
            Self::SequenceExhausted => f.write_str("team sync sequence number is exhausted"),
        }
    }
}

impl std::error::Error for TeamSyncError {}

/// Packet construction only; applying remote operations is not implemented.
#[derive(Debug, Default)]
pub struct TeamSyncEngine {
    pub client_id: String,
    pub current_sequence: u64,
}

impl TeamSyncEngine {
    pub fn new(client_id: &str) -> Self {
        Self {
            client_id: client_id.to_string(),
            current_sequence: 0,
        }
    }

    /// Creates a packet without retaining it in an unbounded in-memory history.
    pub fn create_packet(
        &mut self,
        ops: Vec<ProjectChangeOp>,
    ) -> Result<SyncPacket, TeamSyncError> {
        self.current_sequence = self
            .current_sequence
            .checked_add(1)
            .ok_or(TeamSyncError::SequenceExhausted)?;
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        Ok(SyncPacket {
            client_id: self.client_id.clone(),
            timestamp_ms: now,
            sequence_num: self.current_sequence,
            operations: ops,
        })
    }

    /// Remote packets are rejected until operations can be applied atomically to a project.
    pub fn merge_remote_packet(&mut self, _packet: SyncPacket) -> Result<(), TeamSyncError> {
        Err(TeamSyncError::NotImplemented)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_team_sync_packet_creation_does_not_claim_remote_merge() {
        let mut alice = TeamSyncEngine::new("Alice");
        let mut bob = TeamSyncEngine::new("Bob");

        let p1 = alice
            .create_packet(vec![ProjectChangeOp::AddLayer {
                layer_id: "L1".into(),
                name: "Text Layer".into(),
                layer_type: "Text".into(),
            }])
            .unwrap();

        assert_eq!(
            bob.merge_remote_packet(p1),
            Err(TeamSyncError::NotImplemented)
        );
    }
}
