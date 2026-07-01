use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[cfg(feature = "markdown")]
use scryrs_adapter_markdown::publish_accepted_markdown;
#[cfg(feature = "rspress")]
use scryrs_adapter_rspress::publish_accepted_rspress;
use scryrs_types::SCHEMA_VERSION;
use serde::Serialize;

pub(crate) fn execute_publish_cli(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.is_empty() {
        return write_usage_error(
            err,
            "scryrs publish: missing required subcommand",
            &["scryrs publish --help"],
        );
    }

    match args[0].as_str() {
        "--help" | "-h" => write_publish_help(out).map_or(1, |_| 0),
        "markdown" => execute_markdown_cli(out, err, &args[1..]),
        "rspress" => execute_rspress_cli(out, err, &args[1..]),
        other => write_usage_error(
            err,
            &format!("scryrs publish: unknown subcommand '{other}'"),
            &["scryrs publish --help"],
        ),
    }
}

pub(crate) fn write_publish_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs publish — publish accepted knowledge explicitly\n\n\
USAGE\n\
  scryrs publish markdown <PATH> --output <DIR>\n\
  scryrs publish rspress <PATH> --docs-root <DIR>\n\n\
SUBCOMMANDS\n\
  markdown\n\
      Publish accepted Markdown-backed review decisions to plain generic Markdown.\n\
  rspress\n\
      Publish accepted Markdown-backed review decisions into an Rspress docs tree.\n\n\
NOTES\n\
  Publishing reads .scryrs/accepted/ only. Pending and rejected artifacts never publish.\n\
  Publishing is explicit: `scryrs proposals accept` writes review evidence only and does not publish.\n\n\
EXIT CODES\n\
  0    Success\n\
  1    Runtime or filesystem failure\n\
  2    Usage error or publish-input validation failure"
    )
}

#[cfg(feature = "markdown")]
fn execute_markdown_cli(out: &mut impl Write, err: &mut impl Write, args: &[String]) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_markdown_help(out).map_or(1, |_| 0);
    }

    let usage = "scryrs publish markdown <PATH> --output <DIR>";
    let mut path: Option<&str> = None;
    let mut output: Option<&str> = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--output" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return write_usage_error(
                        err,
                        "scryrs publish markdown: missing value for --output",
                        &[usage],
                    );
                };
                if output.is_some() {
                    return write_usage_error(
                        err,
                        "scryrs publish markdown: duplicate --output argument",
                        &[usage],
                    );
                }
                output = Some(value.as_str());
            }
            token if token.starts_with('-') => {
                return write_usage_error(
                    err,
                    &format!("scryrs publish markdown: unexpected argument '{token}'"),
                    &[usage],
                );
            }
            token => {
                if path.is_some() {
                    return write_usage_error(
                        err,
                        "scryrs publish markdown: unexpected argument after PATH",
                        &[usage],
                    );
                }
                path = Some(token);
            }
        }
        index += 1;
    }

    let Some(path) = path else {
        return write_usage_error(
            err,
            "scryrs publish markdown: missing required PATH argument",
            &[usage],
        );
    };
    let Some(output) = output else {
        return write_usage_error(
            err,
            "scryrs publish markdown: missing required --output argument",
            &[usage],
        );
    };

    let repository_root = match resolve_absolute(path, "scryrs publish markdown", err) {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    let output_root = match resolve_absolute(output, "scryrs publish markdown", err) {
        Ok(path) => path,
        Err(exit) => return exit,
    };

    match publish_accepted_markdown(&repository_root, &output_root) {
        Ok(paths) => {
            let summary = MarkdownPublishSummary {
                command: "publish",
                mode: "markdown",
                schema_version: SCHEMA_VERSION,
                count: paths.len(),
                paths: paths
                    .iter()
                    .map(|path| path.display().to_string())
                    .collect(),
            };
            write_json_summary(out, err, &summary)
        }
        Err(error) => write_publish_error(err, "scryrs publish markdown", &error.to_string()),
    }
}

#[cfg(not(feature = "markdown"))]
fn execute_markdown_cli(_out: &mut impl Write, err: &mut impl Write, _args: &[String]) -> i32 {
    let _ = writeln!(
        err,
        "scryrs publish markdown: unavailable (markdown feature not enabled)"
    );
    2
}

