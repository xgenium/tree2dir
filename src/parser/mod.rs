//! ASCII / Unicode directory tree parser.
//!
//! Entry points:
//! - [`parse::parse_tree`] — convert raw text into a [`Vec<TreeNode>`].
//! - [`tree_node::TreeNode`] / [`tree_node::EntryKind`] — the AST types.

pub mod parse;
pub mod tree_node;

// Re-export the most commonly used items at the module boundary.
pub use parse::parse_tree;
pub use tree_node::EntryKind;
