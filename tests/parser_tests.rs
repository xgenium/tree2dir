//! Parser unit tests.
//!
//! These tests exercise [`tree2dir::parser::parse`] in isolation, covering
//! the edge cases listed in the technical specification:
//!
//! - Tab / space mixed indentation
//! - Trees that lack trailing slashes on directory names
//! - Deeply nested folder structures
//! - Comment and blank-line filtering
//! - Unicode scalar depth calculation correctness
//!
//! `serial_test` is used on tests that share global state (none currently)
//! or that must not run concurrently to avoid flaky failures.

use tree2dir::parser::{parse_tree, EntryKind};

// ---------------------------------------------------------------------------
// Basic parsing
// ---------------------------------------------------------------------------

#[test]
fn parses_simple_tree() {
    let input = "\
my-app/
├── src/
│   └── main.rs
└── README.md
";
    let nodes = parse_tree(input);
    assert_eq!(nodes.len(), 4);
    assert_eq!(nodes[0].kind, EntryKind::Dir); // my-app
    assert_eq!(nodes[1].kind, EntryKind::Dir); // src
    assert_eq!(nodes[2].kind, EntryKind::File); // main.rs
    assert_eq!(nodes[3].kind, EntryKind::File); // README.md
}

// ---------------------------------------------------------------------------
// Comment and blank-line filtering
// ---------------------------------------------------------------------------

#[test]
fn skips_blank_lines() {
    let input = "\
project/

└── main.rs
";
    let nodes = parse_tree(input);
    assert_eq!(nodes.len(), 2);
}

#[test]
fn skips_comment_lines() {
    let input = "\
project/
# This is a comment
└── main.rs
";
    let nodes = parse_tree(input);
    assert_eq!(nodes.len(), 2);
}

// ---------------------------------------------------------------------------
// Directory heuristics
// ---------------------------------------------------------------------------

#[test]
fn trailing_slash_forces_dir() {
    let nodes = parse_tree("standalone_dir/");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].kind, EntryKind::Dir);
    // The trailing slash must be stripped from the stored name.
    assert_eq!(nodes[0].name, "standalone_dir");
}

#[test]
fn lookahead_infers_dir_without_slash() {
    // "src" has no trailing slash, but the next line is deeper, so it must be
    // classified as a directory.
    let input = "\
src
    main.rs
";
    let nodes = parse_tree(input);
    assert_eq!(nodes[0].kind, EntryKind::Dir);
    assert_eq!(nodes[1].kind, EntryKind::File);
}

#[test]
fn leaf_without_slash_is_file() {
    let nodes = parse_tree("Cargo.toml");
    assert_eq!(nodes.len(), 1);
    assert_eq!(nodes[0].kind, EntryKind::File);
}

// ---------------------------------------------------------------------------
// Deeply nested structures
// ---------------------------------------------------------------------------

#[test]
fn deeply_nested_structure() {
    let input = "\
a/
├── b/
│   └── c/
│       └── d/
│           └── deep.txt
";
    let nodes = parse_tree(input);
    assert_eq!(nodes.len(), 5);
    assert_eq!(nodes[4].kind, EntryKind::File);
}

// ---------------------------------------------------------------------------
// Unicode scalar depth
// ---------------------------------------------------------------------------

#[test]
fn unicode_box_chars_counted_as_scalars() {
    // "│   " (1 box char + 3 spaces) must yield depth 4, not 6 (byte count).
    use tree2dir::parser::parse::split_prefix;
    let (depth, _) = split_prefix("│   file.txt").unwrap();
    assert_eq!(depth, 4);
}

// ---------------------------------------------------------------------------
// Tab indentation
// ---------------------------------------------------------------------------

#[test]
fn tab_indent_treated_as_single_scalar() {
    use tree2dir::parser::parse::split_prefix;
    // A single tab should produce depth 1.
    let (depth, name) = split_prefix("\tfile.txt").unwrap();
    assert_eq!(depth, 1);
    assert_eq!(name, "file.txt");
}

#[test]
fn mixed_tab_space_indentation() {
    // Each tab and space counts as one scalar, so two characters = depth 2.
    let input = "parent\n\t sub.txt";
    let nodes = parse_tree(input);
    // "parent" should be a Dir (lookahead) and "sub.txt" a File.
    assert_eq!(nodes[0].kind, EntryKind::Dir);
    assert_eq!(nodes[1].kind, EntryKind::File);
}
