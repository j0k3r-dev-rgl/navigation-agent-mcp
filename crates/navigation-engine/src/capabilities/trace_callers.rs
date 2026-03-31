use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::path::{Path, PathBuf};

use crate::analyzers::{AnalyzerLanguage, AnalyzerRegistry, CallerDefinition, FindCallersQuery};
use crate::error::EngineError;
use crate::protocol::{
    EngineRequest, EngineResponse, TraceCallersCallsTarget, TraceCallersClassificationRecord,
    TraceCallersItem, TraceCallersProbableEntryPoint, TraceCallersRecursiveClassifications,
    TraceCallersRecursiveCycle, TraceCallersRecursiveNode, TraceCallersRecursivePathSegment,
    TraceCallersRecursiveResult, TraceCallersRecursiveSummary, TraceCallersRecursiveVia,
    TraceCallersRequestPayload, TraceCallersResult,
};
use crate::workspace::{
    canonicalize_workspace_root, collect_supported_source_files, contains_hard_ignored_segment,
    public_path, resolve_scope,
};

use super::find_symbol::find_symbol;

pub const CAPABILITY: &str = "workspace.trace_callers";
const DEFAULT_MAX_DEPTH: u32 = 3;

pub fn handle(request: EngineRequest) -> EngineResponse {
    let parsed_payload =
        serde_json::from_value::<TraceCallersRequestPayload>(request.payload.clone());

    match parsed_payload {
        Ok(payload) => match trace_callers(&request.workspace_root, payload) {
            Ok(result) => EngineResponse::success(request.id, &result),
            Err(error) => EngineResponse::error(request.id, error),
        },
        Err(error) => {
            EngineResponse::error(request.id, EngineError::invalid_request(error.to_string()))
        }
    }
}

pub fn trace_callers(
    workspace_root: &str,
    payload: TraceCallersRequestPayload,
) -> Result<TraceCallersResult, EngineError> {
    let workspace_root = canonicalize_workspace_root(workspace_root)?;
    let scope = resolve_scope(&workspace_root, Some(payload.path.as_str()))?;

    if !scope.absolute_path.is_file() {
        return Err(EngineError::file_not_found(payload.path.as_str()));
    }

    if contains_hard_ignored_segment(&workspace_root, &scope.absolute_path) {
        return Ok(TraceCallersResult {
            resolved_path: Some(scope.public_path),
            items: Vec::new(),
            total_matched: 0,
            truncated: false,
            recursive: None,
        });
    }

    let analyzer_language = parse_analyzer_language(&payload.analyzer_language)?;
    let registry = AnalyzerRegistry::new();
    let supported_extensions = registry.supported_extensions(analyzer_language);
    let files = collect_supported_source_files(
        &workspace_root,
        &resolve_scope(&workspace_root, None)?,
        &supported_extensions,
        false,
    )?;

    let symbol_check = find_symbol(
        workspace_root.to_string_lossy().as_ref(),
        crate::protocol::FindSymbolRequestPayload {
            symbol: payload.symbol.clone(),
            path: Some(payload.path.clone()),
            analyzer_language: payload.analyzer_language.clone(),
            public_language_filter: payload.public_language_filter.clone(),
            kind: "any".to_string(),
            match_mode: "exact".to_string(),
            limit: usize::MAX,
        },
    )?;

    if symbol_check.total_matched == 0 {
        return Err(EngineError::symbol_not_found(
            payload.symbol.as_str(),
            payload.path.as_str(),
        ));
    }

    let direct = find_workspace_callers(
        &workspace_root,
        &files,
        &registry,
        analyzer_language,
        &scope.absolute_path,
        scope.public_path.as_str(),
        payload.symbol.as_str(),
    )?;

    let items = direct.iter().map(map_item).collect::<Vec<_>>();
    let recursive = if payload.recursive {
        Some(build_recursive_result(
            &workspace_root,
            &files,
            &registry,
            analyzer_language,
            &scope.absolute_path,
            scope.public_path.as_str(),
            payload.symbol.as_str(),
            payload.max_depth.unwrap_or(DEFAULT_MAX_DEPTH),
        )?)
    } else {
        None
    };

    Ok(TraceCallersResult {
        resolved_path: Some(scope.public_path),
        total_matched: items.len(),
        items,
        truncated: false,
        recursive,
    })
}

