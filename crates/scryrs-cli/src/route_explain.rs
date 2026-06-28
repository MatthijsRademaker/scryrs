use std::io::Write;
use std::path::Path;

use scryrs_runtime::explain_hints;
use scryrs_types::{ROUTE_SCHEMA_VERSION, RouteManifestDocument};

pub(crate) fn execute_route_explain(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.is_empty() {
        return route_explain_usage_err(
            err,
            "scryrs route explain: missing required PATH argument",
        );
    }

    // --help / -h
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_route_explain_help(out).map_or(1, |_| 0);
    }

    // Parse PATH and --query.
    let mut path_arg: Option<&str> = None;
    let mut query: Option<&str> = None;
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--query" {
            if i + 1 < args.len() {
                query = Some(args[i + 1].as_str());
                i += 2;
            } else {
                return route_explain_usage_err(
                    err,
                    "scryrs route explain: --query requires a value",
                );
            }
        } else if args[i].starts_with("-") && args[i] != "--query" {
            return route_explain_usage_err(
                err,
                &format!("scryrs route explain: unexpected argument '{}'", args[i]),
            );
        } else if path_arg.is_none() {
            path_arg = Some(args[i].as_str());
            i += 1;
        } else {
            return route_explain_usage_err(err, "scryrs route explain: unexpected extra argument");
        }
    }

    // Missing PATH.
    let path = match path_arg {
        Some(p) => p,
        None => {
            return route_explain_usage_err(
                err,
                "scryrs route explain: missing required PATH argument",
            );
        }
    };

    // Missing --query.
    let query = match query {
        Some(q) => q,
        None => {
            return route_explain_usage_err(
                err,
                "scryrs route explain: missing required --query argument",
            );
        }
    };

    // Resolve PATH to absolute repo root.
    let repo_root = match std::path::absolute(path) {
        Ok(p) => p,
        Err(e) => {
            let _ = writeln!(
                err,
                "scryrs route explain: cannot resolve path '{path}': {e}"
            );
            return 2;
        }
    };

    // Load .scryrs/routes.json (required).
    let routes_path = repo_root.join(".scryrs/routes.json");
    let routes_json = match std::fs::read_to_string(&routes_path) {
        Ok(s) => s,
        Err(_) => {
            let _ = writeln!(
                err,
                "scryrs route explain: route artifact not found at {}",
                routes_path.display()
            );
            let _ = writeln!(
                err,
                "Run `scryrs route <PATH>` first to generate the route manifest."
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
    };

    let manifest: RouteManifestDocument = match serde_json::from_str(&routes_json) {
        Ok(d) => d,
        Err(e) => {
            let _ = writeln!(err, "scryrs route explain: malformed route artifact: {e}");
            let _ = writeln!(
                err,
                "Run `scryrs route <PATH>` to regenerate the route manifest."
            );
            let _ = writeln!(err, "See `scryrs --help`");
            return 2;
        }
    };

    // Validate schema version.
    if manifest.schema_version != ROUTE_SCHEMA_VERSION {
        let _ = writeln!(
            err,
            "scryrs route explain: route schema version mismatch: got '{}', expected '{}'",
            manifest.schema_version, ROUTE_SCHEMA_VERSION
        );
        let _ = writeln!(
            err,
            "Run `scryrs route <PATH>` to regenerate the route manifest."
        );
        let _ = writeln!(err, "See `scryrs --help`");
        return 2;
    }

    // Verify explain didn't change the artifact on disk.
    // Read routes.json again to confirm it's byte-identical.
    let _ = check_artifact_unchanged(&routes_path, &routes_json, err);

    // Call explain_hints and serialize.
    let hint_doc = explain_hints(&manifest, query);
    let json = match serde_json::to_string(&hint_doc) {
        Ok(j) => j,
        Err(e) => {
            let _ = writeln!(err, "scryrs route explain: serialization error: {e}");
            return 1;
        }
    };

    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    0
}

fn check_artifact_unchanged(
    routes_path: &Path,
    original: &str,
    err: &mut impl Write,
) -> std::io::Result<()> {
    std::fs::read_to_string(routes_path).map(|after| {
        if after != original {
            let _ = writeln!(
                err,
                "scryrs route explain: warning: route artifact was modified during execution"
            );
        }
    })
}

fn route_explain_usage_err(err: &mut impl Write, msg: &str) -> i32 {
    let _ = writeln!(err, "{msg}");
    let _ = writeln!(err, "Usage: scryrs route explain <PATH> --query <TEXT>");
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

fn write_route_explain_help(out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "scryrs route explain — query route manifest for matching entries\n\n\
USAGE\n\
  scryrs route explain <PATH> --query <TEXT>\n\n\
DESCRIPTION\n\
  Reads the route manifest artifact (.scryrs/routes.json) and returns\n\
  deterministic, evidence-backed route recommendations matching the query.\n\
  No model, randomness, or graph inspection — purely manifest-driven.\n\n\
EXAMPLES\n\
  scryrs route explain . --query \"authentication\"\n\n\
MATCHING\n\
  Case-insensitive substring match against these fields:\n\
    label, subject, id, target, kind, evidence_links[].subject\n\n\
  Matches are tiered:\n\
    Exact match (tier 3) > prefix match (tier 2) > substring match (tier 1)\n\
    Within a tier, entries follow manifest order (by id ascending).\n\n\
OUTPUT\n\
  Single-line JSON RouteHintDocument with schemaVersion and hints array.\n\
  Each hint carries routeId, target, label, rank, reason, and evidence.\n\
  The reason field includes a \"; query match on <fields>\" suffix.\n\
  Zero matches produces a valid document with an empty hints array.\n\n\
EXIT CODES\n\
  0    Success (including zero-match results)\n\
  1    Serialization or stdout write failure\n\
  2    Usage error, missing artifact, malformed artifact, schema mismatch"
    )
}
