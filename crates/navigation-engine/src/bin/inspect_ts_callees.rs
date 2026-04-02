use std::fs;
use std::path::PathBuf;

use navigation_engine::analyzers::types::FindCalleesQuery;
use navigation_engine::analyzers::{AnalyzerLanguage, AnalyzerRegistry};

fn main() {
    let path = PathBuf::from("/home/j0k3r/sias/app/front/app/routes/change-password.tsx");
    let source = fs::read_to_string(&path).expect("read source");
    let registry = AnalyzerRegistry::new();
    let analyzer = registry
        .analyzer_for_file(AnalyzerLanguage::Auto, &path)
        .expect("analyzer");
    let callees = analyzer.find_callees(
        &path,
        &source,
        &FindCalleesQuery {
            source_path: path.clone(),
            target_symbol: "action".to_string(),
        },
    );

    for callee in callees {
        println!(
            "{} | path={} | receiver={:?} | snippet={:?}",
            callee.callee, callee.path, callee.receiver_type, callee.snippet
        );
    }
}
