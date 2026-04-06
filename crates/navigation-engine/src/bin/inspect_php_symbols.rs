use navigation_engine::analyzers::{
    language_analyzer::LanguageAnalyzer, php::PhpAnalyzer, FindSymbolQuery,
};
use std::env;
use std::fs;
use std::path::Path;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <php_file>", args[0]);
        std::process::exit(1);
    }

    let file_path = Path::new(&args[1]);
    let source = fs::read_to_string(file_path).expect("Failed to read file");

    let analyzer = PhpAnalyzer;
    let query = FindSymbolQuery {
        symbol: String::new(),
        kind: "any".to_string(),
        match_mode: "exact".to_string(),
        public_language_filter: None,
        limit: 100,
    };

    let symbols = analyzer.find_symbols(file_path, &source, &query);

    println!(
        "Found {} symbols in {}:",
        symbols.len(),
        file_path.display()
    );
    for symbol in symbols {
        println!(
            "  - {} ({}) at lines {}-{} [language: {:?}]",
            symbol.symbol, symbol.kind, symbol.line, symbol.line_end, symbol.language
        );
    }
}
