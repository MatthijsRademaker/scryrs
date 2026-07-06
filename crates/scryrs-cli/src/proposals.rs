use std::io::{self, Read, Write};
use std::path::PathBuf;

use scryrs_curator::proposals::inventory::{
    self, InventoryError, ProposalListRow, ProposalStateFilter,
};
use scryrs_types::{ProposalDocument, ProposalReviewDecision, ProposedContent, ReviewOutcome};

pub(crate) fn execute_proposals_cli(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
    stdin: &mut impl Read,
) -> i32 {
    if args.is_empty() {
        return write_usage_error(
            err,
            "scryrs proposals: missing required subcommand",
            &["scryrs proposals --help"],
        );
    }

    match args[0].as_str() {
        "--help" | "-h" => write_proposals_help(out).map_or(1, |_| 0),
        "list" => execute_list_cli(out, err, &args[1..]),
        "accept" => execute_review_cli(out, err, &args[1..], ReviewOutcome::Accepted, stdin),
        "reject" => execute_review_cli(out, err, &args[1..], ReviewOutcome::Rejected, stdin),
        other => write_usage_error(
            err,
            &format!("scryrs proposals: unknown subcommand '{other}'"),
            &["scryrs proposals --help"],
        ),
    }
}

pub(crate) fn write_proposals_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "scryrs proposals — review proposal inbox artifacts\n\n\
USAGE\n\
  scryrs proposals list <PATH> [--state pending|accepted|rejected|all]\n\
  scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339> [--content-file <PATH> | --content-stdin]\n\
  scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>\n\n\
SUBCOMMANDS\n\
  list\n\
      Emit deterministic JSON describing pending, accepted, and rejected proposal states.\n\
  accept\n\
      Write .scryrs/accepted/{{proposalId}}.json as a validated ProposalReviewDecision.\n\
      Optional --content-file or --content-stdin overrides accepted Markdown content.\n\
  reject\n\
      Write .scryrs/rejected/{{proposalId}}.json as a validated ProposalReviewDecision.\n\n\
REQUIRED REVIEW METADATA\n\
  --reviewer <NAME>\n\
  --rationale <TEXT>\n\
  --decided-at <RFC3339>\n\n\
NOTES\n\
  singular `propose` generates proposals; plural `proposals` reviews them.\n\
  Review commands preserve .scryrs/proposals/{{proposalId}}.json unchanged.\n\
  Review commands write only under .scryrs/accepted/ and .scryrs/rejected/.\n\
  --content-file and --content-stdin are accept-only and mutually exclusive.\n\n\
EXIT CODES\n\
  0    Success\n\
  1    Serialization or filesystem write failure\n\
  2    Usage or input error"
    )
}

fn write_list_help(out: &mut impl Write) -> io::Result<()> {
    writeln!(
        out,
        "Usage: scryrs proposals list <PATH> [--state pending|accepted|rejected|all]\n\
Emit deterministic JSON rows sorted by proposalId ascending.\n\
State defaults to all."
    )
}

fn write_review_help(out: &mut impl Write, outcome: ReviewOutcome) -> io::Result<()> {
    let command = review_command_name(&outcome);
    let target_dir = inventory::review_dir_name(&outcome);
    let content_override_flags = if outcome == ReviewOutcome::Accepted {
        " [--content-file <PATH> | --content-stdin]"
    } else {
        ""
    };
    writeln!(
        out,
        "Usage: scryrs proposals {command} <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>{content_override_flags}\n\
Writes .scryrs/{target_dir}/{{proposalId}}.json and preserves the source proposal inbox file."
    )
}

