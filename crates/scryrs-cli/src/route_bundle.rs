use std::io::Write;

use crate::route_common::{load_route_manifest, resolve_repo_root, warn_if_route_artifact_changed};

#[cfg(feature = "runtime")]
use scryrs_runtime::explain_hints;
#[cfg(feature = "runtime")]
use scryrs_types::{BUNDLE_SCHEMA_VERSION, RouteBundleDocument};

#[cfg(feature = "runtime")]
pub(crate) fn execute_route_bundle(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.is_empty() {
        return route_bundle_usage_err(err, "scryrs route bundle: missing required PATH argument");
    }

    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_route_bundle_help(out).map_or(1, |_| 0);
    }

    let mut path_arg: Option<&str> = None;
    let mut query: Option<&str> = None;
    let mut limit: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--query" {
            if i + 1 < args.len() {
                query = Some(args[i + 1].as_str());
                i += 2;
            } else {
                return route_bundle_usage_err(
                    err,
                    "scryrs route bundle: --query requires a value",
                );
            }
        } else if args[i] == "--limit" {
            if i + 1 < args.len() {
                limit = Some(args[i + 1].as_str());
                i += 2;
            } else {
                return route_bundle_usage_err(
                    err,
                    "scryrs route bundle: --limit requires a value",
                );
            }
        } else if args[i].starts_with('-') && args[i] != "--query" && args[i] != "--limit" {
            return route_bundle_usage_err(
                err,
                &format!("scryrs route bundle: unexpected argument '{}'", args[i]),
            );
        } else if path_arg.is_none() {
            path_arg = Some(args[i].as_str());
            i += 1;
        } else {
            return route_bundle_usage_err(err, "scryrs route bundle: unexpected extra argument");
        }
    }

    let path = match path_arg {
        Some(path) => path,
        None => {
            return route_bundle_usage_err(
                err,
                "scryrs route bundle: missing required PATH argument",
            );
        }
    };
    let query = match query {
        Some(query) => query,
        None => {
            return route_bundle_usage_err(
                err,
                "scryrs route bundle: missing required --query argument",
            );
        }
    };
    let limit = match limit {
        Some(limit) => limit,
        None => {
            return route_bundle_usage_err(
                err,
                "scryrs route bundle: missing required --limit argument",
            );
        }
    };
    let limit = match limit.parse::<u32>() {
        Ok(0) => {
            return route_bundle_usage_err(err, "scryrs route bundle: --limit must be positive");
        }
        Ok(limit) => limit,
        Err(_) => {
            return route_bundle_usage_err(
                err,
                "scryrs route bundle: --limit must be a positive integer",
            );
        }
    };

    let repo_root = match resolve_repo_root(err, "scryrs route bundle", path) {
        Ok(repo_root) => repo_root,
        Err(exit_code) => return exit_code,
    };
    let loaded = match load_route_manifest(err, "scryrs route bundle", &repo_root) {
        Ok(loaded) => loaded,
        Err(exit_code) => return exit_code,
    };

    let explained = explain_hints(&loaded.manifest, query);
    let bundle = RouteBundleDocument {
        schema_version: BUNDLE_SCHEMA_VERSION.into(),
        query: query.into(),
        limit,
        targets: explained.hints.into_iter().take(limit as usize).collect(),
    };
    let json = match serde_json::to_string(&bundle) {
        Ok(json) => json,
        Err(error) => {
            let _ = writeln!(err, "scryrs route bundle: serialization error: {error}");
            return 1;
        }
    };

    warn_if_route_artifact_changed(
        err,
        "scryrs route bundle",
        &loaded.routes_path,
        &loaded.routes_json,
    );

    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    0
}

#[cfg(not(feature = "runtime"))]
pub(crate) fn execute_route_bundle(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_route_bundle_help(out).map_or(1, |_| 0);
    }

    let _ = writeln!(
        err,
        "scryrs route bundle: unavailable (runtime feature not enabled)"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

fn route_bundle_usage_err(err: &mut impl Write, msg: &str) -> i32 {
    let _ = writeln!(err, "{msg}");
    let _ = writeln!(
        err,
        "Usage: scryrs route bundle <PATH> --query <TEXT> --limit <N>"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

fn write_route_bundle_help(out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "scryrs route bundle — emit a bounded context-loading plan from route hints\n\n\
USAGE\n\
  scryrs route bundle <PATH> --query <TEXT> --limit <N>\n\n\
DESCRIPTION\n\
  Reads only the route manifest artifact (.scryrs/routes.json), reuses the\n\
  exact deterministic explain ranking, then truncates to the requested positive\n\
  limit. The result is a bounded context-loading plan, not an automatic load.\n\
\n\
EXAMPLES\n\
  scryrs route bundle . --query \"authentication\" --limit 5\n\n\
OUTPUT\n\
  Single-line JSON RouteBundleDocument with schemaVersion, query, limit, and\n\
  targets array. Each target reuses the explain-derived RouteHintItem fields:\n\
  routeId, target, optional loadTarget, label, rank, relevance, reason, and\n\
  evidence. Non-loadable targets remain explicit and count toward the limit.\n\
  bundle reuses the explain ordering before truncation and never applies a\n\
  second ranking algorithm. Zero matches produces a valid document with an\n\
  empty targets array.\n\n\
BUNDLE VERSUS EXPLAIN\n\
  bundle is the bounded context-loading plan for agents that need a small\n\
  explainable set of targets. explain remains the unbounded diagnostic ranking\n\
  surface for inspecting all matches.\n\n\
EXIT CODES\n\
  0    Success (including zero-match results)\n\
  1    Serialization or stdout write failure\n\
  2    Usage error, missing artifact, malformed artifact, schema mismatch, or\n\
       non-positive/invalid --limit"
    )
}
