use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::remote_config::{
    RemoteOverrides, resolve_dashboard_target, resolve_proposal_write_token, resolve_remote_inputs,
};

const COMMAND: &str = "scryrs proposals publish";
const MAX_ATTEMPTS: usize = 2;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ProposalPublicationResponse {
    repository_id: String,
    proposal_id: String,
    schema_version: String,
    revision_sha256: String,
    publisher_id: String,
    published_at: String,
    unchanged: bool,
}

trait ProposalPublisher {
    fn publish(
        &self,
        server_url: &str,
        repository_id: &str,
        token: &str,
        proposal_json: &[u8],
        timeout_ms: u64,
    ) -> Result<ProposalPublicationResponse, PublishError>;
}

struct UreqProposalPublisher;

#[derive(Debug, Clone, PartialEq, Eq)]
enum PublishError {
    Timeout,
    Connection(String),
    HttpStatus { status: u16, body: String },
    MalformedResponse(String),
}

impl PublishError {
    fn retryable(&self) -> bool {
        matches!(self, Self::Timeout | Self::Connection(_))
            || matches!(self, Self::HttpStatus { status, .. } if *status >= 500)
    }
}

impl std::fmt::Display for PublishError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Timeout => write!(formatter, "proposal publication timed out"),
            Self::Connection(error) => write!(formatter, "cannot reach live server: {error}"),
            Self::HttpStatus { status, body } => write!(
                formatter,
                "live server returned HTTP {status}: {}",
                body.lines().next().unwrap_or("(empty body)")
            ),
            Self::MalformedResponse(error) => {
                write!(formatter, "invalid proposal publication response: {error}")
            }
        }
    }
}

impl ProposalPublisher for UreqProposalPublisher {
    fn publish(
        &self,
        server_url: &str,
        repository_id: &str,
        token: &str,
        proposal_json: &[u8],
        timeout_ms: u64,
    ) -> Result<ProposalPublicationResponse, PublishError> {
        let url = format!(
            "{}/v1/repositories/{}/proposals",
            server_url.trim_end_matches('/'),
            encode_path_segment(repository_id)
        );
        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .set("User-Agent", "scryrs-cli/0.1.0")
            .timeout(Duration::from_millis(timeout_ms))
            .send_bytes(proposal_json);
        match response {
            Ok(response) => parse_response(response),
            Err(ureq::Error::Status(status, response)) => Err(PublishError::HttpStatus {
                status,
                body: response
                    .into_string()
                    .unwrap_or_else(|error| format!("<read error: {error}>")),
            }),
            Err(ureq::Error::Transport(error)) => {
                let message = error.to_string();
                if message.contains("timed out") || message.contains("Timeout") {
                    Err(PublishError::Timeout)
                } else {
                    Err(PublishError::Connection(message))
                }
            }
        }
    }
}

fn parse_response(response: ureq::Response) -> Result<ProposalPublicationResponse, PublishError> {
    let body = response
        .into_string()
        .map_err(|error| PublishError::MalformedResponse(error.to_string()))?;
    serde_json::from_str(&body)
        .map_err(|error| PublishError::MalformedResponse(format!("{error}: {body}")))
}

pub(crate) fn execute_proposal_publish(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_proposal_publish_help(out).map_or(1, |_| 0);
    }
    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => return usage_error(err, &message),
    };
    let repo_root = match resolve_repo_root(parsed.path) {
        Ok(path) => path,
        Err(message) => {
            let _ = writeln!(err, "{COMMAND}: {message}");
            return 2;
        }
    };
    let proposals = match scryrs_curator::proposals::inventory::load_proposals(&repo_root) {
        Ok(proposals) => proposals,
        Err(error) => {
            let _ = writeln!(err, "{COMMAND}: {error}");
            return 2;
        }
    };
    let proposal = match proposals.get(parsed.proposal_id) {
        Some(proposal) => proposal,
        None => {
            let _ = writeln!(
                err,
                "{COMMAND}: unknown proposal ID '{}'",
                parsed.proposal_id
            );
            return 2;
        }
    };
    let proposal_json = match scryrs_curator::proposals::wire::serialize_proposal(proposal) {
        Ok(json) => json.into_bytes(),
        Err(error) => {
            let _ = writeln!(err, "{COMMAND}: invalid proposal: {error}");
            return 2;
        }
    };
    let target = match resolve_dashboard_target(
        Some(&repo_root),
        parsed.server_url,
        parsed.repository_id,
    ) {
        Ok(Some(target)) => target,
        Ok(None) => {
            let _ = writeln!(
                err,
                "{COMMAND}: live server URL is not configured; set --server-url or SCRYRS_REMOTE_INGEST_URL"
            );
            return 2;
        }
        Err(error) => {
            let _ = writeln!(err, "{COMMAND}: {error}");
            return 2;
        }
    };
    let token = match resolve_proposal_write_token(Some(&repo_root)) {
        Some(token) => token,
        None => {
            let _ = writeln!(
                err,
                "{COMMAND}: SCRYRS_PROPOSAL_WRITE_TOKEN is not configured in the environment or .scryrs/.env"
            );
            return 2;
        }
    };
    let overrides = RemoteOverrides {
        ingest_url: parsed.server_url.map(str::to_owned),
        repository_id: parsed.repository_id.map(str::to_owned),
        ..RemoteOverrides::default()
    };
    let timeout_ms = resolve_remote_inputs(Some(&repo_root), &overrides).timeout_ms;
    let response = match publish_with_retry(
        &UreqProposalPublisher,
        &target.0,
        &target.1,
        &token,
        &proposal_json,
        timeout_ms,
    ) {
        Ok(response) => response,
        Err(error) => {
            let _ = writeln!(err, "{COMMAND}: {error}");
            return 1;
        }
    };
    if response.repository_id != target.1 || response.proposal_id != proposal.id {
        let _ = writeln!(err, "{COMMAND}: publication response identity mismatch");
        return 1;
    }
    if let Err(error) = serde_json::to_writer(&mut *out, &response) {
        let _ = writeln!(err, "{COMMAND}: cannot write response: {error}");
        return 1;
    }
    writeln!(out).map_or(1, |()| 0)
}

