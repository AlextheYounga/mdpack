use crate::Result;
use std::collections::HashSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Clone, Copy, Debug, Default)]
pub struct UnpackOptions {
    pub force: bool,
}

pub fn unpack_from_path(
    input: &Path,
    output_dir: Option<&Path>,
    options: UnpackOptions,
) -> Result<PathBuf> {
    let markdown = fs::read_to_string(input)?;
    unpack_from_str(&markdown, output_dir, options)
}

pub fn unpack_from_str(
    markdown: &str,
    output_dir: Option<&Path>,
    options: UnpackOptions,
) -> Result<PathBuf> {
    let files = parse_bundle(markdown)?;
    if files.is_empty() {
        return Err("No files found in bundle".into());
    }

    let output_dir = match output_dir {
        Some(path) => path.to_path_buf(),
        None => std::env::current_dir()?,
    };
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
        if dest.exists() && !options.force {
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

    Ok(output_dir)
}

struct BundleFile {
    path: String,
    content: String,
}

fn parse_bundle(markdown: &str) -> Result<Vec<BundleFile>> {
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

fn sanitize_rel_path(path: &str) -> Result<PathBuf> {
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
                return Err(format!("Parent path segments not allowed: {}", path).into());
            }
            Component::Prefix(_) | Component::RootDir => {
                return Err(format!("Unsupported path: {}", path).into());
            }
        }
    }

    if result.as_os_str().is_empty() {
        return Err(format!("Empty path in bundle: {}", path).into());
    }

    Ok(result)
}
