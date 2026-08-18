//! Filesystem builder: translates a parsed [`TreeNode`] list into real paths.
//!
//! # Conflict handling
//!
//! When `--force` is **not** set:
//! - **Directories** that already exist are silently ignored — creating a
//!   directory that is already present is not an error.
//! - **Files** that already exist are skipped and a warning is emitted to
//!   standard error, but the build process continues.
//!
//! When `--force` **is** set, existing files are overwritten unconditionally.

use std::{
    fs,
    path::{Component, Path, PathBuf},
};

use crate::{
    error::AppError,
    parser::tree_node::{EntryKind, TreeNode},
};

/// Statistics collected during a build run.
#[derive(Debug, Default, PartialEq, Eq)]
pub struct BuildStats {
    /// Number of directories created.
    pub dirs_created: usize,
    /// Number of files created.
    pub files_created: usize,
    /// Number of files skipped due to conflicts (no `--force`).
    pub files_skipped: usize,
}

/// Options that govern how the builder behaves.
#[derive(Debug, Clone)]
pub struct BuildOptions {
    /// Root directory under which the entire structure is generated.
    pub output_root: PathBuf,
    /// When `true`, print the planned operations without writing to disk.
    pub dry_run: bool,
    /// When `true`, overwrite existing files rather than skipping them.
    pub force: bool,
}

/// Build a filesystem structure from a flat list of [`TreeNode`] values.
///
/// Nodes are consumed in order; a depth-stack tracks the current ancestor
/// chain so that each node's full path can be reconstructed without additional
/// metadata.
///
/// Returns [`BuildStats`] on success. Non-fatal warnings (e.g., skipped
/// files) are printed to `stderr`; fatal I/O errors are surfaced as
/// [`AppError`].
pub fn build(nodes: &[TreeNode], opts: &BuildOptions) -> Result<BuildStats, AppError> {
    if nodes.is_empty() {
        return Err(AppError::EmptyInput);
    }

    // Validate the output root exists (or create it).
    if !opts.dry_run {
        fs::create_dir_all(&opts.output_root).map_err(|e| AppError::io(&opts.output_root, e))?;
    }

    let mut stats = BuildStats::default();
    let resolved = resolve_paths(nodes, &opts.output_root)?;

    for (full_path, kind, _name, _indent) in resolved {
        match kind {
            EntryKind::Dir => create_directory(&full_path, opts, &mut stats)?,
            EntryKind::File => create_file(&full_path, opts, &mut stats)?,
        }
    }

    Ok(stats)
}

// ---------------------------------------------------------------------------
// Internal helpers
// ---------------------------------------------------------------------------

fn ensure_within_root(full_path: &Path, root: &Path) -> Result<(), AppError> {
    let root_c: Vec<_> = root.components().collect();
    let path_c: Vec<_> = full_path.components().collect();
    if path_c.len() < root_c.len() || path_c[..root_c.len()] != root_c[..] {
        return Err(AppError::UnsafePath(full_path.to_path_buf()));
    }
    Ok(())
}

fn resolve_paths(
    nodes: &[TreeNode],
    root: &Path,
) -> Result<Vec<(PathBuf, EntryKind, String, usize)>, AppError> {
    let mut stack: Vec<(usize, PathBuf)> = vec![(0, root.to_path_buf())];
    let mut paths = Vec::with_capacity(nodes.len());

    for node in nodes {
        // Guard against raw segment path traversal attempt (e.g., ../secret).
        if Path::new(&node.name).components().any(|c| {
            matches!(
                c,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        }) {
            return Err(AppError::UnsafePath(PathBuf::from(&node.name)));
        }

        // Unwind the stack until the top entry is a strict ancestor.
        while stack.len() > 1 && stack.last().map(|(d, _)| *d).unwrap_or(0) >= node.depth {
            stack.pop();
        }

        let parent = stack.last().map(|(_, p)| p.as_path()).unwrap_or(root);
        let full_path = parent.join(&node.name);

        // Guard against actual path traversal post-join.
        ensure_within_root(&full_path, root)?;

        let indent_level = stack.len().saturating_sub(1);

        if node.kind == EntryKind::Dir {
            stack.push((node.depth, full_path.clone()));
        }

        paths.push((
            full_path,
            node.kind.clone(),
            node.name.clone(),
            indent_level,
        ));
    }
    Ok(paths)
}

fn create_directory(
    path: &Path,
    opts: &BuildOptions,
    stats: &mut BuildStats,
) -> Result<(), AppError> {
    if opts.dry_run {
        println!("  [dry-run] 📁 {}", path.display());
        stats.dirs_created += 1;
        return Ok(());
    }

    // Silently ignore directories that already exist.
    if path.is_dir() {
        return Ok(());
    }

    fs::create_dir_all(path).map_err(|e| AppError::io(path, e))?;
    stats.dirs_created += 1;
    Ok(())
}

fn create_file(path: &Path, opts: &BuildOptions, stats: &mut BuildStats) -> Result<(), AppError> {
    if opts.dry_run {
        println!("  [dry-run] 📄 {}", path.display());
        stats.files_created += 1;
        return Ok(());
    }

    if path.exists() {
        if opts.force {
            // Overwrite: fall through to write.
        } else {
            // Skip with a non-fatal warning.
            eprintln!("  ⚠  warning: {}", AppError::FileExists(path.to_path_buf()));
            stats.files_skipped += 1;
            return Ok(());
        }
    }

    // Ensure all ancestor directories exist before writing the file.
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).map_err(|e| AppError::io(parent, e))?;
    }

    fs::write(path, b"").map_err(|e| AppError::io(path, e))?;
    stats.files_created += 1;
    Ok(())
}

/// Render a post-execution summary to stdout.
/// Render a post-execution summary to stdout.
pub fn print_summary(
    nodes: &[TreeNode],
    root: &Path,
    stats: &BuildStats,
    elapsed_ms: f64,
    dry_run: bool,
) {
    if dry_run {
        println!("\n✔  Dry-run complete — nothing was written to disk.\n");
    } else {
        println!("\n✔  Successfully generated structure!\n");
    }

    print_tree(nodes, root, 0, "  ");

    let dirs_str = pluralise(stats.dirs_created, "directory", "directories");
    let files_str = pluralise(stats.files_created, "file", "files");

    if dry_run {
        println!(
            "\nWould create {} {dirs_str} and {} {files_str} in {elapsed_ms:.1}ms.",
            stats.dirs_created, stats.files_created
        );
    } else {
        println!(
            "\nDone in {elapsed_ms:.1}ms. Created {} {dirs_str} and {} {files_str}.",
            stats.dirs_created, stats.files_created
        );
    }

    if stats.files_skipped > 0 {
        let skipped_str = pluralise(stats.files_skipped, "file", "files");
        eprintln!(
            "  ⚠  {} existing {skipped_str} skipped (use --force to overwrite).",
            stats.files_skipped
        );
    }
}

fn print_tree(nodes: &[TreeNode], root: &Path, _depth: usize, indent: &str) {
    if let Ok(resolved) = resolve_paths(nodes, root) {
        for (_full_path, kind, name, indent_level) in resolved {
            let display_indent = "  ".repeat(indent_level);
            let connector = if kind == EntryKind::Dir {
                "├── 📁"
            } else {
                "├── 📄"
            };
            println!("{indent}{display_indent}{connector} {name}");
        }
    }
}

fn pluralise<'a>(n: usize, singular: &'a str, plural: &'a str) -> &'a str {
    if n == 1 {
        singular
    } else {
        plural
    }
}
