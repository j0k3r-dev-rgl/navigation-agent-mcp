use std::fs;
use std::path::PathBuf;

use tree_sitter::{Language, Parser};

fn main() {
    let path = PathBuf::from("/home/j0k3r/sias/app/front/app/routes/change-password.tsx");
    let source = fs::read_to_string(&path).expect("read source");

    let mut parser = Parser::new();
    let language = Language::new(tree_sitter_typescript::LANGUAGE_TSX);
    parser.set_language(&language).expect("set language");
    let tree = parser.parse(&source, None).expect("parse tree");

    let root = tree.root_node();
    if let Some(node) = find_smallest_containing(root, source.as_bytes(), "commitSession") {
        let mut current = Some(node);
        let mut chain = Vec::new();
        while let Some(item) = current {
            chain.push(item);
            current = item.parent();
        }
        for (depth, item) in chain.into_iter().take(8).enumerate() {
            println!("LEVEL {depth}");
            print_node(item, source.as_bytes(), 0);
            println!("----");
        }
    }
}

fn find_smallest_containing<'a>(
    node: tree_sitter::Node<'a>,
    source: &[u8],
    needle: &str,
) -> Option<tree_sitter::Node<'a>> {
    let text = node.utf8_text(source).ok()?;
    if !text.contains(needle) {
        return None;
    }

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            if let Some(found) = find_smallest_containing(child, source, needle) {
                return Some(found);
            }
        }
    }

    Some(node)
}

fn print_node(node: tree_sitter::Node<'_>, source: &[u8], depth: usize) {
    let snippet = node
        .utf8_text(source)
        .unwrap_or("")
        .replace('\n', "\\n")
        .chars()
        .take(160)
        .collect::<String>();
    println!(
        "{}{} [{}:{}-{}:{}] {}",
        "  ".repeat(depth),
        node.kind(),
        node.start_position().row + 1,
        node.start_position().column + 1,
        node.end_position().row + 1,
        node.end_position().column + 1,
        snippet
    );

    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            print_node(child, source, depth + 1);
        }
    }
}
