//! The MCP server over stdio.
//!
//! `theos-mcp [--project <folder>]` serves the read tools of the content project to an MCP
//! client over stdin and stdout. stdout carries only the protocol; logs go to stderr, filtered
//! by `THEOS_LOG` (default `warn`).

mod server;
mod state;
mod tools;

use std::path::PathBuf;
use std::process::ExitCode;

use clap::Parser;
use rmcp::ServiceExt;
use rmcp::service::ServerInitializeError;
use rmcp::transport::stdio;
use tracing_subscriber::EnvFilter;

use crate::server::Server;
use crate::state::{Locator, State};

/// The read tools of a Theos content project, as an MCP server over stdio.
#[derive(Parser)]
#[command(version, about)]
struct Args {
    /// The project folder. It must hold `theos.toml` and is never searched upward. Without it,
    /// the nearest folder at or above the current one that holds `theos.toml` is used.
    #[arg(long)]
    project: Option<PathBuf>,
}

/// Sends the logs to stderr, filtered by `THEOS_LOG`.
fn init_logging() {
    let filter = EnvFilter::try_from_env("THEOS_LOG").unwrap_or_else(|_| EnvFilter::new("warn"));
    // Only one subscriber can be installed; if one exists already, it stays.
    tracing_subscriber::fmt()
        .with_env_filter(filter)
        .with_writer(std::io::stderr)
        .try_init()
        .ok();
}

/// Serves until the client closes stdin. A client that closes stdin before it said anything
/// is a normal end, not a failure.
async fn serve(locator: Locator) -> anyhow::Result<()> {
    let service = match Server::new(State::new(locator)).serve(stdio()).await {
        Ok(service) => service,
        Err(ServerInitializeError::ConnectionClosed(_)) => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    service.waiting().await?;
    Ok(())
}

/// Builds the runtime and serves on it.
fn run(locator: Locator) -> anyhow::Result<()> {
    tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()?
        .block_on(serve(locator))
}

fn main() -> ExitCode {
    let args = Args::parse();
    init_logging();
    let result = match args.project {
        Some(folder) => Ok(Locator::Given(folder)),
        None => std::env::current_dir().map(Locator::Search),
    }
    .map_err(anyhow::Error::from)
    .and_then(run);
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            tracing::error!("theos-mcp stopped: {error:#}");
            ExitCode::from(2)
        }
    }
}
