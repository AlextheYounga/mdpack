use mdpack::{PackOptions, UnpackOptions, pack_to_string, unpack_from_str};
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

fn unique_name(prefix: &str) -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("time")
        .as_nanos();
    format!("{}_{}_{}", prefix, nanos, std::process::id())
}

fn temp_dir(prefix: &str) -> PathBuf {
    let mut dir = std::env::temp_dir();
    dir.push(unique_name(prefix));
    fs::create_dir_all(&dir).expect("create temp dir");
    dir
}

fn write_file(path: &Path, content: &str) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent");
    }
    fs::write(path, content).expect("write file");
}

#[test]
fn pack_to_string_uses_longer_fence() {
    let dir = temp_dir("pack_fence");
    let file_path = dir.join("sample.txt");
    write_file(&file_path, "line\n```\nmore");

    let bundle = pack_to_string(&dir, PackOptions::default()).expect("pack");
    assert!(bundle.contains("````txt\nline\n```\nmore\n````"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn unpack_from_str_handles_inner_fences() {
    let dir = temp_dir("unpack_inner");
    let markdown = "`foo.txt`:\n\n```txt\nline1\n```\nline2\n```\n\n`bar.txt`:\n\n```\nbar\n```\n";

    unpack_from_str(markdown, Some(&dir), UnpackOptions::default()).expect("unpack");

    let foo = fs::read_to_string(dir.join("foo.txt")).expect("read foo");
    let bar = fs::read_to_string(dir.join("bar.txt")).expect("read bar");
    assert_eq!(foo, "line1\n```\nline2");
    assert_eq!(bar, "bar");

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn unpack_rejects_parent_segments() {
    let dir = temp_dir("unpack_reject");
    let markdown = "`../oops`:\n\n```\nbad\n```\n";

    let result = unpack_from_str(markdown, Some(&dir), UnpackOptions::default());
    assert!(result.is_err());

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn unpack_defaults_to_project_dir() {
    let unique = unique_name("example");
    let markdown = format!("Project Path: /tmp/{unique}\n\n`foo.txt`:\n\n```\ncontent\n```\n");

    let output_dir = unpack_from_str(&markdown, None, UnpackOptions::default()).expect("unpack");
    let name = output_dir.file_name().and_then(|name| name.to_str());
    assert_eq!(name, Some(unique.as_str()));

    let content = fs::read_to_string(output_dir.join("foo.txt")).expect("read content");
    assert_eq!(content, "content");

    let _ = fs::remove_dir_all(output_dir);
}
