use clap::{Parser, Subcommand};
use std::collections::{BTreeMap, HashSet};
use std::error::Error;
use std::fs;
use std::io::{self, Write};
use std::path::{Component, Path, PathBuf};
use walkdir::{DirEntry, WalkDir};

#[derive(Parser)]
#[command(
    name = "mdpack",
    version,
    about = "Bundle and expand code2prompt-style markdown"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Pack {
        #[arg(value_name = "PATH", default_value = ".")]
        path: PathBuf,
        #[arg(short, long, value_name = "FILE")]
        output: Option<PathBuf>,
        #[arg(long)]
        include_hidden: bool,
    },
    Unpack {
        #[arg(value_name = "FILE")]
        input: PathBuf,
        #[arg(short, long, value_name = "DIR")]
        output: Option<PathBuf>,
        #[arg(long)]
        force: bool,
    },
}

fn main() -> Result<(), Box<dyn Error>> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Pack {
            path,
            output,
            include_hidden,
        } => pack_cmd(&path, output, include_hidden),
        Commands::Unpack {
            input,
            output,
            force,
        } => unpack_cmd(&input, output, force),
    }
}

fn pack_cmd(
    root: &Path,
    output: Option<PathBuf>,
    include_hidden: bool,
) -> Result<(), Box<dyn Error>> {
    if !root.is_dir() {
        return Err(format!("{} is not a directory", root.display()).into());
    }

    let root_abs = fs::canonicalize(root).unwrap_or_else(|_| root.to_path_buf());
    let files = collect_files(&root_abs, include_hidden)?;
    let tree = render_tree(&root_abs, &files);

    let mut bundle = String::new();
    bundle.push_str(&format!("Project Path: {}\n\n", root_abs.display()));
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

    match output {
        Some(path) => fs::write(path, bundle)?,
        None => {
            let mut stdout = io::stdout();
            stdout.write_all(bundle.as_bytes())?;
        }
    }

    Ok(())
}

fn unpack_cmd(input: &Path, output: Option<PathBuf>, force: bool) -> Result<(), Box<dyn Error>> {
    let markdown = fs::read_to_string(input)?;
    let files = parse_bundle(&markdown)?;
    if files.is_empty() {
        return Err("No files found in bundle".into());
    }

    let output_dir = output.unwrap_or_else(|| default_output_dir(&markdown));
    if output_dir.as_os_str().is_empty() {
        return Err("Output directory is empty".into());
    }

    let mut seen = HashSet::new();
    for file in &files {
        let rel = sanitize_rel_path(&file.path)?;
        if !seen.insert(rel.clone()) {
            return Err(format!("Duplicate path in bundle: {}", file.path).into());
        }
        let dest = output_dir.join(&rel);
        if dest.exists() && !force {
            return Err(format!("Refusing to overwrite {}", dest.display()).into());
        }
    }

    fs::create_dir_all(&output_dir)?;
    for file in files {
        let rel = sanitize_rel_path(&file.path)?;
        let dest = output_dir.join(&rel);
        if let Some(parent) = dest.parent() {
            fs::create_dir_all(parent)?;
        }
        fs::write(dest, file.content)?;
    }

    Ok(())
}

fn collect_files(root: &Path, include_hidden: bool) -> Result<Vec<PathBuf>, Box<dyn Error>> {
    let mut files = Vec::new();
    let walker = WalkDir::new(root)
        .follow_links(false)
        .into_iter()
        .filter_entry(|entry| !should_skip_entry(entry, include_hidden));

    for entry in walker {
        let entry = entry?;
        if entry.file_type().is_file() {
            files.push(entry.path().to_path_buf());
        }
    }

    files.sort_by(|a, b| {
        let a_rel = path_to_slash(a.strip_prefix(root).unwrap_or(a));
        let b_rel = path_to_slash(b.strip_prefix(root).unwrap_or(b));
        a_rel.cmp(&b_rel)
    });

    Ok(files)
}

