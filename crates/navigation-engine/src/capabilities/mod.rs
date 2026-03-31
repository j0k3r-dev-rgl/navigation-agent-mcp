pub mod find_symbol;
pub mod inspect_tree;

use crate::error::EngineError;
use crate::protocol::EngineResponse;
use crate::protocol::EngineRequest;

pub fn dispatch(request: EngineRequest) -> EngineResponse {
    match request.capability.as_str() {
        find_symbol::CAPABILITY => find_symbol::handle(request),
        inspect_tree::CAPABILITY => inspect_tree::handle(request),
        capability => EngineResponse::error(
            request.id,
            EngineError::unsupported_capability(capability),
        ),
    }
}