fn execute_list_cli(out: &mut impl Write, err: &mut impl Write, args: &[String]) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_list_help(out).map_or(1, |_| 0);
    }

    let mut path: Option<&str> = None;
    let mut state_raw: Option<&str> = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--state" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return write_usage_error(
                        err,
                        "scryrs proposals list: missing value for --state",
                        &["scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"],
                    );
                };
                if state_raw.is_some() {
                    return write_usage_error(
                        err,
                        "scryrs proposals list: duplicate --state argument",
                        &["scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"],
                    );
                }
                state_raw = Some(value.as_str());
            }
            token if token.starts_with('-') => {
                return write_usage_error(
                    err,
                    &format!("scryrs proposals list: unexpected argument '{token}'"),
                    &["scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"],
                );
            }
            token => {
                if path.is_some() {
                    return write_usage_error(
                        err,
                        "scryrs proposals list: unexpected argument after PATH",
                        &["scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"],
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
            "scryrs proposals list: missing required PATH argument",
            &["scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"],
        );
    };

    let filter = match ProposalStateFilter::parse(state_raw.unwrap_or("all")) {
        Ok(state) => state,
        Err(message) => {
            return write_usage_error(
                err,
                &format!("scryrs proposals list: {message}"),
                &["scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"],
            );
        }
    };

    let repo_root = match resolve_repo_root(path, "scryrs proposals list") {
        Ok(root) => root,
        Err(error) => {
            let _ = writeln!(err, "{}", error.message);
            return error.exit_code;
        }
    };

    match inventory::collect_list_rows(&repo_root) {
        Ok(rows) => {
            let filtered: Vec<&ProposalListRow> =
                rows.iter().filter(|r| filter.matches(r.state)).collect();
            match serde_json::to_string(&filtered) {
                Ok(json) => writeln!(out, "{json}").map_or(1, |_| 0),
                Err(error) => {
                    let _ = writeln!(err, "scryrs proposals list: serialization error: {error}");
                    1
                }
            }
        }
        Err(error) => {
            let mapped = map_inventory_error(error);
            let _ = writeln!(err, "{}", mapped.message);
            mapped.exit_code
        }
    }
}

