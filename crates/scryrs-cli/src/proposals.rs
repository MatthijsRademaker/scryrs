use std::collections::BTreeMap;
use std::ffi::OsStr;
use std::io::{self, Write};
use std::path::{Path, PathBuf};

use scryrs_types::{
    ProposalDocument, ProposalReviewDecision, ProposalTargetType, REVIEW_DECISION_SCHEMA_VERSION,
    ReviewOutcome,
};

pub(crate) fn execute_proposals_cli(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
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
        "accept" => execute_review_cli(out, err, &args[1..], ReviewOutcome::Accepted),
        "reject" => execute_review_cli(out, err, &args[1..], ReviewOutcome::Rejected),
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
  scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>\n\
  scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>\n\n\
SUBCOMMANDS\n\
  list\n\
      Emit deterministic JSON describing pending, accepted, and rejected proposal states.\n\
  accept\n\
      Write .scryrs/accepted/{{proposalId}}.json as a validated ProposalReviewDecision.\n\
  reject\n\
      Write .scryrs/rejected/{{proposalId}}.json as a validated ProposalReviewDecision.\n\n\
REQUIRED REVIEW METADATA\n\
  --reviewer <NAME>\n\
  --rationale <TEXT>\n\
  --decided-at <RFC3339>\n\n\
NOTES\n\
  singular `propose` generates proposals; plural `proposals` reviews them.\n\
  Review commands preserve .scryrs/proposals/{{proposalId}}.json unchanged.\n\
  Review commands write only under .scryrs/accepted/ and .scryrs/rejected/.\n\n\
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
    let target_dir = review_dir_name(&outcome);
    writeln!(
        out,
        "Usage: scryrs proposals {command} <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>\n\
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

    let state = match ProposalStateFilter::parse(state_raw.unwrap_or("all")) {
        Ok(state) => state,
        Err(message) => {
            return write_usage_error(
                err,
                &format!("scryrs proposals list: {message}"),
                &["scryrs proposals list <PATH> [--state pending|accepted|rejected|all]"],
            );
        }
    };

    match collect_list_rows(path, state) {
        Ok(rows) => match serde_json::to_string(&rows) {
            Ok(json) => writeln!(out, "{json}").map_or(1, |_| 0),
            Err(error) => {
                let _ = writeln!(err, "scryrs proposals list: serialization error: {error}");
                1
            }
        },
        Err(error) => {
            let _ = writeln!(err, "{}", error.message);
            error.exit_code
        }
    }
}

fn execute_review_cli(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
    outcome: ReviewOutcome,
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

    match write_review_decision(path, proposal_id, outcome, metadata) {
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

fn collect_list_rows(
    path: &str,
    filter: ProposalStateFilter,
) -> Result<Vec<ProposalListRow>, CommandError> {
    let repo_root = resolve_repo_root(path, "scryrs proposals list")?;
    let proposals = load_proposals(&repo_root, "scryrs proposals list")?;
    let accepted = load_review_decisions(&repo_root, ReviewOutcome::Accepted)?;
    let rejected = load_review_decisions(&repo_root, ReviewOutcome::Rejected)?;

    for proposal_id in accepted.keys() {
        if rejected.contains_key(proposal_id) {
            return Err(CommandError::input(format!(
                "scryrs proposals list: conflicting terminal state for proposal ID '{proposal_id}'"
            )));
        }
    }

    let mut rows = Vec::new();
    for (proposal_id, proposal) in proposals {
        let state = if accepted.contains_key(&proposal_id) {
            ProposalState::Accepted
        } else if rejected.contains_key(&proposal_id) {
            ProposalState::Rejected
        } else {
            ProposalState::Pending
        };

        if !filter.matches(state) {
            continue;
        }

        rows.push(ProposalListRow {
            proposal_id,
            title: proposal.title,
            target_type: proposal.target_type,
            created_at: proposal.created_at,
            state,
        });
    }
    Ok(rows)
}

fn load_proposals(
    repo_root: &Path,
    command_name: &str,
) -> Result<BTreeMap<String, ProposalDocument>, CommandError> {
    let proposals_dir = repo_root.join(".scryrs/proposals");
    let mut proposals = BTreeMap::new();

    for path in json_files_in_dir(&proposals_dir, command_name)? {
        let json = std::fs::read_to_string(&path).map_err(|error| {
            CommandError::input(format!(
                "{command_name}: cannot read proposal document {}: {error}",
                path.display()
            ))
        })?;
        let proposal: ProposalDocument = serde_json::from_str(&json).map_err(|error| {
            CommandError::input(format!(
                "{command_name}: invalid proposal document {}: {error}",
                path.display()
            ))
        })?;
        validate_proposal_document(command_name, &path, &proposal)?;
        if proposals.insert(proposal.id.clone(), proposal).is_some() {
            return Err(CommandError::input(format!(
                "{command_name}: duplicate proposal ID '{}' encountered while listing",
                path.file_stem().and_then(OsStr::to_str).unwrap_or_default()
            )));
        }
    }

    Ok(proposals)
}

fn load_review_decisions(
    repo_root: &Path,
    expected_outcome: ReviewOutcome,
) -> Result<BTreeMap<String, ProposalReviewDecision>, CommandError> {
    let command_name = "scryrs proposals list";
    let review_dir = repo_root.join(format!(".scryrs/{}", review_dir_name(&expected_outcome)));
    let proposals = load_proposals(repo_root, command_name)?;
    let mut decisions = BTreeMap::new();

    for path in json_files_in_dir(&review_dir, command_name)? {
        let json = std::fs::read_to_string(&path).map_err(|error| {
            CommandError::input(format!(
                "{command_name}: cannot read review decision {}: {error}",
                path.display()
            ))
        })?;
        let decision: ProposalReviewDecision = serde_json::from_str(&json).map_err(|error| {
            CommandError::input(format!(
                "{command_name}: invalid reviewed artifact {}: {error}",
                path.display()
            ))
        })?;
        validate_review_decision_artifact(&path, &decision, &expected_outcome)?;
        let proposal = proposals.get(&decision.proposal_id).ok_or_else(|| {
            CommandError::input(format!(
                "{command_name}: reviewed artifact {} has no matching proposal inbox document",
                path.display()
            ))
        })?;
        validate_review_decision_matches_proposal(command_name, &decision, proposal)?;
        if decisions
            .insert(decision.proposal_id.clone(), decision)
            .is_some()
        {
            return Err(CommandError::input(format!(
                "{command_name}: duplicate reviewed artifact for proposal ID '{}'",
                path.file_stem().and_then(OsStr::to_str).unwrap_or_default()
            )));
        }
    }

    Ok(decisions)
}

fn write_review_decision(
    path: &str,
    proposal_id: &str,
    outcome: ReviewOutcome,
    metadata: ReviewMetadata<'_>,
) -> Result<(), CommandError> {
    let command_name = format!("scryrs proposals {}", review_command_name(&outcome));
    validate_rfc3339(metadata.decided_at).map_err(|message| {
        CommandError::input(format!(
            "{command_name}: invalid --decided-at value: {message}"
        ))
    })?;

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
    validate_proposal_document(&command_name, &proposal_path, &proposal)?;

    let decision = build_review_decision(&proposal, &outcome, metadata);
    decision.validate().map_err(|error| {
        CommandError::input(format!("{command_name}: invalid review metadata: {error}"))
    })?;
    validate_review_decision_matches_proposal(&command_name, &decision, &proposal)?;

    let json = serde_json::to_string(&decision).map_err(|error| {
        CommandError::failure(format!("{command_name}: serialization error: {error}"))
    })?;

    let target_dir = repo_root.join(format!(".scryrs/{}", review_dir_name(&outcome)));
    let target_path = target_dir.join(format!("{proposal_id}.json"));
    let conflict_dir = repo_root.join(format!(
        ".scryrs/{}",
        review_dir_name(&opposite_outcome(&outcome))
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

fn validate_proposal_document(
    command_name: &str,
    path: &Path,
    proposal: &ProposalDocument,
) -> Result<(), CommandError> {
    proposal.validate().map_err(|error| {
        CommandError::input(format!(
            "{command_name}: invalid proposal document {}: {error}",
            path.display()
        ))
    })?;

    let expected_filename = format!("{}.json", proposal.id);
    let actual_filename = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    if actual_filename != expected_filename {
        return Err(CommandError::input(format!(
            "{command_name}: invalid proposal document {}: filename does not match proposalId '{}'",
            path.display(),
            proposal.id
        )));
    }

    let computed_id = ProposalDocument::compute_id(&proposal.target_type, &proposal.proposed_content)
        .map_err(|error| {
            CommandError::input(format!(
                "{command_name}: invalid proposal document {}: cannot compute deterministic proposalId: {error}",
                path.display()
            ))
        })?;
    if proposal.id != computed_id {
        return Err(CommandError::input(format!(
            "{command_name}: invalid proposal document {}: proposalId '{}' does not match targetType/proposedContent",
            path.display(),
            proposal.id
        )));
    }

    Ok(())
}

fn validate_review_decision_artifact(
    path: &Path,
    decision: &ProposalReviewDecision,
    expected_outcome: &ReviewOutcome,
) -> Result<(), CommandError> {
    let command_name = "scryrs proposals list";
    decision.validate().map_err(|error| {
        CommandError::input(format!(
            "{command_name}: invalid reviewed artifact {}: {error}",
            path.display()
        ))
    })?;
    validate_rfc3339(&decision.decided_at).map_err(|message| {
        CommandError::input(format!(
            "{command_name}: invalid reviewed artifact {}: decidedAt {message}",
            path.display()
        ))
    })?;

    let expected_filename = format!("{}.json", decision.proposal_id);
    let actual_filename = path.file_name().and_then(OsStr::to_str).unwrap_or_default();
    if actual_filename != expected_filename {
        return Err(CommandError::input(format!(
            "{command_name}: invalid reviewed artifact {}: filename does not match proposalId '{}'",
            path.display(),
            decision.proposal_id
        )));
    }
    if decision.outcome != *expected_outcome {
        return Err(CommandError::input(format!(
            "{command_name}: invalid reviewed artifact {}: outcome does not match {} directory",
            path.display(),
            review_dir_name(expected_outcome)
        )));
    }
    Ok(())
}

fn validate_review_decision_matches_proposal(
    command_name: &str,
    decision: &ProposalReviewDecision,
    proposal: &ProposalDocument,
) -> Result<(), CommandError> {
    if decision.proposal_id != proposal.id {
        return Err(CommandError::input(format!(
            "{command_name}: reviewed artifact proposalId '{}' does not match proposal inbox document '{}'",
            decision.proposal_id, proposal.id
        )));
    }
    if decision.source_evidence != proposal.evidence {
        return Err(CommandError::input(format!(
            "{command_name}: reviewed artifact for proposal ID '{}' does not preserve sourceEvidence from the proposal",
            proposal.id
        )));
    }
    match decision.outcome {
        ReviewOutcome::Accepted => {
            if decision.target_type.as_ref() != Some(&proposal.target_type) {
                return Err(CommandError::input(format!(
                    "{command_name}: reviewed artifact for proposal ID '{}' does not preserve targetType",
                    proposal.id
                )));
            }
            if decision.accepted_content.as_ref() != Some(&proposal.proposed_content) {
                return Err(CommandError::input(format!(
                    "{command_name}: reviewed artifact for proposal ID '{}' does not preserve acceptedContent",
                    proposal.id
                )));
            }
        }
        ReviewOutcome::Rejected => {}
    }
    Ok(())
}

fn build_review_decision(
    proposal: &ProposalDocument,
    outcome: &ReviewOutcome,
    metadata: ReviewMetadata<'_>,
) -> ProposalReviewDecision {
    let (target_type, accepted_content) = match outcome {
        ReviewOutcome::Accepted => (
            Some(proposal.target_type.clone()),
            Some(proposal.proposed_content.clone()),
        ),
        ReviewOutcome::Rejected => (None, None),
    };

    ProposalReviewDecision {
        schema_version: REVIEW_DECISION_SCHEMA_VERSION.into(),
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

fn json_files_in_dir(dir: &Path, command_name: &str) -> Result<Vec<PathBuf>, CommandError> {
    if !dir.exists() {
        return Ok(Vec::new());
    }
    if !dir.is_dir() {
        return Err(CommandError::input(format!(
            "{command_name}: expected directory {}",
            dir.display()
        )));
    }
    let mut paths = Vec::new();
    let entries = std::fs::read_dir(dir).map_err(|error| {
        CommandError::input(format!(
            "{command_name}: cannot read directory {}: {error}",
            dir.display()
        ))
    })?;
    for entry in entries {
        let entry = entry.map_err(|error| {
            CommandError::input(format!(
                "{command_name}: cannot read directory {}: {error}",
                dir.display()
            ))
        })?;
        let path = entry.path();
        if path.is_file() && path.extension() == Some(OsStr::new("json")) {
            paths.push(path);
        }
    }
    paths.sort();
    Ok(paths)
}

fn validate_rfc3339(value: &str) -> Result<(), String> {
    let (date, time_and_offset) = value
        .split_once('T')
        .ok_or_else(|| "must be RFC3339 (missing 'T')".to_string())?;
    validate_date(date)?;
    validate_time_and_offset(time_and_offset)
}

fn validate_date(date: &str) -> Result<(), String> {
    let mut parts = date.split('-');
    let year = parse_fixed_width_u32(parts.next(), 4, "year")?;
    let month = parse_fixed_width_u32(parts.next(), 2, "month")?;
    let day = parse_fixed_width_u32(parts.next(), 2, "day")?;
    if parts.next().is_some() {
        return Err("must be RFC3339 date (too many date fields)".into());
    }
    if !(1..=12).contains(&month) {
        return Err("must be RFC3339 date (month out of range)".into());
    }
    let max_day = days_in_month(year, month);
    if day == 0 || day > max_day {
        return Err("must be RFC3339 date (day out of range)".into());
    }
    Ok(())
}

fn validate_time_and_offset(value: &str) -> Result<(), String> {
    if let Some(prefix) = value.strip_suffix('Z') {
        validate_time(prefix)?;
        return Ok(());
    }

    let offset_index = value
        .rfind(['+', '-'])
        .ok_or_else(|| "must be RFC3339 timestamp with Z or ±HH:MM timezone offset".to_string())?;
    let (time, offset) = value.split_at(offset_index);
    validate_time(time)?;
    validate_offset(offset)
}

fn validate_time(time: &str) -> Result<(), String> {
    let (clock, fraction) = match time.split_once('.') {
        Some((clock, fraction)) => (clock, Some(fraction)),
        None => (time, None),
    };
    let mut parts = clock.split(':');
    let hour = parse_fixed_width_u32(parts.next(), 2, "hour")?;
    let minute = parse_fixed_width_u32(parts.next(), 2, "minute")?;
    let second = parse_fixed_width_u32(parts.next(), 2, "second")?;
    if parts.next().is_some() {
        return Err("must be RFC3339 time (too many time fields)".into());
    }
    if hour > 23 {
        return Err("must be RFC3339 time (hour out of range)".into());
    }
    if minute > 59 {
        return Err("must be RFC3339 time (minute out of range)".into());
    }
    if second > 60 {
        return Err("must be RFC3339 time (second out of range)".into());
    }
    if let Some(fraction) = fraction {
        if fraction.is_empty() || !fraction.chars().all(|ch| ch.is_ascii_digit()) {
            return Err("must be RFC3339 time (invalid fractional seconds)".into());
        }
    }
    Ok(())
}

fn validate_offset(offset: &str) -> Result<(), String> {
    if offset.len() != 6
        || !matches!(offset.as_bytes()[0], b'+' | b'-')
        || offset.as_bytes()[3] != b':'
    {
        return Err("must be RFC3339 timezone offset (expected ±HH:MM)".into());
    }
    let hour = offset[1..3]
        .parse::<u32>()
        .map_err(|_| "must be RFC3339 timezone offset (invalid offset hour)".to_string())?;
    let minute = offset[4..6]
        .parse::<u32>()
        .map_err(|_| "must be RFC3339 timezone offset (invalid offset minute)".to_string())?;
    if hour > 23 {
        return Err("must be RFC3339 timezone offset (hour out of range)".into());
    }
    if minute > 59 {
        return Err("must be RFC3339 timezone offset (minute out of range)".into());
    }
    Ok(())
}

fn parse_fixed_width_u32(
    value: Option<&str>,
    width: usize,
    field_name: &str,
) -> Result<u32, String> {
    let value = value.ok_or_else(|| format!("must be RFC3339 ({field_name} missing)"))?;
    if value.len() != width || !value.chars().all(|ch| ch.is_ascii_digit()) {
        return Err(format!(
            "must be RFC3339 ({field_name} has invalid width or characters)"
        ));
    }
    value
        .parse::<u32>()
        .map_err(|_| format!("must be RFC3339 ({field_name} is not numeric)"))
}

fn days_in_month(year: u32, month: u32) -> u32 {
    match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if is_leap_year(year) => 29,
        2 => 28,
        _ => 0,
    }
}

fn is_leap_year(year: u32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || year % 400 == 0
}

fn review_command_name(outcome: &ReviewOutcome) -> &'static str {
    match outcome {
        ReviewOutcome::Accepted => "accept",
        ReviewOutcome::Rejected => "reject",
    }
}

fn review_dir_name(outcome: &ReviewOutcome) -> &'static str {
    match outcome {
        ReviewOutcome::Accepted => "accepted",
        ReviewOutcome::Rejected => "rejected",
    }
}

fn opposite_outcome(outcome: &ReviewOutcome) -> ReviewOutcome {
    match outcome {
        ReviewOutcome::Accepted => ReviewOutcome::Rejected,
        ReviewOutcome::Rejected => ReviewOutcome::Accepted,
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ProposalStateFilter {
    Pending,
    Accepted,
    Rejected,
    All,
}

impl ProposalStateFilter {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "pending" => Ok(Self::Pending),
            "accepted" => Ok(Self::Accepted),
            "rejected" => Ok(Self::Rejected),
            "all" => Ok(Self::All),
            other => Err(format!(
                "invalid --state value '{other}' (expected pending, accepted, rejected, or all)"
            )),
        }
    }

    fn matches(self, state: ProposalState) -> bool {
        match self {
            Self::All => true,
            Self::Pending => state == ProposalState::Pending,
            Self::Accepted => state == ProposalState::Accepted,
            Self::Rejected => state == ProposalState::Rejected,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
enum ProposalState {
    Pending,
    Accepted,
    Rejected,
}

#[derive(Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalListRow {
    proposal_id: String,
    title: String,
    target_type: ProposalTargetType,
    created_at: String,
    state: ProposalState,
}

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
