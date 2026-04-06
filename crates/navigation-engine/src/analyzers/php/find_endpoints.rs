use std::path::Path;

use super::super::types::{EndpointDefinition, FindEndpointsQuery};

pub(super) fn find_endpoints(
    _path: &Path,
    _source: &str,
    _query: &FindEndpointsQuery,
) -> Vec<EndpointDefinition> {
    // Stub implementation - framework-agnostic for now
    // In the future, this could detect Laravel routes, Symfony attributes, etc.
    Vec::new()
}
