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

struct CurrentDirGuard {
    original: PathBuf,
}

impl CurrentDirGuard {
    fn new(target: &Path) -> Self {
        let original = std::env::current_dir().expect("current dir");
        std::env::set_current_dir(target).expect("set current dir");
        Self { original }
    }
}

impl Drop for CurrentDirGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original);
    }
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
fn pack_skips_gitignored_files_by_default() {
    let dir = temp_dir("pack_gitignore_default");
    write_file(&dir.join(".gitignore"), "ignored.txt\n");
    write_file(&dir.join("included.txt"), "keep");
    write_file(&dir.join("ignored.txt"), "skip");

    let bundle = pack_to_string(&dir, PackOptions::default()).expect("pack");
    assert!(bundle.contains("`included.txt`"));
    assert!(!bundle.contains("`ignored.txt`"));

    let _ = fs::remove_dir_all(dir);
}

#[test]
fn pack_includes_gitignored_files_with_ignored_flag() {
    let dir = temp_dir("pack_gitignore_flag");
    write_file(&dir.join(".gitignore"), "ignored.txt\n");
    write_file(&dir.join("ignored.txt"), "keep");

    let bundle = pack_to_string(
        &dir,
        PackOptions {
            include_hidden: false,
            include_ignored: true,
        },
    )
    .expect("pack");
    assert!(bundle.contains("`ignored.txt`"));

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
fn unpack_defaults_to_current_dir() {
    let dir = temp_dir("unpack_current");
    {
        let _guard = CurrentDirGuard::new(&dir);
        let markdown = "`foo.txt`:\n\n```\ncontent\n```\n";

        let output_dir = unpack_from_str(markdown, None, UnpackOptions::default()).expect("unpack");
        assert_eq!(output_dir, dir);

        let content = fs::read_to_string(dir.join("foo.txt")).expect("read content");
        assert_eq!(content, "content");
    }

    let _ = fs::remove_dir_all(dir);
}