fn parse_analyzer_language(value: &str) -> Result<AnalyzerLanguage, EngineError> {
    match value {
        "auto" => Ok(AnalyzerLanguage::Auto),
        "java" => Ok(AnalyzerLanguage::Java),
        "python" => Ok(AnalyzerLanguage::Python),
        "rust" => Ok(AnalyzerLanguage::Rust),
        "typescript" => Ok(AnalyzerLanguage::Typescript),
        other => Err(EngineError::invalid_request(format!(
            "Unsupported analyzerLanguage '{}'.",
            other
        ))),
    }
}

fn find_workspace_callers(
    workspace_root: &Path,
    files: &[PathBuf],
    registry: &AnalyzerRegistry,
    analyzer_language: AnalyzerLanguage,
    target_path: &Path,
    target_public_path: &str,
    target_symbol: &str,
) -> Result<Vec<CallerDefinition>, EngineError> {
    let mut callers = Vec::new();

    for file_path in files {
        let Some(analyzer) = registry.analyzer_for_file(analyzer_language, file_path) else {
            continue;
        };

        let source = std::fs::read_to_string(file_path)
            .map_err(|error| EngineError::backend_execution_failed(error.to_string()))?;
        let file_public_path = public_path(workspace_root, file_path);
        let query = FindCallersQuery {
            target_path: target_path.to_path_buf(),
            target_symbol: target_symbol.to_string(),
        };

        callers.extend(
            analyzer
                .find_callers(workspace_root, file_path, &source, &query)
                .into_iter()
                .map(|mut caller| {
                    caller.path = file_public_path.clone();
                    caller.calls.path = target_public_path.to_string();
                    caller
                }),
        );
    }

    callers.sort_by(|left, right| {
        (
            &left.path,
            left.line,
            left.column.unwrap_or(0),
            &left.caller,
            &left.relation,
        )
            .cmp(&(
                &right.path,
                right.line,
                right.column.unwrap_or(0),
                &right.caller,
                &right.relation,
            ))
    });

    callers.dedup_by(|left, right| {
        left.path == right.path
            && left.line == right.line
            && left.column == right.column
            && left.caller == right.caller
            && left.caller_symbol == right.caller_symbol
            && left.relation == right.relation
    });

    Ok(callers)
}

