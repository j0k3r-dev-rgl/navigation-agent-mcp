use std::path::Path;

use navigation_engine::analyzers::java::JavaAnalyzer;
use navigation_engine::analyzers::language_analyzer::LanguageAnalyzer;
use navigation_engine::analyzers::{FindEndpointsQuery, FindSymbolQuery};

fn any_symbol_query() -> FindSymbolQuery {
    FindSymbolQuery {
        symbol: "Example".to_string(),
        kind: "any".to_string(),
        match_mode: "exact".to_string(),
        public_language_filter: None,
        limit: 50,
    }
}

fn any_endpoint_query() -> FindEndpointsQuery {
    FindEndpointsQuery {
        kind: "any".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 50,
    }
}

fn rest_endpoint_query() -> FindEndpointsQuery {
    FindEndpointsQuery {
        kind: "rest".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 50,
    }
}

fn graphql_endpoint_query() -> FindEndpointsQuery {
    FindEndpointsQuery {
        kind: "graphql".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 50,
    }
}

#[test]
fn extracts_java_definitions_with_public_kinds() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package demo;

public @interface Audit {}

public interface ExamplePort {
    void execute();
}

public enum Status {
    ACTIVE,
    INACTIVE
}

public record ExampleRecord(String value) {}

public class ExampleService {
    public ExampleService() {}
    public void execute() {}
}
"#;

    let items = analyzer.find_symbols(
        Path::new("src/main/java/demo/ExampleService.java"),
        source,
        &any_symbol_query(),
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

    assert!(kinds.contains(&("Audit", "annotation", Some("java"))));
    assert!(kinds.contains(&("ExamplePort", "interface", Some("java"))));
    assert!(kinds.contains(&("Status", "enum", Some("java"))));
    assert!(kinds.contains(&("ExampleRecord", "type", Some("java"))));
    assert!(kinds.contains(&("ExampleService", "class", Some("java"))));
    assert!(kinds.contains(&("ExampleService", "constructor", Some("java"))));
    assert!(kinds.contains(&("execute", "method", Some("java"))));
}

#[test]
fn extracts_get_mapping_endpoint() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@RestController
public class TitularRestController {
    @GetMapping("/{id}")
    public TitularResponse getTitularById(@PathVariable String id) {
        return new TitularResponse();
    }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/TitularRestController.java"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "getTitularById");
    assert_eq!(endpoint.kind, "rest");
    assert_eq!(endpoint.path, Some("/{id}".to_string()));
    assert_eq!(endpoint.framework, Some("spring".to_string()));
    assert_eq!(endpoint.language, Some("java".to_string()));
    assert_eq!(endpoint.line, 6);
}

#[test]
fn extracts_post_mapping_with_class_base_path() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@RestController
@RequestMapping("/titulares")
public class TitularRestController {
    @PostMapping
    public ResponseEntity<TitularResponse> createTitular(@RequestBody CreateTitularRequest request) {
        return ResponseEntity.ok(new TitularResponse());
    }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/TitularRestController.java"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "createTitular");
    assert_eq!(endpoint.kind, "rest");
    assert_eq!(endpoint.path, Some("/titulares".to_string()));
    assert_eq!(endpoint.framework, Some("spring".to_string()));
}

#[test]
fn extracts_query_mapping_graphql() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@Controller
public class TitularGraphQLController {
    @QueryMapping
    public TitularDetailResponse getTitularById(@Argument String id) {
        return new TitularDetailResponse();
    }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/TitularGraphQLController.java"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "getTitularById");
    assert_eq!(endpoint.kind, "graphql");
    assert_eq!(endpoint.path, None); // GraphQL has no path
    assert_eq!(endpoint.framework, Some("spring".to_string()));
}

#[test]
fn extracts_mutation_mapping_graphql() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@Controller
public class TitularGraphQLController {
    @MutationMapping
    public TitularResponse createTitular(@Argument CreateTitularInput input) {
        return new TitularResponse();
    }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/TitularGraphQLController.java"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 1);

