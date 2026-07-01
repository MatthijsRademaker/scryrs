use std::io::{self, Read, Write};

use clap::{Arg, ArgAction, Command};

use crate::dashboard::{execute_dashboard, write_dashboard_help};
use crate::doctor::execute_doctor_cli;
use crate::graph::write_graph_json;
use crate::help_json::write_cli_surface;
use crate::help_text::write_help;
use crate::hook::execute_hook;
use crate::hotspots::write_hotspots_json;
use crate::init;
use crate::proposals::execute_proposals_cli;
use crate::propose::write_proposals;
use crate::publish::execute_publish_cli;
use crate::record::execute_record;
use crate::route::write_route_json;
use crate::server::{execute_server, write_server_help};
use crate::setup;
use crate::up::execute_up;

pub fn run<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    run_with_writers(args, io::stdout().lock(), io::stderr().lock())
}

pub fn run_with_writers<I, S, O, E>(args: I, out: O, err: E) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    O: Write,
    E: Write,
{
    run_with_io(args, out, err, io::stdin().lock())
}

pub fn run_with_io<I, S, O, E, R>(args: I, mut out: O, mut err: E, mut stdin: R) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    O: Write,
    E: Write,
    R: Read,
{
    let mut args: Vec<String> = args.into_iter().map(Into::into).collect();

    // D1: Pre-clap normalization: root-level -hj -> --help-json
    if args.len() == 1 && args[0] == "-hj" {
        args[0] = "--help-json".to_string();
    }

    // D5: Pre-clap --help-json handling (not a clap flag)
    if args.len() == 1 && args[0] == "--help-json" {
        return write_cli_surface(&mut out).map_or(1, |_| 0);
    }

    if args.len() == 2 && args[0] == "dashboard" && (args[1] == "--help" || args[1] == "-h") {
        return write_dashboard_help(&mut out).map_or(1, |_| 0);
    }

    if args.len() == 2 && args[0] == "server" && (args[1] == "--help" || args[1] == "-h") {
        return write_server_help(&mut out).map_or(1, |_| 0);
    }

    if !args.is_empty() && args[0] == "proposals" {
        return execute_proposals_cli(&mut out, &mut err, &args[1..]);
    }

    if !args.is_empty() && args[0] == "publish" {
        return execute_publish_cli(&mut out, &mut err, &args[1..]);
    }

    if !args.is_empty() && args[0] == "doctor" {
        return execute_doctor_cli(&mut out, &mut err, &args[1..]);
    }

    if args.len() >= 2 && args[0] == "route" && args[1] == "explain" {
        return crate::route_explain::execute_route_explain(&mut out, &mut err, &args[2..]);
    }

    // Unknown command check before clap dispatch.
    // Only known root-level entrypoints pass through to clap.
    // Everything else produces the contract error: "unknown command: 'X'".
    if !args.is_empty() {
        let first = &args[0];
        if first != "hotspots"
            && first != "record"
            && first != "hook"
            && first != "init"
            && first != "setup"
            && first != "dashboard"
            && first != "server"
            && first != "doctor"
            && first != "graph"
            && first != "route"
            && first != "propose"
            && first != "proposals"
            && first != "publish"
            && first != "up"
            && first != "--help"
            && first != "-h"
            && first != "--version"
            && first != "-V"
        {
            if writeln!(err, "unknown command: '{first}'").is_err()
                || writeln!(err, "See `scryrs --help`").is_err()
            {
                return 1;
            }
            return 2;
        }
    }

    // Capture the attempted subcommand before clap consumes args, so
    // error handlers can emit subcommand-specific messages.
    let attempted_command: Option<&str> = if !args.is_empty()
        && (args[0] == "hotspots"
            || args[0] == "record"
            || args[0] == "hook"
            || args[0] == "init"
            || args[0] == "setup"
            || args[0] == "dashboard"
            || args[0] == "server"
            || args[0] == "doctor"
            || args[0] == "graph"
            || args[0] == "route"
            || args[0] == "propose"
            || args[0] == "publish"
            || args[0] == "up")
    {
        Some(args[0].as_str())
    } else {
        None
    };

    // D2: Clap builder API with try_get_matches_from (never get_matches_from).
    // Help/version flags enabled on root (so clap triggers DisplayHelp/DisplayVersion).
    // Disabled on the hotspots subcommand so --help/--version after hotspots are
    // rejected as unknown arguments (preserving the v0 contract).
    let cmd = Command::new("scryrs")
        .no_binary_name(true)
        .subcommand_required(false)
        .version(env!("CARGO_PKG_VERSION"))
        .subcommand(
            Command::new("hotspots")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(Arg::new("PATH").required(true).value_name("PATH")),
        )
        .subcommand(
            Command::new("record")
                .about("Ingest JSONL trace events from stdin or file")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("stdin")
                        .long("stdin")
                        .num_args(0)
                        .action(ArgAction::SetTrue)
                        .help("Read JSONL events from stdin"),
                )
                .arg(
                    Arg::new("file")
                        .long("file")
                        .value_name("PATH")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Read JSONL events from PATH"),
                )
                .arg(
                    Arg::new("mode")
                        .long("mode")
                        .value_name("MODE")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Transport mode: live (default, remote ingest) or local (SQLite)"),
                ),
        )
        .subcommand(
            Command::new("hook")
                .about("Translate and record a harness's native tool event (fail-open)")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("harness")
                        .required(true)
                        .value_name("HARNESS")
                        .help("Harness name (claude-code or pi)"),
                )
                .arg(
                    Arg::new("stdin")
                        .long("stdin")
                        .num_args(0)
                        .action(ArgAction::SetTrue)
                        .conflicts_with("file")
                        .help("Read the harness event from stdin (default)"),
                )
                .arg(
                    Arg::new("file")
                        .long("file")
                        .value_name("PATH")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Read the harness event from PATH"),
                ),
        )
        .subcommand(
            Command::new("init")
                .about("Install scryrs trace hook for a supported agent harness")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("agent")
                        .long("agent")
                        .value_name("NAME")
                        .num_args(1)
                        .required(true)
                        .action(ArgAction::Set)
                        .help("Agent harness name (claude-code or pi)"),
                ),
        )
        .subcommand(
            Command::new("setup")
                .about("Configure local or live trace transport (scryrs.json remote, .scryrs/ scaffold)")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("MODE")
                        .required(true)
                        .value_name("MODE")
                        .help("Transport mode: local (SQLite store) or live (remote ingest)"),
                )
                .arg(
                    Arg::new("ingest-url")
                        .long("ingest-url")
                        .value_name("URL")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Live-mode remote ingest URL (overrides .scryrs/.env SCRYRS_REMOTE_INGEST_URL)"),
                )
                .arg(
                    Arg::new("workspace-id")
                        .long("workspace-id")
                        .value_name("ID")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Live-mode workspace identity (overrides .scryrs/.env SCRYRS_WORKSPACE_ID)"),
                )
                .arg(
                    Arg::new("agent-id")
                        .long("agent-id")
                        .value_name("ID")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Optional live-mode agent identity override (autogenerated per container when omitted; not committed)"),
                )
                .arg(
                    Arg::new("repository-id")
                        .long("repository-id")
                        .value_name("ID")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Optional live-mode repository identity override (derived from Git remote origin when omitted; not committed)"),
                )
                .arg(
                    Arg::new("docker-network")
                        .long("docker-network")
                        .value_name("NAME")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Docker network name for the --with-compose self-host opt-in (overrides .scryrs/.env SCRYRS_DOCKER_NETWORK)"),
                )
                .arg(
                    Arg::new("with-compose")
                        .long("with-compose")
                        .num_args(0)
                        .action(ArgAction::SetTrue)
                        .help("Live-mode opt-in: scaffold the self-hosted .scryrs/compose.yml stack (requires docker_network)"),
                )
                .arg(
                    Arg::new("no-interactive")
                        .long("no-interactive")
                        .num_args(0)
                        .action(ArgAction::SetTrue)
                        .help("Disable live-setup prompts; missing live config fails fast"),
                ),
        )
        .subcommand(Command::new("up").about("Start the workspace-managed live-server Compose stack").disable_help_flag(true).disable_version_flag(true))
        .subcommand(
            Command::new("dashboard")
                .about("Start dashboard server")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .value_name("PORT")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("TCP port to bind (default 8080)"),
                )
                .arg(
                    Arg::new("bind")
                        .long("bind")
                        .short('b')
                        .value_name("ADDR")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Bind address (default 127.0.0.1)"),
                )
                .arg(
                    Arg::new("mode")
                        .long("mode")
                        .value_name("MODE")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Source mode: live (default) or local"),
                )
                .arg(
                    Arg::new("server-url")
                        .long("server-url")
                        .value_name("URL")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Live-mode scryrs server base URL (overrides .scryrs/.env SCRYRS_REMOTE_INGEST_URL)"),
                )
                .arg(
                    Arg::new("repository-id")
                        .long("repository-id")
                        .value_name("ID")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Live-mode repository identity (overrides .scryrs/.env SCRYRS_REPOSITORY_ID)"),
                )
                .arg(
                    Arg::new("no-open")
                        .long("no-open")
                        .num_args(0)
                        .action(ArgAction::SetTrue)
                        .help("Do not open browser automatically"),
                )
                .arg(
                    Arg::new("dev")
                        .long("dev")
                        .num_args(0)
                        .action(ArgAction::SetTrue)
                        .help("Serve from filesystem instead of embedded assets"),
                ),
        )
        .subcommand(
            Command::new("server")
                .about("Start the central trace ingest server")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(
                    Arg::new("port")
                        .long("port")
                        .short('p')
                        .value_name("PORT")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("TCP port to bind (default 8081)"),
                )
                .arg(
                    Arg::new("bind")
                        .long("bind")
                        .short('b')
                        .value_name("ADDR")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Bind address (default 127.0.0.1)"),
                )
                .arg(
                    Arg::new("store")
                        .long("store")
                        .value_name("PATH")
                        .num_args(1)
                        .action(ArgAction::Set)
                        .help("Server-owned SQLite store path (default .scryrs/server.db)"),
                ),
        )
        .subcommand(
            Command::new("graph")
                .about("Build the repository knowledge graph")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(Arg::new("PATH").required(true).value_name("PATH")),
        )
        .subcommand(
            Command::new("route")
                .about("Generate the route manifest from a knowledge graph, or explain route entries")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .subcommand_required(false)
                .arg(Arg::new("PATH").required(false).value_name("PATH"))
                .subcommand(
                    Command::new("explain")
                        .about("Query the route manifest for matching entries")
                        .disable_help_flag(true)
                        .disable_version_flag(true)
                        .arg(Arg::new("PATH").value_name("PATH"))
                        .arg(
                            Arg::new("query")
                                .long("query")
                                .value_name("TEXT")
                                .num_args(1)
                                .action(clap::ArgAction::Set)
                                .help("Query text for case-insensitive substring matching"),
                        ),
                ),
        )
        .subcommand(
            Command::new("propose")
                .about("Generate reviewable knowledge proposals from hotspot and graph evidence")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .arg(Arg::new("PATH").required(true).value_name("PATH")),
        )
        .subcommand(
            Command::new("publish")
                .about("Publish accepted knowledge explicitly through markdown or Rspress surfaces")
                .disable_help_flag(true)
                .disable_version_flag(true)
                .subcommand_required(false)
                .subcommand(
                    Command::new("markdown")
                        .disable_help_flag(true)
                        .disable_version_flag(true)
                        .arg(Arg::new("PATH").required(true).value_name("PATH"))
                        .arg(
                            Arg::new("output")
                                .long("output")
                                .value_name("DIR")
                                .required(true)
                                .num_args(1)
                                .action(ArgAction::Set),
                        ),
                )
                .subcommand(
                    Command::new("rspress")
                        .disable_help_flag(true)
                        .disable_version_flag(true)
                        .arg(Arg::new("PATH").required(true).value_name("PATH"))
                        .arg(
                            Arg::new("docs-root")
                                .long("docs-root")
                                .value_name("DIR")
                                .required(true)
                                .num_args(1)
                                .action(ArgAction::Set),
                        ),
                ),
        );

    match cmd.try_get_matches_from(&args) {
        Ok(matches) => {
            match matches.subcommand() {
                Some(("hotspots", m)) => {
                    let path = m
                        .get_one::<String>("PATH")
                        .map(|s| s.as_str())
                        .unwrap_or(".");
                    write_hotspots_json(&mut out, &mut err, path)
                }
                Some(("record", m)) => execute_record(&mut out, &mut err, &mut stdin, m),
                Some(("hook", m)) => {
                    let harness = m
                        .get_one::<String>("harness")
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    execute_hook(&mut out, &mut err, &mut stdin, harness, m)
                }
                Some(("init", m)) => {
                    let agent = m
                        .get_one::<String>("agent")
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    if agent.is_empty() {
                        if writeln!(err, "scryrs init: --agent requires a non-empty value").is_err()
                            || writeln!(err, "Usage: scryrs init --agent <NAME>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    } else {
                        init::execute_init(&mut out, &mut err, agent)
                    }
                }
                Some(("setup", m)) => {
                    let mode_str = m
                        .get_one::<String>("MODE")
                        .map(|s| s.as_str())
                        .unwrap_or("");
                    let mode = match mode_str {
                        "local" => Some(setup::SetupMode::Local),
                        "live" => Some(setup::SetupMode::Live),
                        _ => None,
                    };
                    match mode {
                        Some(mode) => {
                            let ingest_url = m.get_one::<String>("ingest-url").map(|s| s.as_str()).unwrap_or("");
                            let workspace_id = m.get_one::<String>("workspace-id").map(|s| s.as_str()).unwrap_or("");
                            let agent_id = m.get_one::<String>("agent-id").map(|s| s.as_str()).unwrap_or("");
                            let repository_id = m.get_one::<String>("repository-id").map(|s| s.as_str());
                            let docker_network = m.get_one::<String>("docker-network").map(|s| s.as_str()).unwrap_or("");
                            let with_compose = m.get_flag("with-compose");
                            let no_interactive = m.get_flag("no-interactive");
                            setup::execute_setup(
                                &mut out,
                                &mut err,
                                mode,
                                ingest_url,
                                workspace_id,
                                repository_id,
                                agent_id,
                                docker_network,
                                with_compose,
                                no_interactive,
                            )
                        }
                        None => {
                            setup::write_unknown_mode(&mut err, mode_str);
                            2
                        }
                    }
                }
                Some(("up", _)) => execute_up(&mut out, &mut err),
                Some(("dashboard", m)) => execute_dashboard(&mut err, m),
                Some(("server", m)) => execute_server(&mut err, m),
                Some(("graph", m)) => {
                    let path = m
                        .get_one::<String>("PATH")
                        .map(|s| s.as_str())
                        .unwrap_or(".");
                    write_graph_json(&mut out, &mut err, path)
                }
                Some(("route", m)) => {
                    // route explain is intercepted pre-clap, so we only get here
                    // for bare `scryrs route <PATH>`.
                    match m.subcommand() {
                        Some(("explain", _)) => {
                            // Should not be reached — pre-clap intercept catches this.
                            // If it does, treat as usage error.
                            if writeln!(err, "scryrs route explain: internal dispatch error").is_err()
                                || writeln!(err, "See `scryrs --help`").is_err()
                            {
                                1
                            } else {
                                2
                            }
                        }
                        _ => {
                            match m.get_one::<String>("PATH") {
                                Some(path) => write_route_json(&mut out, &mut err, path),
                                None => {
                                    if writeln!(err, "scryrs route: missing required PATH argument").is_err()
                                        || writeln!(err, "Usage: scryrs route <PATH>").is_err()
                                        || writeln!(err, "See `scryrs --help`").is_err()
                                    {
                                        1
                                    } else {
                                        2
                                    }
                                }
                            }
                        }
                    }
                }
                Some(("propose", m)) => {
                    let path = m
                        .get_one::<String>("PATH")
                        .map(|s| s.as_str())
                        .unwrap_or(".");
                    write_proposals(&mut out, &mut err, path)
                }
                // Bare invocation (no subcommand matched).
                _ => write_help(&mut out).map_or(1, |_| 0),
            }
        }
        Err(e) => match e.kind() {
            // D3/D4: Help and version — route to existing contract functions.
            clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::MissingSubcommand => {
                write_help(&mut out).map_or(1, |_| 0)
            }
            clap::error::ErrorKind::DisplayVersion => {
                writeln!(out, "scryrs {}", env!("CARGO_PKG_VERSION")).map_or(1, |_| 0)
            }
            // D4: Usage errors -> exit 2 with contract three-line format.
            clap::error::ErrorKind::MissingRequiredArgument => match attempted_command {
                Some("hook") => {
                    if writeln!(err, "scryrs hook: missing required HARNESS argument").is_err()
                        || writeln!(err, "Usage: scryrs hook <HARNESS> [--stdin | --file <PATH>]")
                            .is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                Some("init") => {
                    if writeln!(err, "scryrs init: missing required --agent argument").is_err()
                        || writeln!(err, "Usage: scryrs init --agent <NAME>").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                Some("setup") => {
                    setup::write_missing_mode(&mut err);
                    2
                }
                Some("dashboard") => {
                    if writeln!(err, "scryrs dashboard: missing required argument").is_err()
                        || writeln!(err, "Usage: scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--server-url <URL> --repository-id <ID>] [--no-open] [--dev]").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                Some("server") => {
                    if writeln!(err, "scryrs server: missing required argument").is_err()
                        || writeln!(err, "Usage: scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                Some("graph") => {
                    if writeln!(err, "scryrs graph: missing required PATH argument").is_err()
                        || writeln!(err, "Usage: scryrs graph <PATH>").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                Some("route") => {
                    if writeln!(err, "scryrs route: missing required PATH argument").is_err()
                        || writeln!(err, "Usage: scryrs route <PATH>").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                Some("propose") => {
                    if writeln!(err, "scryrs propose: missing required PATH argument").is_err()
                        || writeln!(err, "Usage: scryrs propose <PATH>").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
                _ => {
                    if writeln!(err, "scryrs hotspots: missing required PATH argument").is_err()
                        || writeln!(err, "Usage: scryrs hotspots <PATH>").is_err()
                        || writeln!(err, "See `scryrs --help`").is_err()
                    {
                        1
                    } else {
                        2
                    }
                }
            },
            clap::error::ErrorKind::TooManyValues | clap::error::ErrorKind::UnknownArgument => {
                match attempted_command {
                    Some("hook") => {
                        if writeln!(err, "scryrs hook: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs hook <HARNESS> [--stdin | --file <PATH>]")
                                .is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("up") => {
                        if writeln!(err, "scryrs up: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs up").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("dashboard") => {
                        if writeln!(err, "scryrs dashboard: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs dashboard [--port <PORT>] [--bind <ADDR>] [--server-url <URL> --repository-id <ID>] [--no-open] [--dev]").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("server") => {
                        if writeln!(err, "scryrs server: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs server [--bind <ADDR>] [--port <PORT>] [--store <PATH>]").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("record") => {
                        if writeln!(err, "scryrs record: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs record --stdin").is_err()
                            || writeln!(err, "Usage: scryrs record --file <PATH>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("graph") => {
                        if writeln!(err, "scryrs graph: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs graph <PATH>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("route") => {
                        if writeln!(err, "scryrs route: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs route <PATH>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("propose") => {
                        if writeln!(err, "scryrs propose: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs propose <PATH>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("init") => {
                        if writeln!(err, "scryrs init: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs init --agent <NAME>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    Some("setup") => {
                        if writeln!(err, "scryrs setup: unexpected argument").is_err()
                            || writeln!(err, "Usage: scryrs setup <local|live>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                    _ => {
                        if writeln!(err, "scryrs hotspots: unexpected argument after PATH").is_err()
                            || writeln!(err, "Usage: scryrs hotspots <PATH>").is_err()
                            || writeln!(err, "See `scryrs --help`").is_err()
                        {
                            1
                        } else {
                            2
                        }
                    }
                }
            }
            // Unrecognized clap error -> exit 1.
            _ => 1,
        },
    }
}
