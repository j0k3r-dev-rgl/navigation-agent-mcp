use std::env;
use std::fs;
use std::path::PathBuf;

use tree_sitter::Parser;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: inspect_python_ast <file_path> [symbol_to_find]");
        return;
    }

    let path = PathBuf::from(&args[1]);
    let source = fs::read_to_string(&path).expect("read source");

    let mut parser = Parser::new();
    parser
        .set_language(&tree_sitter_python::LANGUAGE.into())
        .expect("set language");
    let tree = parser.parse(&source, None).expect("parse tree");

    let root = tree.root_node();

    if args.len() > 2 {
        let needle = &args[2];
        if let Some(node) = find_smallest_containing(root, source.as_bytes(), needle) {
            println!("--- TREE AROUND SYMBOL '{}' ---", needle);
            let mut current = Some(node);
            let mut chain = Vec::new();
            while let Some(item) = current {
                chain.push(item);
                current = item.parent();
            }
            for (depth, item) in chain.into_iter().take(5).enumerate() {
                println!("LEVEL {depth}");
                print_node_summary(item, source.as_bytes());
                if item.kind() == "function_definition"
                    || item.kind() == "async_function_definition"
                {
                    if let Some(name_node) = item.child_by_field_name("name") {
                        println!("  NAME NODE KIND: {}", name_node.kind());
                        println!(
                            "  NAME NODE TEXT: '{}'",
                            name_node.utf8_text(source.as_bytes()).unwrap_or("")
                        );
                    }
                }
                println!("----");
            }
        } else {
            println!("Symbol '{}' not found in {}", needle, args[1]);
        }
    } else {
        println!("--- FULL AST FOR {} ---", args[1]);
        print_node(root, source.as_bytes(), 0, 5);
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

fn print_node_summary(node: tree_sitter::Node<'_>, source: &[u8]) {
    let snippet = node
        .utf8_text(source)
        .unwrap_or("")
        .replace('\n', "\\n")
        .chars()
        .take(160)
        .collect::<String>();
    println!(
        "{} [{}:{}-{}:{}] {}",
        node.kind(),
        node.start_position().row + 1,
        node.start_position().column + 1,
        node.end_position().row + 1,
        node.end_position().column + 1,
        snippet
    );
}

fn print_node(node: tree_sitter::Node<'_>, source: &[u8], depth: usize, max_depth: usize) {
    if depth > max_depth {
        return;
    }
    let snippet = node
        .utf8_text(source)
        .unwrap_or("")
        .replace('\n', "\\n")
        .chars()
        .take(80)
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
            print_node(child, source, depth + 1, max_depth);
        }
    }
}
