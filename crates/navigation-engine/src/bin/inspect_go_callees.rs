use std::fs;
use std::path::PathBuf;

use navigation_engine::analyzers::types::FindCalleesQuery;
use navigation_engine::analyzers::{AnalyzerLanguage, AnalyzerRegistry};

fn main() {
    let path = std::env::args()
        .nth(1)
        .map(PathBuf::from)
        .unwrap_or_else(|| {
            PathBuf::from(
                "/home/j0k3r/navigation-agent-mcp/examples/go/internal/http/handlers/user_handler.go",
            )
        });
    let target_symbol = std::env::args()
        .nth(2)
        .unwrap_or_else(|| "UserHandler.CreateUser".to_string());
    let source = fs::read_to_string(&path).unwrap();
    let registry = AnalyzerRegistry::new();
    let analyzer = registry
        .analyzer_for_file(AnalyzerLanguage::Go, &path)
        .unwrap();
    let callees = analyzer.find_callees(&path, &source, &FindCalleesQuery { target_symbol });
    for callee in callees {
        println!(
            "{} | path={} | snippet={:?}",
            callee.callee, callee.path, callee.snippet
        );
    }
}
