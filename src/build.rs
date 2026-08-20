//! The build-script half: compile a project's whole documentation set
//! into its binary.
//!
//! A consumer names no files. `diataxis.yml` already says where the
//! pages live, so a build script reads that and embeds every page:
//!
//! ```ignore
//! // build.rs
//! fn main() -> std::io::Result<()> {
//!     diataxis::build::embed_docs()
//! }
//! ```
//!
//! ```ignore
//! // src/docs.rs
//! static DOCS: &[(&str, &str)] = diataxis::embedded_docs!();
//! ```
//!
//! A page added to the directory is in the next build. Nothing lists it
//! twice, and nothing lists it at all.
//!
//! This module carries no dependencies, so a build-dependency on this
//! crate compiles almost nothing:
//!
//! ```toml
//! [build-dependencies]
//! diataxis = { version = "0.1", default-features = false, features = ["build"] }
//! ```

use std::path::{Path, PathBuf};

/// The generated file, inside `OUT_DIR`.
pub const GENERATED: &str = "diataxis_docs.rs";

/// Embed every page of this project's documentation set.
///
/// Reads `docs_dir` from `diataxis.yml` beside `Cargo.toml`, or uses
/// `docs` when there is no config. Writes the array
/// [`embedded_docs!`](crate::embedded_docs) reads.
///
/// # Errors
///
/// Fails when the pages directory cannot be read, or when the generated
/// file cannot be written.
pub fn embed_docs() -> std::io::Result<()> {
    let manifest = std::env::var("CARGO_MANIFEST_DIR").map_err(|_| {
        std::io::Error::other("CARGO_MANIFEST_DIR is unset; run this from build.rs")
    })?;
    embed_docs_from(Path::new(&manifest))
}

/// Embed the documentation set rooted at `root`.
///
/// # Errors
///
/// Fails when the pages directory cannot be read, or when the generated
/// file cannot be written.
pub fn embed_docs_from(root: &Path) -> std::io::Result<()> {
    let out_dir = std::env::var("OUT_DIR")
        .map_err(|_| std::io::Error::other("OUT_DIR is unset; run this from build.rs"))?;
    let dir = root.join(docs_dir(root));
    let generated = generate(&dir)?;
    std::fs::write(Path::new(&out_dir).join(GENERATED), generated)
}

/// Where the pages live, as `diataxis.yml` says.
///
/// A build script must not pull a YAML parser in for one key, so this
/// reads the one key. A value it cannot read leaves the default, and
/// the build embeds what `docs` holds rather than failing.
fn docs_dir(root: &Path) -> String {
    let Ok(text) = std::fs::read_to_string(root.join("diataxis.yml")) else {
        return "docs".to_string();
    };
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("docs_dir:") {
            let value = value.trim().trim_matches(['"', '\'']);
            if !value.is_empty() && !value.starts_with('/') && !value.contains("..") {
                return value.to_string();
            }
        }
    }
    "docs".to_string()
}

/// The array of `(path, content)` pairs, as Rust source.
///
/// Every page is emitted with `include_str!`, so the compiler tracks
/// its content. The directory is emitted as `rerun-if-changed`, so a
/// page added or removed rebuilds too.
fn generate(dir: &Path) -> std::io::Result<String> {
    println!("cargo:rerun-if-changed={}", dir.display());
    let mut out = String::from("&[\n");
    for path in pages_in(dir)? {
        println!("cargo:rerun-if-changed={}", path.display());
        let literal = path.display().to_string();
        out.push_str(&format!("    ({literal:?}, include_str!({literal:?})),\n"));
    }
    out.push(']');
    Ok(out)
}

/// Every markdown file in the directory, sorted by name.
///
/// A missing directory holds no pages. A project that has not written
/// its documentation yet still builds, and `docs` lists nothing.
fn pages_in(dir: &Path) -> std::io::Result<Vec<PathBuf>> {
    let entries = match std::fs::read_dir(dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            println!(
                "cargo:warning=diataxis: no pages directory at {}",
                dir.display()
            );
            return Ok(Vec::new());
        }
        Err(e) => return Err(e),
    };
    let mut paths = Vec::new();
    for entry in entries {
        let path = entry?.path();
        if path.extension().is_some_and(|e| e == "md") {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("diataxis-build-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn every_page_in_the_directory_is_emitted() {
        let root = temp_dir("all");
        let docs = root.join("docs");
        std::fs::create_dir_all(&docs).unwrap();
        std::fs::write(docs.join("b-second.md"), "b").unwrap();
        std::fs::write(docs.join("a-first.md"), "a").unwrap();
        std::fs::write(docs.join("notes.txt"), "not a page").unwrap();

        let generated = generate(&docs).unwrap();
        assert_eq!(generated.matches("include_str!").count(), 2);
        // Sorted, so a build is the same twice.
        let first = generated.find("a-first.md").unwrap();
        let second = generated.find("b-second.md").unwrap();
        assert!(first < second);
        assert!(!generated.contains("notes.txt"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_missing_directory_holds_no_pages() {
        let root = temp_dir("missing");
        let generated = generate(&root.join("docs")).unwrap();
        assert_eq!(generated, "&[\n]");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn the_config_names_the_directory() {
        let root = temp_dir("configured");
        std::fs::write(root.join("diataxis.yml"), "docs_dir: guides\ntitle: acme\n").unwrap();
        assert_eq!(docs_dir(&root), "guides");
    }

    #[test]
    fn no_config_leaves_the_default() {
        let root = temp_dir("default");
        assert_eq!(docs_dir(&root), "docs");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn a_directory_that_climbs_out_is_refused() {
        let root = temp_dir("escape");
        std::fs::write(root.join("diataxis.yml"), "docs_dir: ../../etc\n").unwrap();
        assert_eq!(docs_dir(&root), "docs");
        std::fs::write(root.join("diataxis.yml"), "docs_dir: /etc\n").unwrap();
        assert_eq!(docs_dir(&root), "docs");
        let _ = std::fs::remove_dir_all(&root);
    }
}
