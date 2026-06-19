//! Feature-gated CLI scaffold for scryrs.

use std::io::{self, Write};

use scryrs_types::{FeatureDescriptor, SCHEMA_VERSION};

pub fn run<I, S>(args: I) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
{
    run_with_writers(args, io::stdout().lock(), io::stderr().lock())
}

pub fn run_with_writers<I, S, O, E>(args: I, mut out: O, mut err: E) -> i32
where
    I: IntoIterator<Item = S>,
    S: Into<String>,
    O: Write,
    E: Write,
{
    let args: Vec<String> = args.into_iter().map(Into::into).collect();

    match args.as_slice() {
        [] => write_help(&mut out).map_or(1, |_| 0),
        [flag] if flag == "--help" || flag == "-h" => write_help(&mut out).map_or(1, |_| 0),
        [flag] if flag == "--version" || flag == "-V" => {
            writeln!(out, "scryrs {}", env!("CARGO_PKG_VERSION")).map_or(1, |_| 0)
        }
        [command] if command == "components" => write_components_text(&mut out).map_or(1, |_| 0),
        [command, flag, format]
            if command == "components" && flag == "--format" && format == "json" =>
        {
            write_components_json(&mut out).map_or(1, |_| 0)
        }
        [command] if is_known_stub(command) => writeln!(
            out,
            "scryrs {command}: scaffold only; implementation pending"
        )
        .map_or(1, |_| 0),
        [unknown, ..] => {
            if writeln!(err, "unknown command: {unknown}").is_err()
                || writeln!(err, "run `scryrs --help`").is_err()
            {
                1
            } else {
                2
            }
        }
    }
}

#[allow(clippy::vec_init_then_push)]
fn descriptors() -> Vec<FeatureDescriptor> {
    let mut features = Vec::new();

    #[cfg(feature = "core")]
    features.push(scryrs_core::descriptor());

    #[cfg(feature = "graph")]
    features.push(scryrs_graph::descriptor());

    #[cfg(feature = "policy")]
    features.push(scryrs_policy::descriptor());

    #[cfg(feature = "curator")]
    features.push(scryrs_curator::descriptor());

    #[cfg(feature = "llm")]
    features.push(scryrs_llm::descriptor());

    #[cfg(feature = "markdown")]
    features.push(scryrs_adapter_markdown::descriptor());

    #[cfg(feature = "rspress")]
    features.push(scryrs_adapter_rspress::descriptor());

    #[cfg(feature = "runtime")]
    features.push(scryrs_runtime::descriptor());

    #[cfg(feature = "sandbox")]
    features.push(scryrs_sandbox::descriptor());

    #[cfg(feature = "telemetry")]
    features.push(scryrs_telemetry::descriptor());

    features
}

fn write_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs - context intelligence for AI-assisted codebases\n\n\
Usage:\n\
  scryrs components [--format json]\n\
  scryrs trace\n\
  scryrs hotspots\n\
  scryrs propose\n\
  scryrs graph\n\
  scryrs route\n\
  scryrs adapters\n\n\
Scaffold commands print placeholders until feature implementations land."
    )
}

fn write_components_text(out: &mut impl Write) -> io::Result<()> {
    for feature in descriptors() {
        writeln!(out, "{} - {}", feature.title, feature.summary)?;
    }

    Ok(())
}

fn write_components_json(out: &mut impl Write) -> io::Result<()> {
    let components = descriptors();
    writeln!(out, "{{")?;
    writeln!(
        out,
        "  \"schemaVersion\": \"{}\",",
        escape_json(SCHEMA_VERSION)
    )?;
    writeln!(out, "  \"components\": [")?;

    for (index, feature) in components.iter().enumerate() {
        let comma = if index + 1 == components.len() {
            ""
        } else {
            ","
        };
        writeln!(
            out,
            "    {{ \"id\": \"{}\", \"title\": \"{}\", \"summary\": \"{}\" }}{}",
            escape_json(feature.id),
            escape_json(feature.title),
            escape_json(feature.summary),
            comma
        )?;
    }

    writeln!(out, "  ]")?;
    writeln!(out, "}}")
}

fn is_known_stub(command: &str) -> bool {
    matches!(
        command,
        "trace"
            | "hotspots"
            | "report"
            | "suggest-docs"
            | "propose"
            | "graph"
            | "route"
            | "adapters"
    )
}

fn escape_json(value: &str) -> String {
    value
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn help_returns_success() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["--help"], &mut out, &mut err), 0);
        assert!(err.is_empty());
        assert!(String::from_utf8_lossy(&out).contains("Usage:"));
    }

    #[test]
    fn unknown_command_returns_usage_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run_with_writers(["unknown"], &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("unknown command: unknown"));
    }

    #[test]
    fn components_can_be_emitted_as_json() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(
            run_with_writers(["components", "--format", "json"], &mut out, &mut err),
            0
        );
        assert!(err.is_empty());

        let output = String::from_utf8_lossy(&out);
        assert!(output.contains("\"schemaVersion\": \"0.1.0\""));
        assert!(output.contains("\"components\""));
    }
}
