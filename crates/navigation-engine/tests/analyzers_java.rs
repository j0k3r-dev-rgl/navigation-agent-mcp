use std::path::Path;

use navigation_engine::analyzers::java::JavaAnalyzer;
use navigation_engine::analyzers::language_analyzer::LanguageAnalyzer;
use navigation_engine::analyzers::types::FindCalleesQuery;
use navigation_engine::analyzers::{FindCallersQuery, FindEndpointsQuery, FindSymbolQuery};

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

#[test]
fn finds_java_method_callers_and_marks_controller_entrypoints() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

@RestController
public class NavigationController {
    @GetMapping("/dashboard")
    public String getNavigation() {
        return loader();
    }
}
"#;

    let items = analyzer.find_callers(
        Path::new("src/main/java"),
        Path::new("src/main/java/com/example/NavigationController.java"),
        source,
        &FindCallersQuery {
            target_path: Path::new("src/main/java/com/example/DashboardService.java").to_path_buf(),
            target_symbol: "loader".to_string(),
        },
    );

    assert_eq!(items.len(), 1);
    assert_eq!(items[0].caller, "NavigationController#getNavigation");
    assert_eq!(items[0].relation, "calls");
    assert_eq!(
        items[0].probable_entry_point_reasons,
        vec!["public controller method"]
    );
}

// ============================================================================
// CalleeFilter Tests - Iteration 1 (Basic Filtering)
// ============================================================================

fn callees_query(target_symbol: &str) -> FindCalleesQuery {
    FindCalleesQuery {
        target_symbol: target_symbol.to_string(),
    }
}

// === Object Method Filtering Tests ===

#[test]
fn filters_object_methods_to_string_equals_hashcode() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

public class ExampleService {
    public void process(String input) {
        input.toString();
        input.equals("test");
        input.hashCode();
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/ExampleService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        !callee_names.contains(&"toString"),
        "toString should be filtered"
    );
    assert!(
        !callee_names.contains(&"equals"),
        "equals should be filtered"
    );
    assert!(
        !callee_names.contains(&"hashCode"),
        "hashCode should be filtered"
    );
}

#[test]
fn filters_object_methods_getclass_clone_notify() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

public class ExampleService {
    public void process(Object obj) {
        obj.getClass();
        obj.clone();
        obj.notify();
        obj.notifyAll();
        obj.wait();
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/ExampleService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        !callee_names.contains(&"getClass"),
        "getClass should be filtered"
    );
    assert!(!callee_names.contains(&"clone"), "clone should be filtered");
    assert!(
        !callee_names.contains(&"notify"),
        "notify should be filtered"
    );
    assert!(
        !callee_names.contains(&"notifyAll"),
        "notifyAll should be filtered"
    );
    assert!(!callee_names.contains(&"wait"), "wait should be filtered");
}

// === Constructor Preservation Tests ===

#[test]
fn preserves_constructors() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

