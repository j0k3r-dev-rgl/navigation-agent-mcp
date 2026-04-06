use navigation_engine::analyzers::{
    language_analyzer::LanguageAnalyzer, php::PhpAnalyzer, types::FindCallersQuery,
};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 4 {
        eprintln!("Usage: {} <target_file> <symbol> <search_in_file>", args[0]);
        std::process::exit(1);
    }

    let target_path = PathBuf::from(&args[1]);
    let symbol = &args[2];
    let search_file = Path::new(&args[3]);
    let source = fs::read_to_string(search_file).expect("Failed to read file");

    let analyzer = PhpAnalyzer;
    let query = FindCallersQuery {
        target_path,
        target_symbol: symbol.to_string(),
    };

    let workspace_root = Path::new(".");
    let callers = analyzer.find_callers(workspace_root, search_file, &source, &query);

    println!(
        "Found {} callers of '{}' in {}:",
        callers.len(),
        symbol,
        search_file.display()
    );
    for caller in callers {
        println!(
            "  - {} calls {} at line {} [receiver: {:?}]",
            caller.caller, symbol, caller.line, caller.receiver_type
        );
    }
}
