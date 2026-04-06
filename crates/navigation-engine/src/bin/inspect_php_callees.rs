use navigation_engine::analyzers::{
    language_analyzer::LanguageAnalyzer, php::PhpAnalyzer, types::FindCalleesQuery,
};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <php_file> <symbol>", args[0]);
        std::process::exit(1);
    }

    let file_path = Path::new(&args[1]);
    let symbol = &args[2];
    let source = fs::read_to_string(file_path).expect("Failed to read file");

    let analyzer = PhpAnalyzer;
    let query = FindCalleesQuery {
        target_symbol: symbol.to_string(),
    };

    let callees = analyzer.find_callees(file_path, &source, &query);

    println!(
        "Found {} callees for '{}' in {}:",
        callees.len(),
        symbol,
        file_path.display()
    );
    for callee in callees {
        println!(
            "  - {} at line {} (relation: {}) [receiver: {:?}]",
            callee.callee, callee.line, callee.relation, callee.receiver_type
        );
    }
}
