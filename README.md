# mdpack

Pack codebases into code2prompt-style Markdown bundles and expand them back into
files. Inspired by [code2prompt](https://github.com/mufeedvh/code2prompt).
Ships as both a CLI and a reusable Rust library.

## Features

- Bundle a directory into a single Markdown file.
- Restore a bundle back into a directory.
- Safe path handling (no absolute paths or parent traversal).
- Works as a CLI and as a library API.

## Install (CLI)

From crates.io (once published):

```sh
cargo install mdpack
```

From this repository:

```sh
cargo install --git https://github.com/AlextheYounga/mdpack.git
```

## CLI usage

Pack a directory:

```sh
mdpack pack ./my-project -o bundle.md
```

Unpack a bundle:

```sh
mdpack unpack bundle.md -o ./my-project
```

Options:

- `--include-hidden` to include dotfiles during packing.
- `--force` to overwrite existing files during unpacking.

## Library usage

Add as a dependency:

```toml
[dependencies]
mdpack = { git = "https://github.com/AlextheYounga/mdpack.git" }
```

Pack to a string or file:

```rust
use mdpack::{pack_to_path, pack_to_string, PackOptions};
use std::path::Path;

let options = PackOptions { include_hidden: false };
let bundle = pack_to_string(Path::new("./my-project"), options)?;
pack_to_path(Path::new("./my-project"), Path::new("bundle.md"), options)?;
```

Unpack from a string or file:

```rust
use mdpack::{unpack_from_path, unpack_from_str, UnpackOptions};
use std::path::Path;

let options = UnpackOptions { force: false };
let output = unpack_from_str("`foo.txt`:\n\n```\ncontent\n```\n", None, options)?;
unpack_from_path(Path::new("bundle.md"), Some(Path::new("./out")), options)?;
```

## Format

Bundles follow the code2prompt layout:

- `Project Path: ...`
- `Source Tree:` section with an ASCII tree
- Per-file blocks in the form:

````text
`path/to/file`:

```lang
<file contents>
```
````

## Tests

```sh
cargo test
```
