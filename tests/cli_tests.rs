//! Integration tests for the tree2dir CLI.
//!
//! These tests invoke the compiled binary through [`assert_cmd`] and use
//! [`tempfile`] to provide isolated temporary directories, ensuring no test
//! run pollutes the repository or the user's home directory.

use std::fs;
use std::path::Path;

use assert_cmd::Command;
use tempfile::TempDir;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Obtain a `Command` pointing to the compiled `tree2dir` binary.
fn cmd() -> Command {
    Command::cargo_bin("tree2dir").expect("tree2dir binary not found — run `cargo build` first")
}

/// Write `content` to a temporary file inside `dir` and return its path.
fn write_tree_file(dir: &TempDir, name: &str, content: &str) -> std::path::PathBuf {
    let path = dir.path().join(name);
    fs::write(&path, content).expect("failed to write temp tree file");
    path
}

// ---------------------------------------------------------------------------
// File Mode
// ---------------------------------------------------------------------------

#[test]
fn file_mode_creates_expected_structure() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    let tree = "\
my-project/
├── src/
│   └── main.rs
└── README.md
";
    let tree_file = write_tree_file(&input_dir, "tree.txt", tree);

    cmd()
        .args([
            "--file",
            tree_file.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    let root = output_dir.path().join("my-project");
    assert!(root.join("src").is_dir(), "src/ should be a directory");
    assert!(
        root.join("src/main.rs").is_file(),
        "src/main.rs should exist"
    );
    assert!(root.join("README.md").is_file(), "README.md should exist");
}

// ---------------------------------------------------------------------------
// Dry-run Mode
// ---------------------------------------------------------------------------

#[test]
fn dry_run_does_not_write_files() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    let tree = "ghost-app/\n└── ghost.rs\n";
    let tree_file = write_tree_file(&input_dir, "ghost.txt", tree);

    cmd()
        .args([
            "--file",
            tree_file.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
            "--dry-run",
        ])
        .assert()
        .success();

    // Nothing should have been written.
    assert!(
        !output_dir.path().join("ghost-app").exists(),
        "dry-run must not create any paths on disk"
    );
}

// ---------------------------------------------------------------------------
// Force flag — existing file overwrite
// ---------------------------------------------------------------------------

#[test]
fn force_flag_overwrites_existing_file() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    let tree = "README.md\n";
    let tree_file = write_tree_file(&input_dir, "tree.txt", tree);

    // Pre-create the file with sentinel content.
    let target = output_dir.path().join("README.md");
    fs::write(&target, b"old content").unwrap();

    cmd()
        .args([
            "--file",
            tree_file.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
            "--force",
        ])
        .assert()
        .success();

    // The file should have been overwritten (now empty).
    let content = fs::read(&target).unwrap();
    assert!(
        content.is_empty(),
        "existing file should be overwritten by --force"
    );
}

// ---------------------------------------------------------------------------
// No-force flag — existing file is skipped with a warning
// ---------------------------------------------------------------------------

#[test]
fn without_force_existing_file_is_skipped() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();

    let tree = "important.txt\n";
    let tree_file = write_tree_file(&input_dir, "tree.txt", tree);

    let target = output_dir.path().join("important.txt");
    fs::write(&target, b"preserved").unwrap();

    // Run WITHOUT --force.
    cmd()
        .args([
            "--file",
            tree_file.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success(); // Must not crash — skipping is a warning, not a fatal error.

    // The file's original content must be intact.
    let content = fs::read(&target).unwrap();
    assert_eq!(
        content, b"preserved",
        "existing file must not be overwritten without --force"
    );
}

// ---------------------------------------------------------------------------
// Structural Quine Test
//
// Feed `example_tree.txt` (which describes a typical project layout) to
// tree2dir, generate it inside a temporary folder, and verify that every
// expected path exists on disk.
// ---------------------------------------------------------------------------

#[test]
fn structural_quine_example_tree() {
    let output_dir = TempDir::new().unwrap();

    // The example file is part of the repository and always available.
    let example_file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("example_tree.txt");

    assert!(
        example_file.exists(),
        "examples/example_tree.txt must exist"
    );

    cmd()
        .args([
            "--file",
            example_file.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .success();

    // Verify the top-level anchor from the example file exists.
    let root = output_dir.path().join("my-app");
    assert!(root.is_dir(), "my-app/ should exist");
    assert!(root.join("src").is_dir(), "my-app/src/ should exist");
    assert!(
        root.join("src/main.rs").is_file(),
        "my-app/src/main.rs should exist"
    );
    assert!(
        root.join("src/config.rs").is_file(),
        "my-app/src/config.rs should exist"
    );
    assert!(
        root.join("README.md").is_file(),
        "my-app/README.md should exist"
    );
}

// ---------------------------------------------------------------------------
// Empty / invalid input
// ---------------------------------------------------------------------------

#[test]
fn empty_input_exits_with_error() {
    let input_dir = TempDir::new().unwrap();
    let output_dir = TempDir::new().unwrap();
    let tree_file = write_tree_file(&input_dir, "empty.txt", "");

    cmd()
        .args([
            "--file",
            tree_file.to_str().unwrap(),
            "--output",
            output_dir.path().to_str().unwrap(),
        ])
        .assert()
        .failure();
}
