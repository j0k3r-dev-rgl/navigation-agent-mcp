use tree_sitter::{Language, Parser};

fn main() {
    let source = r#"
type UserRepository interface { Save() error }
type Result = string
import "example/app/internal/service"

type UserHandler struct { service *service.UserService }

func (h *UserHandler) CreateUser() {
    h.service.CreateUser()
    writeJSON()
}
"#;

    let mut parser = Parser::new();
    let language = Language::new(tree_sitter_go::LANGUAGE);
    parser.set_language(&language).unwrap();
    let tree = parser.parse(source, None).unwrap();
    print_node(tree.root_node(), source.as_bytes(), 0);
}

fn print_node(node: tree_sitter::Node<'_>, source: &[u8], depth: usize) {
    let snippet = node
        .utf8_text(source)
        .unwrap_or("")
        .replace('\n', "\\n")
        .chars()
        .take(120)
        .collect::<String>();
    println!("{}{} {}", "  ".repeat(depth), node.kind(), snippet);
    for index in 0..node.named_child_count() {
        if let Some(child) = node.named_child(index) {
            print_node(child, source, depth + 1);
        }
    }
}
