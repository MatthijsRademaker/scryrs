use std::io::Write;
use std::net::IpAddr;
use std::path::PathBuf;

use clap::ArgMatches;

pub(crate) fn write_server_help(out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "scryrs server — start the central trace ingest server\n\n\
Starts a long-lived HTTP server for trace event ingest with\n\
read-only live hotspot query and signal streaming endpoints.\n\
Accepts versioned trace-event batches, validates them deterministically,\n\
and persists accepted events into a server-owned SQLite store with\n\
first-writer-wins idempotency.\n\n\
USAGE\n\
  scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]\n\n\
FLAGS\n\
  -b, --bind <ADDR>    Bind address (default 127.0.0.1)\n\
  -p, --port <PORT>    TCP port to bind (default 8081)\n\
      --store <PATH>   Server-owned SQLite store path (default .scryrs/server.db)\n\n\
ENDPOINTS\n\
  POST /v1/trace-events/batch\n\
      Accepts JSON ServerIngestEnvelope, returns JSON BatchIngestResponse\n\
      with deterministic accepted_count, duplicate_count, rejected_count,\n\
      received_count, and per-item diagnostics.\n\
  GET /v1/repositories/{{repository_id}}/hotspots\n\
      Query live hotspot rankings from server-owned state.\n\
      Supports ?window=cumulative and optional ?session_id.\n\
      Returns JSON LiveHotspotsResponse with ranked HotspotEntry items.\n\
  GET /v1/repositories/{{repository_id}}/signals\n\
      Server-Sent Events stream of HotspotSignal records.\n\
      Supports ?after=<signal_id> for cursor-based replay/resume.\n"
    )
}

#[cfg(feature = "server")]
pub(crate) fn execute_server(err: &mut impl Write, m: &ArgMatches) -> i32 {
    let port = match parse_port(m.get_one::<String>("port")) {
        Ok(port) => port,
        Err(message) => {
            let _ = writeln!(err, "scryrs server: {message}");
            let _ = writeln!(
                err,
                "Usage: scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]"
            );
            return 2;
        }
    };
    let bind_address = match parse_bind(m.get_one::<String>("bind")) {
        Ok(bind) => bind,
        Err(message) => {
            let _ = writeln!(err, "scryrs server: {message}");
            let _ = writeln!(
                err,
                "Usage: scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]"
            );
            return 2;
        }
    };
    let store_path = match parse_store(m.get_one::<String>("store")) {
        Ok(path) => path,
        Err(message) => {
            let _ = writeln!(err, "scryrs server: {message}");
            let _ = writeln!(
                err,
                "Usage: scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]"
            );
            return 2;
        }
    };

    let config = match scryrs_server::Config::try_new(
        port,
        bind_address,
        store_path,
        scryrs_server::DEFAULT_SIGNAL_THRESHOLD,
    ) {
        Ok(config) => config,
        Err(error) => {
            let _ = writeln!(err, "scryrs server: {error}");
            return 2;
        }
    };

    match scryrs_server::run(config) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(err, "scryrs server: {error}");
            1
        }
    }
}

#[cfg(not(feature = "server"))]
pub(crate) fn execute_server(err: &mut impl Write, _m: &ArgMatches) -> i32 {
    let _ = writeln!(
        err,
        "scryrs server: unavailable (server feature not enabled)"
    );
    2
}

fn parse_port(raw: Option<&String>) -> Result<u16, String> {
    match raw {
        Some(value) => value
            .parse::<u16>()
            .map_err(|error| format!("invalid --port value '{value}': {error}"))
            .and_then(|port| {
                if port == 0 {
                    Err("invalid --port value '0': port must be between 1 and 65535".into())
                } else {
                    Ok(port)
                }
            }),
        None => Ok(8081),
    }
}

fn parse_bind(raw: Option<&String>) -> Result<IpAddr, String> {
    raw.map_or_else(
        || Ok(IpAddr::V4(std::net::Ipv4Addr::LOCALHOST)),
        |value| {
            value
                .parse::<IpAddr>()
                .map_err(|error| format!("invalid --bind value '{value}': {error}"))
        },
    )
}

fn parse_store(raw: Option<&String>) -> Result<PathBuf, String> {
    match raw {
        Some(value) => {
            let path = PathBuf::from(value);
            if path.as_os_str().is_empty() {
                Err("invalid --store value: path must not be empty".into())
            } else {
                Ok(path)
            }
        }
        None => Ok(PathBuf::from(".scryrs/server.db")),
    }
}
