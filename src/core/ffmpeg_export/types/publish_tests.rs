use super::*;
use std::io::{Read, Write};

struct TempDir(std::path::PathBuf);

impl TempDir {
    fn new() -> Self {
        let sequence = TEMP_OUTPUT_SEQUENCE.fetch_add(1, Ordering::Relaxed);
        let path = std::env::temp_dir().join(format!(
            "kagari-publish-test-{}-{sequence}",
            std::process::id()
        ));
        std::fs::create_dir(&path).expect("create isolated publish test directory");
        Self(path)
    }
}

impl Drop for TempDir {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

#[test]
fn fallback_keeps_destination_hidden_until_copy_finishes() {
    let directory = TempDir::new();
    let source = directory.0.join("source.mov");
    let destination = directory.0.join("output.mov");
    let payload = b"complete render payload";
    std::fs::write(&source, payload).expect("write source output");

    let copied = stage_copy_and_publish(
        &source,
        &destination,
        |input, output| {
            assert!(!destination.exists());
            let mut contents = Vec::new();
            input.read_to_end(&mut contents)?;
            output.write_all(&contents[..4])?;
            assert!(!destination.exists());
            output.write_all(&contents[4..])?;
            assert!(!destination.exists());
            Ok(contents.len() as u64)
        },
        rename_no_replace,
    )
    .expect("publish completed staged copy");

    assert_eq!(
        copied,
        path_file_identity(&destination).expect("destination identity")
    );
    assert_eq!(
        std::fs::read(&destination).expect("read published output"),
        payload
    );
    assert_eq!(
        std::fs::read(&source).expect("read original output"),
        payload
    );
}

#[test]
fn no_replace_conflict_preserves_existing_output_and_cleans_stage() {
    let directory = TempDir::new();
    let source = directory.0.join("source.mov");
    let destination = directory.0.join("output.mov");
    std::fs::write(&source, b"new render").expect("write source output");
    std::fs::write(&destination, b"existing file").expect("write existing output");

    let result = stage_copy_and_publish(&source, &destination, std::io::copy, rename_no_replace);

    assert_eq!(
        result
            .expect_err("existing destination must not be replaced")
            .kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        std::fs::read(&destination).expect("read existing output"),
        b"existing file"
    );
    assert_eq!(
        std::fs::read_dir(&directory.0)
            .expect("list test files")
            .count(),
        2
    );
}

#[test]
fn existing_destination_skips_hard_link_fallback() {
    let directory = TempDir::new();
    let source = directory.0.join("source.mov");
    let destination = directory.0.join("output.mov");
    std::fs::write(&source, b"new render").expect("write source output");
    std::fs::write(&destination, b"existing file").expect("write existing output");

    let result = publish_file_no_replace(&source, &destination);

    assert_eq!(
        result
            .expect_err("existing destination must not be replaced")
            .kind(),
        std::io::ErrorKind::AlreadyExists
    );
    assert_eq!(
        std::fs::read(&destination).expect("read existing output"),
        b"existing file"
    );
    assert_eq!(
        std::fs::read_dir(&directory.0)
            .expect("list test files")
            .count(),
        2
    );
}

#[test]
fn overwrite_replaces_the_confirmed_destination() {
    let directory = TempDir::new();
    let temporary = directory.0.join("completed.mov");
    let destination = directory.0.join("existing.mov");
    std::fs::write(&temporary, b"new render").expect("write completed render");
    std::fs::write(&destination, b"confirmed old render").expect("write destination");
    let expected = snapshot_overwrite_identity(&destination, true)
        .expect("snapshot identity")
        .expect("existing destination identity");

    publish_export_output(&temporary, &destination, Some(expected))
        .expect("replace confirmed destination");

    assert_eq!(
        std::fs::read(&destination).expect("read published render"),
        b"new render"
    );
    assert!(!temporary.exists(), "published temporary file is consumed");
}

#[test]
fn overwrite_preserves_a_destination_replaced_during_render() {
    let directory = TempDir::new();
    let temporary = directory.0.join("completed.mov");
    let destination = directory.0.join("existing.mov");
    let replacement = directory.0.join("external.mov");
    std::fs::write(&temporary, b"new render").expect("write completed render");
    std::fs::write(&destination, b"confirmed old render").expect("write destination");
    let expected = snapshot_overwrite_identity(&destination, true)
        .expect("snapshot identity")
        .expect("existing destination identity");
    std::fs::write(&replacement, b"external replacement").expect("write concurrent output");
    replace_existing_file(&replacement, &destination).expect("simulate concurrent replacement");

    let error = publish_export_output(&temporary, &destination, Some(expected))
        .expect_err("changed destination must not be overwritten");

    assert!(error.to_string().contains("destination changed"));
    assert_eq!(
        std::fs::read(&destination).expect("read concurrent output"),
        b"external replacement"
    );
    assert_eq!(
        std::fs::read(&temporary).expect("read completed render"),
        b"new render"
    );
}
