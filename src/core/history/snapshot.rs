//! Bounded serialization and project comparison writers for undo history.

use std::io::{self, Write};
use crate::core::timeline::Project;

pub(crate) struct ProjectComparisonWriter<'a> {
    reference: &'a [u8],
    written: usize,
    matches: bool,
}

impl<'a> ProjectComparisonWriter<'a> {
    pub fn new(reference: &'a [u8]) -> Self {
        Self {
            reference,
            written: 0,
            matches: true,
        }
    }

    pub fn is_equal(&self) -> bool {
        self.matches && self.written == self.reference.len()
    }
}

impl Write for ProjectComparisonWriter<'_> {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let end = self.written.saturating_add(bytes.len());
        if self.reference.get(self.written..end) != Some(bytes) {
            self.matches = false;
        }
        self.written = end;
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) struct BoundedVecWriter {
    bytes: Vec<u8>,
    limit: usize,
}

impl BoundedVecWriter {
    pub fn new(limit: usize) -> Self {
        Self {
            bytes: Vec::new(),
            limit,
        }
    }

    pub fn finish(mut self) -> Option<Vec<u8>> {
        self.bytes.shrink_to_fit();
        (self.bytes.capacity() <= self.limit).then_some(self.bytes)
    }
}

impl Write for BoundedVecWriter {
    fn write(&mut self, bytes: &[u8]) -> io::Result<usize> {
        let next_len = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| io::Error::other("history snapshot size overflow"))?;
        if next_len > self.limit {
            return Err(io::Error::other(
                "history snapshot exceeds its memory budget",
            ));
        }
        if next_len > self.bytes.capacity() {
            let grown_capacity = self
                .bytes
                .capacity()
                .saturating_mul(2)
                .max(4096)
                .min(self.limit);
            let target_capacity = next_len.max(grown_capacity).min(self.limit);
            self.bytes
                .try_reserve_exact(target_capacity - self.bytes.len())
                .map_err(|error| io::Error::other(format!("history allocation failed: {error}")))?;
        }
        self.bytes.extend_from_slice(bytes);
        Ok(bytes.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

pub(crate) fn serialize_project_bounded(project: &Project, limit: usize) -> Option<Vec<u8>> {
    let mut writer = BoundedVecWriter::new(limit);
    serde_json::to_writer(&mut writer, project).ok()?;
    writer.finish()
}