fn build_recursive_result(
    workspace_root: &Path,
    files: &[PathBuf],
    registry: &AnalyzerRegistry,
    analyzer_language: AnalyzerLanguage,
    root_path: &Path,
    root_public_path: &str,
    root_symbol: &str,
    max_depth: u32,
) -> Result<TraceCallersRecursiveResult, EngineError> {
    let root_key = node_key(root_public_path, root_symbol);
    let root_node = TraceCallersRecursiveNode {
        key: root_key.clone(),
        path: root_public_path.to_string(),
        symbol: root_symbol.to_string(),
        depth: 0,
        via: None,
    };

    let mut queue = VecDeque::from([(
        root_path.to_path_buf(),
        root_public_path.to_string(),
        root_symbol.to_string(),
        0u32,
    )]);
    let mut nodes = BTreeMap::from([(root_key.clone(), root_node.clone())]);
    let mut parents = BTreeMap::<String, String>::new();
    let mut adjacency = BTreeMap::<String, BTreeSet<String>>::new();
    let mut direct_callers = Vec::new();
    let mut indirect_callers = Vec::new();
    let mut probable_public_entry_points = Vec::new();
    let mut cycles = Vec::new();
    let mut seen = BTreeSet::from([root_key.clone()]);

    while let Some((target_abs_path, target_public, target_symbol, depth)) = queue.pop_front() {
        if depth >= max_depth {
            continue;
        }

        let target_key = node_key(&target_public, &target_symbol);
        let callers = find_workspace_callers(
            workspace_root,
            files,
            registry,
            analyzer_language,
            &target_abs_path,
            &target_public,
            &target_symbol,
        )?;

        for caller in callers {
            let caller_symbol = caller
                .caller_symbol
                .clone()
                .unwrap_or_else(|| caller.caller.clone());
            let child_depth = depth + 1;
            let child_key = node_key(&caller.path, &caller_symbol);

            if path_contains(&parents, &target_key, &child_key) {
                cycles.push(TraceCallersRecursiveCycle {
                    from_key: target_key.clone(),
                    to_key: child_key.clone(),
                    path: ancestor_path_keys(&parents, &target_key),
                });
                continue;
            }

            adjacency
                .entry(target_key.clone())
                .or_default()
                .insert(child_key.clone());

            nodes
                .entry(child_key.clone())
                .or_insert_with(|| TraceCallersRecursiveNode {
                    key: child_key.clone(),
                    path: caller.path.clone(),
                    symbol: caller_symbol.clone(),
                    depth: child_depth,
                    via: Some(TraceCallersRecursiveVia {
                        relation: Some(caller.relation.clone()),
                        line: Some(caller.line),
                        column: caller.column,
                        snippet: caller.snippet.clone(),
                    }),
                });

            if seen.insert(child_key.clone()) {
                parents.insert(child_key.clone(), target_key.clone());
                queue.push_back((
                    workspace_root.join(&caller.path),
                    caller.path.clone(),
                    caller_symbol.clone(),
                    child_depth,
                ));
            }

            let classification = map_classification(
                &caller,
                child_depth,
                &target_public,
                &target_symbol,
                &parents,
                &nodes,
                &child_key,
            );
            if child_depth == 1 {
                direct_callers.push(classification.clone());
            } else {
                indirect_callers.push(classification.clone());
            }

            if !caller.probable_entry_point_reasons.is_empty() {
                probable_public_entry_points.push(TraceCallersProbableEntryPoint {
                    key: Some(child_key),
                    path: caller.path.clone(),
                    symbol: caller_symbol,
                    depth: Some(child_depth),
                    reasons: caller.probable_entry_point_reasons.clone(),
                    probable: Some(true),
                    path_from_target: classification.path_from_target.clone(),
                });
            }
        }
    }

    let mut node_list = nodes.into_values().collect::<Vec<_>>();
    node_list.sort_by(|left, right| {
        (&left.path, left.depth, &left.symbol).cmp(&(&right.path, right.depth, &right.symbol))
    });

    let adjacency = adjacency
        .into_iter()
        .map(|(key, values)| (key, values.into_iter().collect::<Vec<_>>()))
        .collect::<BTreeMap<_, _>>();

    let paths = build_paths(&parents, &node_list, &root_key, &adjacency);
    let mut probable_entry_points = probable_public_entry_points.clone();
    probable_entry_points.sort_by(|left, right| {
        (&left.path, left.depth, &left.symbol).cmp(&(&right.path, right.depth, &right.symbol))
    });
    probable_entry_points.dedup_by(|left, right| {
        left.path == right.path && left.symbol == right.symbol && left.depth == right.depth
    });

    Ok(TraceCallersRecursiveResult {
        enabled: true,
        root: root_node,
        max_depth: max_depth.max(1),
        max_depth_observed: node_list.iter().map(|node| node.depth).max().unwrap_or(0),
        node_count: node_list.len(),
        edge_count: direct_callers.len() + indirect_callers.len(),
        path_count: paths.len(),
        nodes: node_list,
        adjacency,
        paths,
        cycles,
        truncated: Vec::new(),
        probable_entry_points: probable_entry_points.clone(),
        classifications: TraceCallersRecursiveClassifications {
            summary: TraceCallersRecursiveSummary {
                direct_caller_count: direct_callers.len(),
                indirect_caller_count: indirect_callers.len(),
                probable_public_entry_point_count: probable_entry_points.len(),
                implementation_interface_chain_count: 0,
            },
            direct_callers,
            indirect_callers,
            probable_public_entry_points: probable_entry_points,
            implementation_interface_chain: Vec::new(),
        },
    })
}

