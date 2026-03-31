pub mod find_symbol;
pub mod inspect_tree;
pub mod list_endpoints;

use crate::error::EngineError;
use crate::protocol::EngineRequest;
use crate::protocol::EngineResponse;

pub fn dispatch(request: EngineRequest) -> EngineResponse {
    match request.capability.as_str() {
        find_symbol::CAPABILITY => find_symbol::handle(request),
        inspect_tree::CAPABILITY => inspect_tree::handle(request),
        list_endpoints::CAPABILITY => list_endpoints::handle(request),
        capability => {
            EngineResponse::error(request.id, EngineError::unsupported_capability(capability))
        }
    }
}