public class ExampleService {
    public void create() {
        new TitularModel();
        new ResponseEntity<>();
        new java.util.ArrayList<>();
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/ExampleService.java"),
        source,
        &callees_query("create"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    // Note: Constructor names include generic syntax (<>) when present
    assert!(
        callee_names.contains(&"TitularModel"),
        "Project constructor should be preserved"
    );
    assert!(
        callee_names.contains(&"ResponseEntity<>"),
        "Spring constructor should be preserved"
    );
    assert!(
        callee_names.contains(&"java.util.ArrayList<>"),
        "Java constructor should be preserved"
    );
}

// === Package Filtering Tests ===

#[test]
fn filters_java_lang_methods() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

public class ExampleService {
    private String field;

    public void process() {
        field.length();
        field.substring(0, 5);
        field.trim();
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/ExampleService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        !callee_names.contains(&"length"),
        "String.length should be filtered (java.lang)"
    );
    assert!(
        !callee_names.contains(&"substring"),
        "String.substring should be filtered (java.lang)"
    );
    assert!(
        !callee_names.contains(&"trim"),
        "String.trim should be filtered (java.lang)"
    );
}

#[test]
fn filters_java_util_methods() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

import java.util.List;
import java.util.Map;

public class ExampleService {
    public void process() {
        List.of(1, 2, 3);
        Map.of("a", 1);
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/ExampleService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        !callee_names.contains(&"of"),
        "List.of/Map.of should be filtered (java.util)"
    );
}

// === Port Allowlist Tests ===

#[test]
fn preserves_port_methods() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

public class TitularService {
    private GetTitularByIdPort getTitularByIdPort;
    private EditTitularPort editTitularPort;

    public void process() {
        getTitularByIdPort.execute("123");
        editTitularPort.execute(new EditTitularRequest());
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/TitularService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        callee_names.contains(&"execute"),
        "Port.execute should be preserved"
    );
}

// === Model Getter/Setter Filtering Tests ===

#[test]
fn filters_getters_on_model_types() {
    let analyzer = JavaAnalyzer;
    // Note: Using class field instead of method parameter (parameters not tracked in Iteration 1)
    let source = r#"
package com.example;

public class TitularService {
    private TitularPersistenceModel model;

    public void process() {
        model.getNames();
        model.getSurnames();
        model.getEmail();
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/TitularService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        !callee_names.contains(&"getNames"),
        "Model getter should be filtered"
    );
    assert!(
        !callee_names.contains(&"getSurnames"),
        "Model getter should be filtered"
    );
    assert!(
        !callee_names.contains(&"getEmail"),
        "Model getter should be filtered"
    );
}

#[test]
fn filters_setters_on_model_types() {
    let analyzer = JavaAnalyzer;
    // Note: Using class field instead of method parameter (parameters not tracked in Iteration 1)
    let source = r#"
package com.example;

public class TitularService {
    private TitularPersistenceModel model;

    public void process() {
        model.setNames("Juan");
        model.setSurnames("Perez");
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/TitularService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        !callee_names.contains(&"setNames"),
        "Model setter should be filtered"
    );
    assert!(
        !callee_names.contains(&"setSurnames"),
        "Model setter should be filtered"
    );
}

#[test]
fn preserves_getters_on_non_model_types() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

public class TitularService {
    private GetTitularByIdService getTitularService;

    public void process() {
        getTitularService.getTitularById("123");
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/TitularService.java"),
        source,
        &callees_query("process"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    assert!(
        callee_names.contains(&"getTitularById"),
        "Service getter should be preserved"
    );
}

// === Spring Framework Filtering Tests ===

#[test]
fn filters_spring_internal_calls() {
    let analyzer = JavaAnalyzer;
    let source = r#"
package com.example;

import org.springframework.http.ResponseEntity;

public class TitularRestController {
    public ResponseEntity<TitularResponse> create() {
        return ResponseEntity.ok(new TitularResponse());
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/TitularRestController.java"),
        source,
        &callees_query("create"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    // ResponseEntity.ok should be filtered (org.springframework)
    assert!(
        !callee_names.contains(&"ok"),
        "ResponseEntity.ok should be filtered"
    );
    // But the constructor should be preserved
    assert!(
        callee_names.contains(&"TitularResponse"),
        "Project constructor should be preserved"
    );
}

// === Integration Tests ===

#[test]
fn filters_noise_in_realistic_service_method() {
    let analyzer = JavaAnalyzer;
    // Note: This test focuses on Iteration 1 capabilities:
    // - Class field type resolution (not method parameters)
    // - Object method filtering, java.util filtering, Port preservation
    // Builder chain tracking is Iteration 2
    let source = r#"
package com.example.modules.titular.application.use_cases;

import java.util.List;

public class CreateTitularUseCase {
    private EditTitularPort editTitularPort;
    private TitularMapper titularMapper;
    private TitularModel model;

    public TitularResponse execute() {
        // Project calls should be preserved
        TitularResult result = editTitularPort.execute(model);

        // Java utils should be filtered
        List<TitularResponse> list = List.of();

        // Object methods should be filtered
        model.toString();

        return titularMapper.toResponse(result);
    }
}
"#;

    let callees = analyzer.find_callees(
        Path::new("src/main/java/com/example/modules/titular/application/use_cases/CreateTitularUseCase.java"),
        source,
        &callees_query("execute"),
    );

    let callee_names: Vec<_> = callees.iter().map(|c| c.callee.as_str()).collect();

    // Should be filtered:
    assert!(
        !callee_names.contains(&"toString"),
        "Object.toString should be filtered"
    );
    assert!(!callee_names.contains(&"of"), "List.of should be filtered");

    // Should be preserved:
    assert!(
        callee_names.contains(&"execute"),
        "Port.execute should be preserved"
    );
    assert!(
        callee_names.contains(&"toResponse"),
        "Mapper.toResponse should be preserved"
    );
}
