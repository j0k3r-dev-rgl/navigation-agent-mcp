pub mod find_symbol;
pub mod inspect_tree;
pub mod list_endpoints;
pub mod search_text;
pub mod trace_callers;
pub mod trace_symbol;

use crate::error::EngineError;
use crate::protocol::EngineRequest;
use crate::protocol::EngineResponse;

pub fn dispatch(request: EngineRequest) -> EngineResponse {
    match request.capability.as_str() {
        find_symbol::CAPABILITY => find_symbol::handle(request),
        inspect_tree::CAPABILITY => inspect_tree::handle(request),
        list_endpoints::CAPABILITY => list_endpoints::handle(request),
        search_text::CAPABILITY => search_text::handle(request),
        trace_callers::CAPABILITY => trace_callers::handle(request),
        trace_symbol::CAPABILITY => trace_symbol::handle(request),
        capability => {
            EngineResponse::error(request.id, EngineError::unsupported_capability(capability))
        }
    }
}