fn map_item(item: &CallerDefinition) -> TraceCallersItem {
    TraceCallersItem {
        path: item.path.clone(),
        line: item.line,
        column: item.column,
        caller: item.caller.clone(),
        caller_symbol: item.caller_symbol.clone(),
        relation: item.relation.clone(),
        language: item.language.clone(),
        snippet: item.snippet.clone(),
        receiver_type: item.receiver_type.clone(),
    }
}

fn map_classification(
    caller: &CallerDefinition,
    depth: u32,
    target_public_path: &str,
    target_symbol: &str,
    parents: &BTreeMap<String, String>,
    nodes: &BTreeMap<String, TraceCallersRecursiveNode>,
    child_key: &str,
) -> TraceCallersClassificationRecord {
    TraceCallersClassificationRecord {
        path: caller.path.clone(),
        symbol: caller
            .caller_symbol
            .clone()
            .unwrap_or_else(|| caller.caller.clone()),
        caller: caller.caller.clone(),
        depth,
        line: caller.line,
        column: caller.column,
        relation: caller.relation.clone(),
        language: caller.language.clone(),
        receiver_type: caller.receiver_type.clone(),
        snippet: caller.snippet.clone(),
        calls: TraceCallersCallsTarget {
            path: target_public_path.to_string(),
            symbol: target_symbol.to_string(),
        },
        path_from_target: build_path_from_target(parents, nodes, child_key),
    }
}

fn build_path_from_target(
    parents: &BTreeMap<String, String>,
    nodes: &BTreeMap<String, TraceCallersRecursiveNode>,
    leaf_key: &str,
) -> Vec<TraceCallersRecursivePathSegment> {
    let mut keys = ancestor_path_keys(parents, leaf_key);
    if keys.last().map(String::as_str) != Some(leaf_key) {
        keys.push(leaf_key.to_string());
    }
    keys.into_iter()
        .filter_map(|key| nodes.get(&key))
        .map(|node| TraceCallersRecursivePathSegment {
            path: node.path.clone(),
            symbol: node.symbol.clone(),
            depth: node.depth,
        })
        .collect()
}

fn ancestor_path_keys(parents: &BTreeMap<String, String>, leaf_key: &str) -> Vec<String> {
    let mut keys = vec![leaf_key.to_string()];
    let mut current = leaf_key;
    while let Some(parent) = parents.get(current) {
        keys.push(parent.clone());
        current = parent;
    }
    keys.reverse();
    keys
}

fn build_paths(
    parents: &BTreeMap<String, String>,
    nodes: &[TraceCallersRecursiveNode],
    root_key: &str,
    adjacency: &BTreeMap<String, Vec<String>>,
) -> Vec<Vec<TraceCallersRecursivePathSegment>> {
    nodes
        .iter()
        .filter(|node| node.key == root_key || !adjacency.contains_key(&node.key))
        .map(|node| {
            ancestor_path_keys(parents, &node.key)
                .into_iter()
                .filter_map(|key| nodes.iter().find(|item| item.key == key))
                .map(|item| TraceCallersRecursivePathSegment {
                    path: item.path.clone(),
                    symbol: item.symbol.clone(),
                    depth: item.depth,
                })
                .collect::<Vec<_>>()
        })
        .filter(|path| !path.is_empty())
        .collect()
}

fn node_key(path: &str, symbol: &str) -> String {
    format!("{}::{}", path, symbol)
}

fn path_contains(parents: &BTreeMap<String, String>, start_key: &str, target_key: &str) -> bool {
    if start_key == target_key {
        return true;
    }
    let mut current = start_key;
    while let Some(parent) = parents.get(current) {
        if parent == target_key {
            return true;
        }
        current = parent;
    }
    false
}