    let endpoint = &items[0];
    assert_eq!(endpoint.name, "createTitular");
    assert_eq!(endpoint.kind, "graphql");
}

#[test]
fn extracts_multiple_endpoints_in_same_controller() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@RestController
@RequestMapping("/titulares")
public class TitularRestController {
    @GetMapping
    public List<TitularResponse> listTitulares() {
        return List.of();
    }

    @GetMapping("/{id}")
    public TitularResponse getTitularById(@PathVariable String id) {
        return new TitularResponse();
    }

    @PostMapping
    public ResponseEntity<TitularResponse> createTitular(@RequestBody CreateRequest request) {
        return ResponseEntity.ok(new TitularResponse());
    }

    @PutMapping("/{id}")
    public ResponseEntity<TitularResponse> updateTitular(@PathVariable String id, @RequestBody UpdateRequest request) {
        return ResponseEntity.ok(new TitularResponse());
    }

    @DeleteMapping("/{id}")
    public ResponseEntity<Void> deleteTitular(@PathVariable String id) {
        return ResponseEntity.noContent().build();
    }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/TitularRestController.java"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 5);

    // Verify all endpoints have correct paths
    let paths: Vec<(String, Option<String>)> = items
        .iter()
        .map(|e| (e.name.clone(), e.path.clone()))
        .collect();

    assert!(paths.contains(&("listTitulares".to_string(), Some("/titulares".to_string()))));
    assert!(paths.contains(&(
        "getTitularById".to_string(),
        Some("/titulares/{id}".to_string())
    )));
    assert!(paths.contains(&("createTitular".to_string(), Some("/titulares".to_string()))));
    assert!(paths.contains(&(
        "updateTitular".to_string(),
        Some("/titulares/{id}".to_string())
    )));
    assert!(paths.contains(&(
        "deleteTitular".to_string(),
        Some("/titulares/{id}".to_string())
    )));
}

#[test]
fn filters_by_rest_kind() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@RestController
@RequestMapping("/api")
public class MixedController {
    @GetMapping("/items")
    public String getItems() { return ""; }

    @PostMapping("/items")
    public String createItem() { return ""; }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/MixedController.java"),
        source,
        &rest_endpoint_query(),
    );
    assert_eq!(items.len(), 2);

    for item in &items {
        assert_eq!(item.kind, "rest");
    }
}

#[test]
fn filters_by_graphql_kind() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@Controller
public class MixedGraphQLController {
    @QueryMapping
    public String getItem() { return ""; }

    @MutationMapping
    public String createItem() { return ""; }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/MixedGraphQLController.java"),
        source,
        &graphql_endpoint_query(),
    );
    assert_eq!(items.len(), 2);

    for item in &items {
        assert_eq!(item.kind, "graphql");
    }
}

#[test]
fn no_endpoints_for_non_controller_class() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

public class RegularService {
    @GetMapping("/items")
    public String getItems() { return ""; }
}
"#;

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/RegularService.java"),
        source,
        &any_endpoint_query(),
    );
    assert_eq!(items.len(), 0);
}

#[test]
fn respects_limit_parameter() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@RestController
public class LimitedController {
    @GetMapping("/one") public String one() { return ""; }
    @GetMapping("/two") public String two() { return ""; }
    @GetMapping("/three") public String three() { return ""; }
}
"#;

    let limited_query = FindEndpointsQuery {
        kind: "any".to_string(),
        public_language_filter: None,
        public_framework_filter: None,
        limit: 2,
    };

    let items = analyzer.find_endpoints(
        Path::new("src/main/java/com/example/LimitedController.java"),
        source,
        &limited_query,
    );
    assert_eq!(items.len(), 2);
}

#[test]
fn supports_spring_framework_filter() {
    let analyzer = JavaAnalyzer;
    assert!(analyzer.supports_framework(Some("spring")));
    assert!(!analyzer.supports_framework(Some("react-router")));
    assert!(analyzer.supports_framework(None));
}
