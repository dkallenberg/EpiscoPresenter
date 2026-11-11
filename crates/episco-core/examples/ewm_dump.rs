//! Parses an `.ewm` file or a service plan (`.toml`) and prints the result as
//! JSON, with warnings on stderr. Handy for checking files from the terminal:
//!
//!     cargo run -p episco-core --example ewm_dump -- examples/liturgies/sursum-corda.ewm
//!     cargo run -p episco-core --example ewm_dump -- examples/service.toml

use std::process::ExitCode;

use episco_core::{ewm, Service};

fn main() -> ExitCode {
    let Some(path) = std::env::args().nth(1) else {
        eprintln!("usage: ewm_dump <file.ewm | plan.toml>");
        return ExitCode::from(2);
    };
    let result = if path.ends_with(".toml") {
        Service::load_plan(&path)
            .map(|l| (serde_json::to_string_pretty(&l.service), l.warnings))
            .map_err(|e| e.to_string())
    } else {
        ewm::load_file(&path)
            .map(|p| (serde_json::to_string_pretty(&p.document), p.warnings.iter().map(ToString::to_string).collect()))
            .map_err(|e| e.to_string())
    };
    match result {
        Ok((json, warnings)) => {
            for w in warnings {
                eprintln!("warning: {w}");
            }
            println!("{}", json.expect("documents always serialise"));
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("error: {e}");
            ExitCode::FAILURE
        }
    }
}