#[cfg(feature = "rspress")]
fn execute_rspress_cli(out: &mut impl Write, err: &mut impl Write, args: &[String]) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_rspress_help(out).map_or(1, |_| 0);
    }

    let usage = "scryrs publish rspress <PATH> --docs-root <DIR>";
    let mut path: Option<&str> = None;
    let mut docs_root: Option<&str> = None;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--docs-root" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return write_usage_error(
                        err,
                        "scryrs publish rspress: missing value for --docs-root",
                        &[usage],
                    );
                };
                if docs_root.is_some() {
                    return write_usage_error(
                        err,
                        "scryrs publish rspress: duplicate --docs-root argument",
                        &[usage],
                    );
                }
                docs_root = Some(value.as_str());
            }
            token if token.starts_with('-') => {
                return write_usage_error(
                    err,
                    &format!("scryrs publish rspress: unexpected argument '{token}'"),
                    &[usage],
                );
            }
            token => {
                if path.is_some() {
                    return write_usage_error(
                        err,
                        "scryrs publish rspress: unexpected argument after PATH",
                        &[usage],
                    );
                }
                path = Some(token);
            }
        }
        index += 1;
    }

    let Some(path) = path else {
        return write_usage_error(
            err,
            "scryrs publish rspress: missing required PATH argument",
            &[usage],
        );
    };
    let Some(docs_root) = docs_root else {
        return write_usage_error(
            err,
            "scryrs publish rspress: missing required --docs-root argument",
            &[usage],
        );
    };

    let repository_root = match resolve_absolute(path, "scryrs publish rspress", err) {
        Ok(path) => path,
        Err(exit) => return exit,
    };
    let docs_root = match resolve_absolute(docs_root, "scryrs publish rspress", err) {
        Ok(path) => path,
        Err(exit) => return exit,
    };

    match publish_accepted_rspress(&repository_root, &docs_root) {
        Ok(entries) => {
            let summary = RspressPublishSummary {
                command: "publish",
                mode: "rspress",
                schema_version: SCHEMA_VERSION,
                count: entries.len(),
                entries: entries
                    .into_iter()
                    .map(|entry| RspressPublishEntrySummary {
                        path: entry.path,
                        proposal_id: entry.proposal_id,
                        target_type: entry.target_type,
                        nav_text: entry.nav_text,
                        nav_link: entry.nav_link,
                    })
                    .collect(),
            };
            write_json_summary(out, err, &summary)
        }
        Err(error) => write_publish_error(err, "scryrs publish rspress", &error.to_string()),
    }
}

#[cfg(not(feature = "rspress"))]
fn execute_rspress_cli(_out: &mut impl Write, err: &mut impl Write, _args: &[String]) -> i32 {
    let _ = writeln!(
        err,
        "scryrs publish rspress: unavailable (rspress feature not enabled)"
    );
    2
}

fn write_markdown_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "Usage: scryrs publish markdown <PATH> --output <DIR>\n\
Publishes accepted Markdown-backed review decisions under <DIR>/<target-type>/<proposal-id>.md.\n\
Reads .scryrs/accepted/ only and never deletes stale generic Markdown files."
    )
}

fn write_rspress_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "Usage: scryrs publish rspress <PATH> --docs-root <DIR>\n\
Publishes accepted Markdown-backed review decisions into <DIR>/accepted-knowledge/ and updates <DIR>/_nav.json deterministically.\n\
Validates malformed _nav.json before clearing or rewriting accepted-knowledge/."
    )
}

fn resolve_absolute(path: &str, command_name: &str, err: &mut impl Write) -> Result<PathBuf, i32> {
    std::path::absolute(Path::new(path)).map_err(|error| {
        let _ = writeln!(err, "{command_name}: cannot resolve path '{path}': {error}");
        2
    })
}

fn write_publish_error(err: &mut impl Write, command_name: &str, message: &str) -> i32 {
    if writeln!(err, "{message}").is_err() {
        return 1;
    }
    if is_publish_validation_error(message) {
        2
    } else {
        let _ = writeln!(err, "{command_name}: runtime failure");
        1
    }
}

fn is_publish_validation_error(message: &str) -> bool {
    message.contains("invalid accepted artifact")
        || message.contains("malformed _nav.json")
        || message.contains("_nav.json is not a JSON array")
        || (message.contains("expected directory") && message.contains(".scryrs/accepted"))
}

fn write_usage_error(err: &mut impl Write, message: &str, usage_lines: &[&str]) -> i32 {
    if writeln!(err, "{message}").is_err() {
        return 1;
    }
    for usage_line in usage_lines {
        if writeln!(err, "Usage: {usage_line}").is_err() {
            return 1;
        }
    }
    if writeln!(err, "See `scryrs publish --help`").is_err() {
        return 1;
    }
    2
}

fn write_json_summary(out: &mut impl Write, err: &mut impl Write, summary: &impl Serialize) -> i32 {
    match serde_json::to_string(summary) {
        Ok(json) => writeln!(out, "{json}").map_or_else(
            |_| {
                let _ = writeln!(err, "scryrs publish: cannot write stdout");
                1
            },
            |_| 0,
        ),
        Err(error) => {
            let _ = writeln!(err, "scryrs publish: serialization error: {error}");
            1
        }
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct MarkdownPublishSummary<'a> {
    command: &'a str,
    mode: &'a str,
    schema_version: &'a str,
    count: usize,
    paths: Vec<String>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RspressPublishSummary<'a> {
    command: &'a str,
    mode: &'a str,
    schema_version: &'a str,
    count: usize,
    entries: Vec<RspressPublishEntrySummary>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RspressPublishEntrySummary {
    path: String,
    proposal_id: String,
    target_type: String,
    nav_text: String,
    nav_link: String,
}
