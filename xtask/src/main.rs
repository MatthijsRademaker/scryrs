use std::io::{self, Write};

fn main() {
    let command = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "help".to_string());
    let exit_code = run(&command, io::stdout().lock(), io::stderr().lock());

    std::process::exit(exit_code);
}

fn run(command: &str, mut out: impl Write, mut err: impl Write) -> i32 {
    match command {
        "help" => write_help(&mut out).map_or(1, |_| 0),
        "bootstrap" => writeln!(
            out,
            "xtask bootstrap: scaffold only; implementation pending"
        )
        .map_or(1, |_| 0),
        "ci-fast" => {
            writeln!(out, "xtask ci-fast: scaffold only; implementation pending").map_or(1, |_| 0)
        }
        other => {
            if writeln!(err, "unknown xtask command: {other}").is_err() {
                1
            } else {
                2
            }
        }
    }
}

fn write_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "xtask commands:\n\
  bootstrap  prepare local developer environment (pending)\n\
  ci-fast    run fast local checks (pending)"
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_command_exits_with_usage_error() {
        let mut out = Vec::new();
        let mut err = Vec::new();

        assert_eq!(run("wat", &mut out, &mut err), 2);
        assert!(out.is_empty());
        assert!(String::from_utf8_lossy(&err).contains("unknown xtask command: wat"));
    }
}
