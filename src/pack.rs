use crate::Result;
use ignore::WalkBuilder;
use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Default)]
pub struct PackOptions {
    pub include_hidden: bool,
    pub include_ignored: bool,
}

pub fn pack_to_string(root: &Path, options: PackOptions) -> Result<String> {
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()).into());
    }

    let root_abs = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let files = collect_files(&root_abs, options)?;
    let tree = render_tree(&root_abs, &files);

    let mut bundle = String::new();
    bundle.push_str("Source Tree:\n\n```txt\n");
    bundle.push_str(&tree);
    if !tree.ends_with('\n') {
        bundle.push('\n');
    }
    bundle.push_str("```\n\n");

    for file in files {
        let rel = file.strip_prefix(&root_abs).unwrap_or(&file);
        let rel_str = path_to_slash(rel);

        let bytes = fs::read(&file)?;
        let content = match String::from_utf8(bytes) {
            Ok(text) => text,
            Err(_) => {
                eprintln!("Skipping non-UTF8 file: {}", rel_str);
                continue;
            }
        };

        let fence = fence_for_content(&content);
        let lang = language_for_path(rel);

        bundle.push_str(&format!("`{}`:\n\n", rel_str));
        bundle.push_str(&fence);
        if !lang.is_empty() {
            bundle.push_str(&lang);
        }
        bundle.push('\n');
        bundle.push_str(&content);
        if !content.ends_with('\n') {
            bundle.push('\n');
        }
        bundle.push_str(&fence);
        bundle.push_str("\n\n");
    }

    Ok(bundle)
}

pub fn pack_to_path(root: &Path, output: &Path, options: PackOptions) -> Result<()> {
    let bundle = pack_to_string(root, options)?;
    fs::write(output, bundle)?;
    Ok(())
}

fn collect_files(root: &Path, options: PackOptions) -> Result<Vec<PathBuf>> {
    let mut files = Vec::new();
    let mut walker = WalkBuilder::new(root);
    walker
        .follow_links(false)
        .hidden(!options.include_hidden)
        .parents(false)
        .git_ignore(!options.include_ignored)
        .git_exclude(false)
        .git_global(false)
        .require_git(false);

    for entry in walker.build() {
        let entry = entry?;
        let path = entry.path();
        if should_skip_path(path, root) {
            continue;
        }
        if path.is_file() {
            files.push(path.to_path_buf());
        }
    }

    files.sort_by(|a, b| {
        let a_rel = path_to_slash(a.strip_prefix(root).unwrap_or(a));
        let b_rel = path_to_slash(b.strip_prefix(root).unwrap_or(b));
        a_rel.cmp(&b_rel)
    });

    Ok(files)
}

fn should_skip_path(path: &Path, root: &Path) -> bool {
    let Ok(rel) = path.strip_prefix(root) else {
        return false;
    };

    rel.components()
        .any(|component| matches!(component, Component::Normal(name) if name == ".git"))
}

fn render_tree(root: &Path, files: &[PathBuf]) -> String {
    let mut root_node = TreeNode::default();
    for file in files {
        if let Ok(rel) = file.strip_prefix(root) {
            insert_path(&mut root_node, rel);
        }
    }

    let root_name = root
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| root.display().to_string());

    let mut lines = Vec::new();
    lines.push(root_name);
    render_tree_lines(&root_node, "", &mut lines);
    lines.join("\n")
}

#[derive(Default)]
struct TreeNode {
    children: BTreeMap<String, TreeNode>,
}

fn insert_path(node: &mut TreeNode, path: &Path) {
    let mut current = node;
    for component in path.components() {
        if let Component::Normal(name) = component {
            let name = name.to_string_lossy().to_string();
            current = current.children.entry(name).or_default();
        }
    }
}

fn render_tree_lines(node: &TreeNode, prefix: &str, lines: &mut Vec<String>) {
    let total = node.children.len();
    for (index, (name, child)) in node.children.iter().enumerate() {
        let last = index + 1 == total;
        let connector = if last { "`-- " } else { "|-- " };
        lines.push(format!("{}{}{}", prefix, connector, name));
        let next_prefix = if last {
            format!("{}    ", prefix)
        } else {
            format!("{}|   ", prefix)
        };
        render_tree_lines(child, &next_prefix, lines);
    }
}

fn path_to_slash(path: &Path) -> String {
    let mut parts = Vec::new();
    for component in path.components() {
        if let Component::Normal(name) = component {
            parts.push(name.to_string_lossy());
        }
    }
    parts.join("/")
}

fn language_for_path(path: &Path) -> String {
    path.extension()
        .and_then(|ext| ext.to_str())
        .filter(|ext| !ext.is_empty())
        .map(|ext| ext.to_string())
        .unwrap_or_else(|| "txt".to_string())
}

fn fence_for_content(content: &str) -> String {
    let mut max_run = 0;
    for line in content.lines() {
        let bytes = line.as_bytes();
        let mut idx = 0;
        while idx < bytes.len() && idx < 3 && bytes[idx] == b' ' {
            idx += 1;
        }
        let mut run = 0;
        while idx + run < bytes.len() && bytes[idx + run] == b'`' {
            run += 1;
        }
        if run > max_run {
            max_run = run;
        }
    }

    let len = std::cmp::max(3, max_run + 1);
    "`".repeat(len)
}