pub(crate) fn write_proposal_publish_help(out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "scryrs proposals publish — publish one local proposal to live storage\n\n\
USAGE\n\
  scryrs proposals publish <PATH> <ID> [--server-url <URL>] [--repository-id <ID>]\n\n\
DESCRIPTION\n\
  Loads and validates .scryrs/proposals/{{ID}}.json and preserves the local proposal artifact\n\
  byte-for-byte while publishing complete schema, evidence, and content. Transient transport and 5xx failures\n\
  retry once; server revision hashes make replay idempotent.\n\n\
CONFIGURATION\n\
  Server URL and repository ID use normal remote precedence. Token resolves only\n\
  from SCRYRS_PROPOSAL_WRITE_TOKEN or .scryrs/.env; no token flag exists.\n\n\
EXIT CODES\n\
  0    Published or idempotent replay\n\
  1    Network, server, or response-contract failure\n\
  2    Usage/config error or invalid/missing local proposal"
    )
}

fn publish_with_retry(
    publisher: &impl ProposalPublisher,
    server_url: &str,
    repository_id: &str,
    token: &str,
    proposal_json: &[u8],
    timeout_ms: u64,
) -> Result<ProposalPublicationResponse, PublishError> {
    let mut last_error = None;
    for attempt in 0..MAX_ATTEMPTS {
        match publisher.publish(server_url, repository_id, token, proposal_json, timeout_ms) {
            Ok(response) => return Ok(response),
            Err(error) if error.retryable() && attempt + 1 < MAX_ATTEMPTS => {
                last_error = Some(error);
            }
            Err(error) => return Err(error),
        }
    }
    Err(last_error.unwrap_or_else(|| PublishError::Connection("retry exhausted".into())))
}

struct ParsedArgs<'a> {
    path: &'a str,
    proposal_id: &'a str,
    server_url: Option<&'a str>,
    repository_id: Option<&'a str>,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs<'_>, String> {
    let mut positional = Vec::new();
    let mut server_url = None;
    let mut repository_id = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--server-url" | "--repository-id" => {
                let flag = args[index].as_str();
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("{flag} requires a value"))?;
                if flag == "--server-url" {
                    server_url = Some(value.as_str());
                } else {
                    repository_id = Some(value.as_str());
                }
                index += 2;
            }
            value if value.starts_with('-') => {
                return Err(format!("unexpected argument '{value}'"));
            }
            value => {
                positional.push(value);
                index += 1;
            }
        }
    }
    if positional.len() != 2 {
        return Err("requires PATH and ID".into());
    }
    Ok(ParsedArgs {
        path: positional[0],
        proposal_id: positional[1],
        server_url,
        repository_id,
    })
}

fn resolve_repo_root(path: &str) -> Result<PathBuf, String> {
    let path = Path::new(path);
    if !path.exists() {
        return Err(format!("path does not exist: {}", path.display()));
    }
    if !path.is_dir() {
        return Err(format!("path is not a directory: {}", path.display()));
    }
    path.canonicalize()
        .map_err(|error| format!("cannot resolve path {}: {error}", path.display()))
}

fn usage_error(err: &mut impl Write, message: &str) -> i32 {
    let _ = writeln!(err, "{COMMAND}: {message}");
    let _ = writeln!(
        err,
        "Usage: scryrs proposals publish <PATH> <ID> [--server-url <URL>] [--repository-id <ID>]"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

fn encode_path_segment(value: &str) -> String {
    value
        .bytes()
        .map(|byte| {
            if byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'.' | b'_' | b'~') {
                (byte as char).to_string()
            } else {
                format!("%{byte:02X}")
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use std::cell::RefCell;
    use std::collections::VecDeque;

    use super::*;

    struct FakePublisher {
        outcomes: RefCell<VecDeque<Result<ProposalPublicationResponse, PublishError>>>,
        calls: RefCell<usize>,
    }

    impl ProposalPublisher for FakePublisher {
        fn publish(
            &self,
            _server_url: &str,
            _repository_id: &str,
            _token: &str,
            _proposal_json: &[u8],
            _timeout_ms: u64,
        ) -> Result<ProposalPublicationResponse, PublishError> {
            *self.calls.borrow_mut() += 1;
            self.outcomes
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| panic!("missing fake outcome"))
        }
    }

    fn success() -> ProposalPublicationResponse {
        ProposalPublicationResponse {
            repository_id: "repo-a".into(),
            proposal_id: "proposal-a".into(),
            schema_version: scryrs_types::PROPOSAL_SCHEMA_VERSION.into(),
            revision_sha256: "a".repeat(64),
            publisher_id: "alice".into(),
            published_at: "2026-07-26T12:00:00Z".into(),
            unchanged: false,
        }
    }

    #[test]
    fn transient_server_failure_retries_once() {
        let publisher = FakePublisher {
            outcomes: RefCell::new(
                vec![
                    Err(PublishError::HttpStatus {
                        status: 503,
                        body: "unavailable".into(),
                    }),
                    Ok(success()),
                ]
                .into(),
            ),
            calls: RefCell::new(0),
        };
        let response =
            publish_with_retry(&publisher, "http://server", "repo-a", "token", b"{}", 100)
                .unwrap_or_else(|error| panic!("publish: {error}"));
        assert_eq!(response, success());
        assert_eq!(*publisher.calls.borrow(), 2);
    }
}
