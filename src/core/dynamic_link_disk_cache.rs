//! Bi-directional Dynamic Link Protocol & Intelligent Persistent Disk Cache Engine (AE Parity).
//!
//! Provides enterprise-grade LRU disk caching with hash invalidation and
//! live bidirectional IPC Dynamic Link communication for external NLE hosts (Premiere Pro / DaVinci).

#![allow(dead_code)]

pub mod cache;
pub mod types;

#[cfg(test)]
mod tests;

pub use cache::*;
pub use types::*;