fn execute_review_cli(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
    outcome: ReviewOutcome,
    stdin: &mut impl Read,
) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_review_help(out, outcome).map_or(1, |_| 0);
    }

    let command = review_command_name(&outcome);
    let usage = format!(
        "scryrs proposals {command} <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>"
    );

    let mut path: Option<&str> = None;
    let mut proposal_id: Option<&str> = None;
    let mut reviewer: Option<&str> = None;
    let mut rationale: Option<&str> = None;
    let mut decided_at: Option<&str> = None;
    let mut content_file: Option<&str> = None;
    let mut content_stdin = false;

    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--reviewer" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: missing value for --reviewer"),
                        &[usage.as_str()],
                    );
                };
                if reviewer.is_some() {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: duplicate --reviewer argument"),
                        &[usage.as_str()],
                    );
                }
                reviewer = Some(value.as_str());
            }
            "--rationale" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: missing value for --rationale"),
                        &[usage.as_str()],
                    );
                };
                if rationale.is_some() {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: duplicate --rationale argument"),
                        &[usage.as_str()],
                    );
                }
                rationale = Some(value.as_str());
            }
            "--content-file" => {
                if outcome == ReviewOutcome::Rejected {
                    return write_usage_error(
                        err,
                        &format!(
                            "scryrs proposals {command}: --content-file is only supported on the accept subcommand",
                        ),
                        &[usage.as_str()],
                    );
                }
                if content_stdin {
                    return write_usage_error(
                        err,
                        &format!(
                            "scryrs proposals {command}: --content-file and --content-stdin are mutually exclusive",
                        ),
                        &[usage.as_str()],
                    );
                }
                index += 1;
                let Some(value) = args.get(index) else {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: missing value for --content-file"),
                        &[usage.as_str()],
                    );
                };
                if content_file.is_some() {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: duplicate --content-file argument"),
                        &[usage.as_str()],
                    );
                }
                content_file = Some(value.as_str());
            }
            "--content-stdin" => {
                if outcome == ReviewOutcome::Rejected {
                    return write_usage_error(
                        err,
                        &format!(
                            "scryrs proposals {command}: --content-stdin is only supported on the accept subcommand",
                        ),
                        &[usage.as_str()],
                    );
                }
                if content_file.is_some() {
                    return write_usage_error(
                        err,
                        &format!(
                            "scryrs proposals {command}: --content-file and --content-stdin are mutually exclusive",
                        ),
                        &[usage.as_str()],
                    );
                }
                if content_stdin {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: duplicate --content-stdin argument"),
                        &[usage.as_str()],
                    );
                }
                content_stdin = true;
            }
            "--decided-at" => {
                index += 1;
                let Some(value) = args.get(index) else {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: missing value for --decided-at"),
                        &[usage.as_str()],
                    );
                };
                if decided_at.is_some() {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: duplicate --decided-at argument"),
                        &[usage.as_str()],
                    );
                }
                decided_at = Some(value.as_str());
            }
            token if token.starts_with('-') => {
                return write_usage_error(
                    err,
                    &format!("scryrs proposals {command}: unexpected argument '{token}'"),
                    &[usage.as_str()],
                );
            }
            token => {
                if path.is_none() {
                    path = Some(token);
                } else if proposal_id.is_none() {
                    proposal_id = Some(token);
                } else {
                    return write_usage_error(
                        err,
                        &format!("scryrs proposals {command}: unexpected argument after ID"),
                        &[usage.as_str()],
                    );
                }
            }
        }
        index += 1;
    }

    let Some(path) = path else {
        return write_usage_error(
            err,
            &format!("scryrs proposals {command}: missing required PATH argument"),
            &[usage.as_str()],
        );
    };
    let Some(proposal_id) = proposal_id else {
        return write_usage_error(
            err,
            &format!("scryrs proposals {command}: missing required ID argument"),
            &[usage.as_str()],
        );
    };
    let Some(reviewer) = reviewer else {
        return write_usage_error(
            err,
            &format!("scryrs proposals {command}: missing required --reviewer argument"),
            &[usage.as_str()],
        );
    };
    let Some(rationale) = rationale else {
        return write_usage_error(
            err,
            &format!("scryrs proposals {command}: missing required --rationale argument"),
            &[usage.as_str()],
        );
    };
    let Some(decided_at) = decided_at else {
        return write_usage_error(
            err,
            &format!("scryrs proposals {command}: missing required --decided-at argument"),
            &[usage.as_str()],
        );
    };

    let metadata = ReviewMetadata {
        reviewer,
        rationale,
        decided_at,
    };

    let override_content = match (content_file, content_stdin) {
        (Some(file_path), _) => {
            let bytes = match std::fs::read(file_path) {
                Ok(bytes) => bytes,
                Err(error) => {
                    return write_usage_error(
                        err,
                        &format!(
                            "scryrs proposals {command}: cannot read --content-file '{file_path}': {error}",
                        ),
                        &[usage.as_str()],
                    );
                }
            };
            if bytes.is_empty() {
                return write_usage_error(
                    err,
                    &format!("scryrs proposals {command}: --content-file '{file_path}' is empty",),
                    &[usage.as_str()],
                );
            }
            let content = match String::from_utf8(bytes) {
                Ok(s) => s,
                Err(error) => {
                    return write_usage_error(
                        err,
                        &format!(
                            "scryrs proposals {command}: --content-file '{file_path}' is not valid UTF-8: {error}",
                        ),
                        &[usage.as_str()],
                    );
                }
            };
            Some(ProposedContent::Markdown(content))
        }
        (None, true) => {
            let mut buf = String::new();
            if stdin.read_to_string(&mut buf).is_err() {
                return write_usage_error(
                    err,
                    &format!("scryrs proposals {command}: cannot read content from stdin"),
                    &[usage.as_str()],
                );
            }
            if buf.is_empty() {
                return write_usage_error(
                    err,
                    &format!("scryrs proposals {command}: no content received on stdin"),
                    &[usage.as_str()],
                );
            }
            Some(ProposedContent::Markdown(buf))
        }
        (None, false) => None,
    };

    match write_review_decision(path, proposal_id, outcome, metadata, override_content) {
        Ok(()) => {
            let _ = out.flush();
            0
        }
        Err(error) => {
            let _ = writeln!(err, "{}", error.message);
            error.exit_code
        }
    }
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
    if writeln!(err, "See `scryrs proposals --help`").is_err() {
        return 1;
    }
    2
}

