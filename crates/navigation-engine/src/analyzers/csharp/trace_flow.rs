use std::path::Path;

use tree_sitter::{Node, Parser};

use crate::tree_sitter_ext::NodeExt;
use super::super::types::{CalleeDefinition, FindCalleesQuery, infer_public_language};
use super::common::node_text;

pub(super) fn find_callees(
    path: &Path,
    source: &str,
    query: &FindCalleesQuery,
) -> Vec<CalleeDefinition> {
    let mut parser = Parser::new();
    if parser
        .set_language(&tree_sitter_c_sharp::LANGUAGE.into())
        .is_err()
    {
        return Vec::new();
    }

    let Some(tree) = parser.parse(source, None) else {
        return Vec::new();
    };

    let public_language = infer_public_language(path);
    let mut callees = Vec::new();

    collect_csharp_callees(
        tree.root_node(),
        source.as_bytes(),
        path,
        public_language.as_deref(),
        &query.target_symbol,
        false,
        &mut callees,
    );

    callees
}

fn collect_csharp_callees(
    node: Node,
    source: &[u8],
    path: &Path,
    public_language: Option<&str>,
    target_symbol: &str,
    in_target: bool,
    callees: &mut Vec<CalleeDefinition>,
) {
    let mut is_now_in_target = in_target;

    if !in_target {
        if matches!(node.kind(), "method_declaration" | "constructor_declaration") {
            if let Some(name) = node.child_by_field_name("name").and_then(|n| node_text(n, source)) {
                if name == target_symbol {
                    is_now_in_target = true;
                } else {
                    // Check qualified name (Class.Method)
                    let mut parent = node.parent();
                    while let Some(p) = parent {
                        if matches!(p.kind(), "class_declaration" | "interface_declaration" | "struct_declaration" | "record_declaration") {
                            if let Some(class_name) = p.child_by_field_name("name").and_then(|n| node_text(n, source)) {
                                let qualified = format!("{}.{}", class_name, name);
                                if qualified == target_symbol {
                                    is_now_in_target = true;
                                    break;
                                }
                            }
                        }
                        parent = p.parent();
                    }
                }
            }
        }
    }

    if is_now_in_target {
        if node.kind() == "invocation_expression" {
            if let Some(callee) = extract_callee(node, source, path, public_language) {
                callees.push(callee);
            }
        } else if node.kind() == "object_creation_expression" {
            if let Some(callee) = extract_object_creation(node, source, path, public_language) {
                callees.push(callee);
            }
        }
    }

    for i in 0..node.named_child_count() {
        if let Some(child) = node.named_child_at(i) {
            collect_csharp_callees(
                child,
                source,
                path,
                public_language,
                target_symbol,
                is_now_in_target,
                callees,
            );
        }
    }
}

fn extract_callee(
    node: Node,
    source: &[u8],
    path: &Path,
    public_language: Option<&str>,
) -> Option<CalleeDefinition> {
    let function = node.child_by_field_name("function")?;
    
    let (name, receiver) = match function.kind() {
        "member_access_expression" => {
            let name = function.child_by_field_name("name").and_then(|n| node_text(n, source))?;
            let receiver = function.child_by_field_name("expression").and_then(|n| node_text(n, source));
            (name, receiver)
        }
        "identifier" => (node_text(function, source)?, None),
        _ => return None,
    };

    if is_noise(&name, receiver.as_deref()) {
        return None;
    }

    let callee_display = if let Some(ref r) = receiver {
        format!("{}.{}", r, name)
    } else {
        name
    };

    Some(CalleeDefinition {
        path: path.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        end_line: (node.end_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        callee: callee_display,
        callee_symbol: None,
        receiver_type: receiver,
        relation: "calls".to_string(),
        language: public_language.map(String::from),
        snippet: node_text(node, source),
    })
}

fn extract_object_creation(
    node: Node,
    source: &[u8],
    path: &Path,
    public_language: Option<&str>,
) -> Option<CalleeDefinition> {
    let type_node = node.child_by_field_name("type")?;
    let type_name = node_text(type_node, source)?;

    Some(CalleeDefinition {
        path: path.to_string_lossy().replace('\\', "/"),
        line: (node.start_position().row + 1) as u32,
        end_line: (node.end_position().row + 1) as u32,
        column: Some((node.start_position().column + 1) as u32),
        callee: type_name,
        callee_symbol: None,
        receiver_type: None,
        relation: "calls".to_string(),
        language: public_language.map(String::from),
        snippet: node_text(node, source),
    })
}

fn is_noise(name: &str, receiver: Option<&str>) -> bool {
    let noise_methods = ["ToString", "Equals", "GetHashCode", "GetType"];
    if noise_methods.contains(&name) {
        return true;
    }

    if let Some(r) = receiver {
        if r == "Math" || r == "Console" {
            return true;
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::analyzers::types::FindCalleesQuery;

    #[test]
    fn test_extract_basic_calls() {
        let source = r#"
            class Test {
                void Method() {
                    LocalCall();
                    other.RemoteCall();
                    new MyObject();
                }
            }
        "#;
        let query = FindCalleesQuery {
            target_symbol: "Method".to_string(),
        };
        let callees = find_callees(Path::new("test.cs"), source, &query);
        
        assert_eq!(callees.len(), 3);
        assert_eq!(callees[0].callee, "LocalCall");
        assert_eq!(callees[1].callee, "other.RemoteCall");
        assert_eq!(callees[2].callee, "MyObject");
    }

    #[test]
    fn test_qualified_target() {
        let source = r#"
            namespace MyNamespace;
            class Test {
                void Method() {
                    Call();
                }
            }
        "#;
        let query = FindCalleesQuery {
            target_symbol: "Test.Method".to_string(),
        };
        let callees = find_callees(Path::new("test.cs"), source, &query);
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].callee, "Call");
    }

    #[test]
    fn test_async_task_method() {
        let source = r#"
            public class OrderWorkflowService
            {
                public async Task<bool> ProcessOrderAsync(ProcessOrderRequest request)
                {
                    var draftOrder = await LoadDraftOrderAsync(request.OrderId);
                    return true;
                }
            }
        "#;
        let query = FindCalleesQuery {
            target_symbol: "OrderWorkflowService.ProcessOrderAsync".to_string(),
        };
        let callees = find_callees(Path::new("test.cs"), source, &query);
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].callee, "LoadDraftOrderAsync");
    }

    #[test]
    fn test_noise_filtering() {
        let source = r#"
            class Test {
                void Method() {
                    Console.WriteLine("hi");
                    Math.Min(1, 2);
                    obj.ToString();
                    ImportantCall();
                }
            }
        "#;
        let query = FindCalleesQuery {
            target_symbol: "Method".to_string(),
        };
        let callees = find_callees(Path::new("test.cs"), source, &query);
        assert_eq!(callees.len(), 1);
        assert_eq!(callees[0].callee, "ImportantCall");
    }

    #[test]
    fn test_static_calls() {
        let source = r#"
            class Test {
                void Method() {
                    Logger.Log("msg");
                    Database.Save(record);
                }
            }
        "#;
        let query = FindCalleesQuery {
            target_symbol: "Method".to_string(),
        };
        let callees = find_callees(Path::new("test.cs"), source, &query);
        assert_eq!(callees.len(), 2);
        assert_eq!(callees[0].callee, "Logger.Log");
        assert_eq!(callees[1].callee, "Database.Save");
    }
}
