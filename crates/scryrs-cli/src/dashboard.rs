use std::io::Write;
use std::net::IpAddr;

use clap::ArgMatches;

pub(crate) fn write_dashboard_help(out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "scryrs dashboard — start local dashboard server\n\n\
Starts a local HTTP server for browsing .scryrs hotspot, session, and event data.\n\n\
USAGE\n\
  scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--no-open] [--dev]\n\n\
FLAGS\n\
  -p, --port <PORT>   TCP port to bind (default 8080)\n\
  -b, --bind <ADDR>   Bind address (default 127.0.0.1)\n\
      --no-open       Do not open browser automatically\n\
      --dev           Serve SPA from crates/scryrs-dashboard/frontend/dist/ instead of embedded assets\n\n\
REST API\n\
  GET /api/hotspots\n\
  GET /api/sessions\n\
  GET /api/sessions/:sessionId\n\
  GET /api/events\n"
    )
}

#[cfg(feature = "dashboard")]
pub(crate) fn execute_dashboard(err: &mut impl Write, m: &ArgMatches) -> i32 {
    let port = match parse_port(m.get_one::<String>("port")) {
        Ok(port) => port,
        Err(message) => {
            let _ = writeln!(err, "scryrs dashboard: {message}");
            let _ = writeln!(
                err,
                "Usage: scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--no-open] [--dev]"
            );
            return 2;
        }
    };
    let bind_address = match parse_bind(m.get_one::<String>("bind")) {
        Ok(bind) => bind,
        Err(message) => {
            let _ = writeln!(err, "scryrs dashboard: {message}");
            let _ = writeln!(
                err,
                "Usage: scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--no-open] [--dev]"
            );
            return 2;
        }
    };
    let repo_root = match std::env::current_dir() {
        Ok(path) => path,
        Err(error) => {
            let _ = writeln!(
                err,
                "scryrs dashboard: cannot determine current directory: {error}"
            );
            return 1;
        }
    };
    let config = match scryrs_dashboard::Config::try_new(
        port,
        bind_address,
        m.get_flag("no-open"),
        m.get_flag("dev"),
        repo_root,
    ) {
        Ok(config) => config,
        Err(error) => {
            let _ = writeln!(err, "scryrs dashboard: {error}");
            return 2;
        }
    };

    match scryrs_dashboard::run(config) {
        Ok(()) => 0,
        Err(error) => {
            let _ = writeln!(err, "scryrs dashboard: {error}");
            1
        }
    }
}

#[cfg(not(feature = "dashboard"))]
pub(crate) fn execute_dashboard(err: &mut impl Write, _m: &ArgMatches) -> i32 {
    let _ = writeln!(
        err,
        "scryrs dashboard: unavailable (dashboard feature not enabled)"
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
        None => Ok(8080),
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