fn map_inventory_error(error: InventoryError) -> CommandError {
    match &error {
        InventoryError::MissingDirectory { path } => CommandError::input(format!(
            "scryrs proposals list: proposals directory not found: {}",
            path.display()
        )),
        InventoryError::Io { path, message } => CommandError::input(format!(
            "scryrs proposals list: cannot read {}: {message}",
            path.display()
        )),
        InventoryError::Parse { path, message } => CommandError::input(format!(
            "scryrs proposals list: invalid JSON {}: {message}",
            path.display()
        )),
        InventoryError::Validation { path, message } => CommandError::input(format!(
            "scryrs proposals list: invalid artifact {}: {message}",
            path.display()
        )),
        InventoryError::DuplicateId { path_a, path_b, .. } => CommandError::input(format!(
            "scryrs proposals list: duplicate proposal files {} and {}",
            path_a.display(),
            path_b.display()
        )),
        InventoryError::ConflictingState { proposal_id, .. } => CommandError::input(format!(
            "scryrs proposals list: conflicting terminal state for proposal ID '{proposal_id}'"
        )),
        InventoryError::OrphanReview { path, proposal_id } => CommandError::input(format!(
            "scryrs proposals list: reviewed artifact {} has no matching proposal inbox document for proposal ID '{proposal_id}'",
            path.display()
        )),
    }
}

fn write_review_decision(
    path: &str,
    proposal_id: &str,
    outcome: ReviewOutcome,
    metadata: ReviewMetadata<'_>,
    override_content: Option<ProposedContent>,
) -> Result<(), CommandError> {
    let command_name = format!("scryrs proposals {}", review_command_name(&outcome));
    scryrs_curator::proposals::inventory::validate_rfc3339(metadata.decided_at).map_err(
        |message| {
            CommandError::input(format!(
                "{command_name}: invalid --decided-at value: {message}"
            ))
        },
    )?;

    let repo_root = resolve_repo_root(path, &command_name)?;
    let proposal_path = repo_root.join(format!(".scryrs/proposals/{proposal_id}.json"));
    if !proposal_path.is_file() {
        return Err(CommandError::input(format!(
            "{command_name}: unknown proposal ID '{proposal_id}'"
        )));
    }

    let proposal_json = std::fs::read_to_string(&proposal_path).map_err(|error| {
        CommandError::input(format!(
            "{command_name}: cannot read proposal document {}: {error}",
            proposal_path.display()
        ))
    })?;
    let proposal: ProposalDocument = serde_json::from_str(&proposal_json).map_err(|error| {
        CommandError::input(format!(
            "{command_name}: invalid proposal document {}: {error}",
            proposal_path.display()
        ))
    })?;
    inventory::validate_proposal_document(&proposal_path, &proposal).map_err(|error| {
        CommandError::input(format!(
            "{command_name}: invalid proposal document {}: {error}",
            proposal_path.display()
        ))
    })?;

    if override_content.is_some() && !inventory::is_markdown_target_type(&proposal.target_type) {
        return Err(CommandError::input(format!(
            "{command_name}: --content-file and --content-stdin are not supported for target type '{}'",
            serde_json::to_string(&proposal.target_type)
                .unwrap_or_else(|_| format!("{:?}", proposal.target_type))
        )));
    }

    let decision = build_review_decision(&proposal, &outcome, metadata, override_content);
    decision.validate().map_err(|error| {
        CommandError::input(format!("{command_name}: invalid review metadata: {error}"))
    })?;
    inventory::validate_review_decision_matches_proposal(&decision, &proposal).map_err(
        |error| CommandError::input(format!("{command_name}: review validation failed: {error}")),
    )?;

    let json = serde_json::to_string(&decision).map_err(|error| {
        CommandError::failure(format!("{command_name}: serialization error: {error}"))
    })?;

    let target_dir = repo_root.join(format!(".scryrs/{}", inventory::review_dir_name(&outcome)));
    let target_path = target_dir.join(format!("{proposal_id}.json"));
    let conflict_dir = repo_root.join(format!(
        ".scryrs/{}",
        inventory::review_dir_name(&opposite_outcome(&outcome))
    ));
    let conflict_path = conflict_dir.join(format!("{proposal_id}.json"));
    if conflict_path.exists() {
        return Err(CommandError::input(format!(
            "{command_name}: conflicting terminal decision already exists at {}",
            conflict_path.display()
        )));
    }

    if target_path.exists() {
        let existing = std::fs::read_to_string(&target_path).map_err(|error| {
            CommandError::input(format!(
                "{command_name}: cannot read existing review decision {}: {error}",
                target_path.display()
            ))
        })?;
        if existing == json {
            return Ok(());
        }
        return Err(CommandError::input(format!(
            "{command_name}: existing review decision differs from requested bytes; refusing to overwrite {}",
            target_path.display()
        )));
    }

    std::fs::create_dir_all(&target_dir).map_err(|error| {
        CommandError::failure(format!(
            "{command_name}: cannot create review directory {}: {error}",
            target_dir.display()
        ))
    })?;
    std::fs::write(&target_path, json).map_err(|error| {
        CommandError::failure(format!(
            "{command_name}: cannot write review decision {}: {error}",
            target_path.display()
        ))
    })?;

    Ok(())
}