fn should_skip_entry(entry: &DirEntry, include_hidden: bool) -> bool {
    if entry.depth() == 0 {
        return false;
    }
    let name = entry.file_name().to_string_lossy();
    if name == ".git" {
        return true;
    }
    if !include_hidden && name.starts_with('.') {
        return true;
    }
    false
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

struct BundleFile {
    path: String,
    content: String,
}

fn parse_bundle(markdown: &str) -> Result<Vec<BundleFile>, Box<dyn Error>> {
    let lines: Vec<String> = markdown.lines().map(|line| line.to_string()).collect();
    let mut files = Vec::new();
    let mut index = 0;

    while index < lines.len() {
        if let Some(path) = file_header_at(&lines, index) {
            let mut fence_index = index + 1;
            while fence_index < lines.len() && lines[fence_index].trim().is_empty() {
                fence_index += 1;
            }
            let fence = parse_fence_line(&lines[fence_index])
                .ok_or("Missing code fence after file header")?;

            let mut cursor = fence_index + 1;
            let mut closing = None;
            while cursor < lines.len() {
                if lines[cursor].trim() == fence.marker {
                    let mut lookahead = cursor + 1;
                    while lookahead < lines.len() && lines[lookahead].trim().is_empty() {
                        lookahead += 1;
                    }
                    if lookahead >= lines.len()
                        || file_header_at(&lines, lookahead).is_some()
                        || is_section_line(&lines[lookahead])
                    {
                        closing = Some(cursor);
                        break;
                    }
                }
                cursor += 1;
            }

            let closing = closing.ok_or_else(|| format!("Unterminated code block for {}", path))?;
            let content = lines[fence_index + 1..closing].join("\n");
            files.push(BundleFile { path, content });
            index = closing + 1;
            continue;
        }
        index += 1;
    }

    Ok(files)
}

fn file_header_at(lines: &[String], idx: usize) -> Option<String> {
    let line = lines[idx].trim();
    if !line.starts_with('`') || !line.ends_with("`:") {
        return None;
    }
    if line.len() <= 3 {
        return None;
    }

    let path = &line[1..line.len() - 2];
    if path.is_empty() {
        return None;
    }

    let mut lookahead = idx + 1;
    while lookahead < lines.len() && lines[lookahead].trim().is_empty() {
        lookahead += 1;
    }
    if lookahead >= lines.len() {
        return None;
    }
    if parse_fence_line(&lines[lookahead]).is_none() {
        return None;
    }

    Some(path.to_string())
}

#[derive(Clone)]
struct Fence {
    marker: String,
}

fn parse_fence_line(line: &str) -> Option<Fence> {
    let trimmed = line.trim();
    let bytes = trimmed.as_bytes();
    let mut count = 0;
    while count < bytes.len() && bytes[count] == b'`' {
        count += 1;
    }
    if count < 3 {
        return None;
    }
    let marker = "`".repeat(count);
    Some(Fence { marker })
}

fn is_section_line(line: &str) -> bool {
    let trimmed = line.trim();
    trimmed.starts_with("Git Diff:") || trimmed == "Git Diff"
}

fn default_output_dir(markdown: &str) -> PathBuf {
    for line in markdown.lines() {
        if let Some(rest) = line.trim().strip_prefix("Project Path:") {
            let value = rest.trim();
            if !value.is_empty() {
                let path = Path::new(value);
                if let Some(name) = path.file_name() {
                    let name = name.to_string_lossy();
                    if !name.is_empty() {
                        return PathBuf::from(name.to_string());
                    }
                }
            }
        }
    }
    PathBuf::from("unpacked")
}

fn sanitize_rel_path(path: &str) -> Result<PathBuf, Box<dyn Error>> {
    let cleaned = path.replace('\\', "/");
    let raw_path = Path::new(&cleaned);
    if raw_path.is_absolute() {
        return Err(format!("Absolute paths not allowed: {}", path).into());
    }

    let mut result = PathBuf::new();
    for component in raw_path.components() {
        match component {
            Component::Normal(name) => result.push(name),
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(format!("Parent path segments not allowed: {}", path).into())
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(format!("Unsupported path: {}", path).into())
            }
        }
    }

    if result.as_os_str().is_empty() {
        return Err(format!("Empty path in bundle: {}", path).into());
    }

    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fence_grows_for_nested_backticks() {
        let content = "line\n```\nmore";
        let fence = fence_for_content(content);
        assert_eq!(fence, "````");
    }

    #[test]
    fn parse_bundle_handles_inner_fences() {
        let markdown =
            "`foo.txt`:\n\n```txt\nline1\n```\nline2\n```\n\n`bar.txt`:\n\n```\nbar\n```\n";
        let files = parse_bundle(markdown).expect("parse bundle");
        assert_eq!(files.len(), 2);
        assert_eq!(files[0].path, "foo.txt");
        assert_eq!(files[0].content, "line1\n```\nline2");
        assert_eq!(files[1].path, "bar.txt");
        assert_eq!(files[1].content, "bar");
    }

    #[test]
    fn sanitize_rejects_parent_dir() {
        assert!(sanitize_rel_path("../oops").is_err());
    }

    #[test]
    fn default_output_dir_uses_project_path() {
        let markdown = "Project Path: /tmp/example\n";
        let dir = default_output_dir(markdown);
        assert_eq!(dir, PathBuf::from("example"));
    }
}
