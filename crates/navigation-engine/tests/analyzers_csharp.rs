use std::path::{Path, PathBuf};

use navigation_engine::analyzers::csharp::CsharpAnalyzer;
use navigation_engine::analyzers::language_analyzer::LanguageAnalyzer;
use navigation_engine::analyzers::types::{FindCallersQuery, FindSymbolQuery};

fn any_query() -> FindSymbolQuery {
    FindSymbolQuery {
        symbol: "".to_string(),
        kind: "any".to_string(),
        match_mode: "fuzzy".to_string(),
        public_language_filter: None,
        limit: 50,
    }
}

#[test]
fn extracts_csharp_definitions_with_public_kinds() {
    let analyzer = CsharpAnalyzer;
    let source = r#"
namespace Example
{
    public class User {}
    public interface IUserRepository { void Save(); }
    public record UserRecord(string Id);
    public enum UserType { Admin, Guest }

    public class UserService
    {
        public UserService() {}
        public void CreateUser() {}
    }
}
"#;

    let items = analyzer.find_symbols(
        Path::new("src/Example.cs"),
        source,
        &any_query(),
    );
    let kinds = items
        .iter()
        .map(|item| {
            (
                item.symbol.as_str(),
                item.kind.as_str(),
                item.language.as_deref(),
            )
        })
        .collect::<Vec<_>>();

    assert!(kinds.contains(&("User", "class", Some("csharp"))));
    assert!(kinds.contains(&("IUserRepository", "interface", Some("csharp"))));
    assert!(kinds.contains(&("UserRecord", "type", Some("csharp"))));
    assert!(kinds.contains(&("UserType", "enum", Some("csharp"))));
    assert!(kinds.contains(&("UserService", "class", Some("csharp"))));
    assert!(kinds.contains(&("UserService.UserService", "constructor", Some("csharp"))));
    assert!(kinds.contains(&("UserService.CreateUser", "method", Some("csharp"))));
}

#[test]
fn finds_csharp_methods_by_simple_name_in_exact_mode() {
    let analyzer = CsharpAnalyzer;
    let source = r#"
namespace Example
{
    public class UserService
    {
        public void CreateUser() {}
    }
}
"#;

    let items = analyzer.find_symbols(
        Path::new("src/Example.cs"),
        source,
        &FindSymbolQuery {
            symbol: "CreateUser".to_string(),
            kind: "method".to_string(),
            match_mode: "exact".to_string(),
            public_language_filter: Some("csharp".to_string()),
            limit: 10,
        },
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].symbol, "UserService.CreateUser");
}

#[test]
fn finds_csharp_callers_for_direct_and_member_calls() {
    let analyzer = CsharpAnalyzer;
    let source = r#"
namespace Example
{
    public class OrderService
    {
        private readonly IRepository _repo;
        public OrderService(IRepository repo) => _repo = repo;

        public async Task ProcessAsync(Guid id)
        {
            var order = await _repo.GetByIdAsync(id);
            if (order == null) throw new Exception();
            await SaveInternalAsync(order);
        }

        private Task SaveInternalAsync(Order order) => _repo.SaveAsync(order);
    }
}
"#;

    let repo_callers = analyzer.find_callers(
        Path::new("."),
        Path::new("src/Example.cs"),
        source,
        &FindCallersQuery {
            target_path: PathBuf::from("src/Interfaces.cs"),
            target_symbol: "GetByIdAsync".to_string(),
        },
    );

    assert_eq!(repo_callers.len(), 1);
    assert_eq!(repo_callers[0].caller, "OrderService.ProcessAsync");
    assert_eq!(repo_callers[0].receiver_type.as_deref(), Some("_repo"));

    let internal_callers = analyzer.find_callers(
        Path::new("."),
        Path::new("src/Example.cs"),
        source,
        &FindCallersQuery {
            target_path: PathBuf::from("src/Example.cs"),
            target_symbol: "SaveInternalAsync".to_string(),
        },
    );

    assert_eq!(internal_callers.len(), 1);
    assert_eq!(internal_callers[0].caller, "OrderService.ProcessAsync");
    assert_eq!(internal_callers[0].receiver_type, None);
}
