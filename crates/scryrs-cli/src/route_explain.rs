use std::io::Write;

use crate::route_common::{load_route_manifest, resolve_repo_root, warn_if_route_artifact_changed};

#[cfg(feature = "runtime")]
use scryrs_runtime::explain_hints;

#[cfg(feature = "runtime")]
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

    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_route_explain_help(out).map_or(1, |_| 0);
    }

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
        } else if args[i].starts_with('-') && args[i] != "--query" {
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

    let path = match path_arg {
        Some(path) => path,
        None => {
            return route_explain_usage_err(
                err,
                "scryrs route explain: missing required PATH argument",
            );
        }
    };
    let query = match query {
        Some(query) => query,
        None => {
            return route_explain_usage_err(
                err,
                "scryrs route explain: missing required --query argument",
            );
        }
    };

    let repo_root = match resolve_repo_root(err, "scryrs route explain", path) {
        Ok(repo_root) => repo_root,
        Err(exit_code) => return exit_code,
    };
    let loaded = match load_route_manifest(err, "scryrs route explain", &repo_root) {
        Ok(loaded) => loaded,
        Err(exit_code) => return exit_code,
    };

    let hint_doc = explain_hints(&loaded.manifest, query);
    let json = match serde_json::to_string(&hint_doc) {
        Ok(json) => json,
        Err(error) => {
            let _ = writeln!(err, "scryrs route explain: serialization error: {error}");
            return 1;
        }
    };

    warn_if_route_artifact_changed(
        err,
        "scryrs route explain",
        &loaded.routes_path,
        &loaded.routes_json,
    );

    if writeln!(out, "{json}").is_err() {
        return 1;
    }

    0
}

#[cfg(not(feature = "runtime"))]
pub(crate) fn execute_route_explain(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_route_explain_help(out).map_or(1, |_| 0);
    }

    let _ = writeln!(
        err,
        "scryrs route explain: unavailable (runtime feature not enabled)"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
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
    Authoritative order: (tier DESC, score DESC, count DESC, manifest_index ASC, route_id ASC)\n\
    score is the saturating sum of evidence link scores; count is evidence link count.\n\n\
OUTPUT\n\
  Single-line JSON RouteHintDocument with schemaVersion and hints array.\n\
  Each hint carries routeId, stable target node id, optional loadTarget,\n\
  label, rank, relevance, reason, and evidence. File loadTarget references are\n\
  repository-relative paths, doc_page references are project-docs/<slug>, and\n\
  search / symbol / domain_term / doc_group routes stay explicitly non_loadable.\n\
  rank remains the manifest ordinal; explain relevance is the packed score\n\
  tier * 1_000_000_000 + min(total_evidence_score, 999_999) * 1_000 + min(evidence_count, 999).\n\
  plain route projection omits relevance. The reason field includes load target\n\
  kind and appends a \"; query match on <fields>\" suffix. Zero matches\n\
  produces a valid document with an empty hints array.\n\n\
EXIT CODES\n\
  0    Success (including zero-match results)\n\
  1    Serialization or stdout write failure\n\
  2    Usage error, missing artifact, malformed artifact, schema mismatch"
    )
}
