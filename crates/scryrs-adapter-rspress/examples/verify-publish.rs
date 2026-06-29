//! Verification harness: invoked by `scripts/verify-docs-publish` to run the
//! Rspress adapter against command-line-provided repository and docs roots.
//!
//! Usage: cargo run --example verify-publish -- <repo_root> <docs_root>

#![allow(clippy::print_stderr)]

use scryrs_adapter_rspress::publish_accepted_rspress;
use std::env;
use std::path::Path;
use std::process;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("Usage: {} <repo_root> <docs_root>", args[0]);
        process::exit(1);
    }

    let repo_root = Path::new(&args[1]);
    let docs_root = Path::new(&args[2]);

    match publish_accepted_rspress(repo_root, docs_root) {
        Ok(entries) => {
            eprintln!(
                "verify-publish: published {} entries to {}",
                entries.len(),
                docs_root.display()
            );
            for entry in &entries {
                eprintln!(
                    "  {} ({}/{})",
                    entry.path, entry.target_type, entry.proposal_id
                );
            }
        }
        Err(err) => {
            eprintln!("verify-publish: error: {err}");
            process::exit(1);
        }
    }
}