fn build_review_decision(
    proposal: &ProposalDocument,
    outcome: &ReviewOutcome,
    metadata: ReviewMetadata<'_>,
    override_content: Option<ProposedContent>,
) -> ProposalReviewDecision {
    let (target_type, accepted_content) = match outcome {
        ReviewOutcome::Accepted => {
            let content = match override_content {
                Some(overridden) if inventory::is_markdown_target_type(&proposal.target_type) => {
                    Some(overridden)
                }
                _ => Some(proposal.proposed_content.clone()),
            };
            (Some(proposal.target_type.clone()), content)
        }
        ReviewOutcome::Rejected => (None, None),
    };

    ProposalReviewDecision {
        schema_version: scryrs_types::REVIEW_DECISION_SCHEMA_VERSION.into(),
        proposal_id: proposal.id.clone(),
        reviewer: metadata.reviewer.to_string(),
        decided_at: metadata.decided_at.to_string(),
        rationale: metadata.rationale.to_string(),
        source_evidence: proposal.evidence.clone(),
        outcome: outcome.clone(),
        target_type,
        accepted_content,
    }
}

fn resolve_repo_root(path: &str, command_name: &str) -> Result<PathBuf, CommandError> {
    std::path::absolute(path).map_err(|error| {
        CommandError::input(format!(
            "{command_name}: cannot resolve path '{path}': {error}"
        ))
    })
}

fn opposite_outcome(outcome: &ReviewOutcome) -> ReviewOutcome {
    match outcome {
        ReviewOutcome::Accepted => ReviewOutcome::Rejected,
        ReviewOutcome::Rejected => ReviewOutcome::Accepted,
    }
}

fn review_command_name(outcome: &ReviewOutcome) -> &'static str {
    match outcome {
        ReviewOutcome::Accepted => "accept",
        ReviewOutcome::Rejected => "reject",
    }
}

// ---------------------------------------------------------------------------
// Internal types
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy)]
struct ReviewMetadata<'a> {
    reviewer: &'a str,
    rationale: &'a str,
    decided_at: &'a str,
}

#[derive(Debug)]
struct CommandError {
    exit_code: i32,
    message: String,
}

impl CommandError {
    fn input(message: String) -> Self {
        Self {
            exit_code: 2,
            message,
        }
    }

    fn failure(message: String) -> Self {
        Self {
            exit_code: 1,
            message,
        }
    }
}
