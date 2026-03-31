mod analyzers;
mod capabilities;
mod error;
mod protocol;
mod workspace;

use std::io::{self, BufRead, Write};

use capabilities::dispatch;
use error::EngineError;
use protocol::{EngineRequest, EngineResponse};

fn main() {
    let stdin = io::stdin();
    let mut stdout = io::stdout();

    for line in stdin.lock().lines() {
        let raw_line = match line {
            Ok(value) => value,
            Err(error) => {
                let response = EngineResponse::error(
                    "transport-read-error".to_string(),
                    EngineError::backend_execution_failed(error.to_string()),
                );
                write_response(&mut stdout, &response);
                continue;
            }
        };

        if raw_line.trim().is_empty() {
            continue;
        }

        let request = match serde_json::from_str::<EngineRequest>(&raw_line) {
            Ok(request) => request,
            Err(error) => {
                let response = EngineResponse::error(
                    "invalid-request".to_string(),
                    EngineError::invalid_request(error.to_string()),
                );
                write_response(&mut stdout, &response);
                continue;
            }
        };

        let response = dispatch(request);
        write_response(&mut stdout, &response);
    }
}

fn write_response(stdout: &mut io::Stdout, response: &EngineResponse) {
    if let Ok(serialized) = serde_json::to_string(response) {
        let _ = writeln!(stdout, "{}", serialized);
        let _ = stdout.flush();
    }
}
