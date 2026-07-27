use std::io::Write;
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::remote_config::{
    RemoteOverrides, resolve_dashboard_target, resolve_remote_inputs, resolve_route_publish_token,
};
use crate::route_common::{load_route_manifest, resolve_repo_root, warn_if_route_artifact_changed};

const COMMAND: &str = "scryrs route publish";
const MAX_ATTEMPTS: usize = 2;

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct RouteManifestPublicationResponse {
    repository_id: String,
    schema_version: String,
    content_sha256: String,
    publisher_id: String,
    published_at: String,
    unchanged: bool,
}

trait RoutePublisher {
    fn publish(
        &self,
        server_url: &str,
        repository_id: &str,
        token: &str,
        manifest_json: &[u8],
        timeout_ms: u64,
    ) -> Result<RouteManifestPublicationResponse, PublishError>;
}

struct UreqRoutePublisher;

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
            Self::Timeout => write!(formatter, "route publication timed out"),
            Self::Connection(error) => write!(formatter, "cannot reach live server: {error}"),
            Self::HttpStatus { status, body } => write!(
                formatter,
                "live server returned HTTP {status}: {}",
                body.lines().next().unwrap_or("(empty body)")
            ),
            Self::MalformedResponse(error) => {
                write!(formatter, "invalid route publication response: {error}")
            }
        }
    }
}

impl RoutePublisher for UreqRoutePublisher {
    fn publish(
        &self,
        server_url: &str,
        repository_id: &str,
        token: &str,
        manifest_json: &[u8],
        timeout_ms: u64,
    ) -> Result<RouteManifestPublicationResponse, PublishError> {
        let url = format!(
            "{}/v1/repositories/{}/routes/manifest",
            server_url.trim_end_matches('/'),
            encode_path_segment(repository_id)
        );
        let response = ureq::post(&url)
            .set("Authorization", &format!("Bearer {token}"))
            .set("Content-Type", "application/json")
            .set("User-Agent", "scryrs-cli/0.1.0")
            .timeout(Duration::from_millis(timeout_ms))
            .send_bytes(manifest_json);
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

fn parse_response(
    response: ureq::Response,
) -> Result<RouteManifestPublicationResponse, PublishError> {
    let body = response
        .into_string()
        .map_err(|error| PublishError::MalformedResponse(error.to_string()))?;
    serde_json::from_str(&body)
        .map_err(|error| PublishError::MalformedResponse(format!("{error}: {body}")))
}

pub(crate) fn execute_route_publish(
    out: &mut impl Write,
    err: &mut impl Write,
    args: &[String],
) -> i32 {
    if args.len() == 1 && (args[0] == "--help" || args[0] == "-h") {
        return write_route_publish_help(out).map_or(1, |_| 0);
    }

    let parsed = match parse_args(args) {
        Ok(parsed) => parsed,
        Err(message) => return usage_error(err, &message),
    };
    let repo_root = match resolve_repo_root(err, COMMAND, parsed.path) {
        Ok(repo_root) => repo_root,
        Err(exit_code) => return exit_code,
    };
    let loaded = match load_route_manifest(err, COMMAND, &repo_root) {
        Ok(loaded) => loaded,
        Err(exit_code) => return exit_code,
    };
    let overrides = RemoteOverrides {
        ingest_url: parsed.server_url.map(str::to_owned),
        repository_id: parsed.repository_id.map(str::to_owned),
        ..RemoteOverrides::default()
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
    let token = match resolve_route_publish_token(Some(&repo_root)) {
        Some(token) => token,
        None => {
            let _ = writeln!(
                err,
                "{COMMAND}: SCRYRS_ROUTE_PUBLISH_TOKEN is not configured in the environment or .scryrs/.env"
            );
            return 2;
        }
    };
    let timeout_ms = resolve_remote_inputs(Some(&repo_root), &overrides).timeout_ms;

    let mut manifest = loaded.manifest.clone();
    match manifest.metadata.repository_id.as_deref() {
        Some(repository_id) if repository_id != target.1 => {
            let _ = writeln!(
                err,
                "{COMMAND}: route manifest repository '{}' does not match publication repository '{}'",
                repository_id, target.1
            );
            return 2;
        }
        _ => manifest.metadata.repository_id = Some(target.1.clone()),
    }
    let manifest_json = match serde_json::to_vec(&manifest) {
        Ok(json) => json,
        Err(error) => {
            let _ = writeln!(err, "{COMMAND}: cannot serialize route manifest: {error}");
            return 1;
        }
    };

    let response = match publish_with_retry(
        &UreqRoutePublisher,
        &target.0,
        &target.1,
        &token,
        &manifest_json,
        timeout_ms,
    ) {
        Ok(response) => response,
        Err(error) => {
            let _ = writeln!(err, "{COMMAND}: {error}");
            return 1;
        }
    };
    if response.repository_id != target.1 {
        let _ = writeln!(
            err,
            "{COMMAND}: response repository mismatch: got '{}', expected '{}'",
            response.repository_id, target.1
        );
        return 1;
    }
    if response.schema_version != scryrs_types::ROUTE_SCHEMA_VERSION {
        let _ = writeln!(
            err,
            "{COMMAND}: response schema mismatch: got '{}', expected '{}'",
            response.schema_version,
            scryrs_types::ROUTE_SCHEMA_VERSION
        );
        return 1;
    }

    warn_if_route_artifact_changed(err, COMMAND, &loaded.routes_path, &loaded.routes_json);
    if let Err(error) = serde_json::to_writer(&mut *out, &response) {
        let _ = writeln!(err, "{COMMAND}: cannot write response: {error}");
        return 1;
    }
    writeln!(out).map_or(1, |()| 0)
}

fn publish_with_retry(
    publisher: &impl RoutePublisher,
    server_url: &str,
    repository_id: &str,
    token: &str,
    manifest_json: &[u8],
    timeout_ms: u64,
) -> Result<RouteManifestPublicationResponse, PublishError> {
    let mut last_error = None;
    for attempt in 0..MAX_ATTEMPTS {
        match publisher.publish(server_url, repository_id, token, manifest_json, timeout_ms) {
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
    server_url: Option<&'a str>,
    repository_id: Option<&'a str>,
}

fn parse_args(args: &[String]) -> Result<ParsedArgs<'_>, String> {
    let mut path = None;
    let mut server_url = None;
    let mut repository_id = None;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--server-url" | "--repository-id" => {
                let flag = args[index].as_str();
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| format!("{COMMAND}: {flag} requires a value"))?;
                if flag == "--server-url" {
                    server_url = Some(value.as_str());
                } else {
                    repository_id = Some(value.as_str());
                }
                index += 2;
            }
            value if value.starts_with('-') => {
                return Err(format!("{COMMAND}: unexpected argument '{value}'"));
            }
            value if path.is_none() => {
                path = Some(value);
                index += 1;
            }
            value => return Err(format!("{COMMAND}: unexpected extra argument '{value}'")),
        }
    }
    Ok(ParsedArgs {
        path: path.ok_or_else(|| format!("{COMMAND}: missing required PATH argument"))?,
        server_url,
        repository_id,
    })
}

fn usage_error(err: &mut impl Write, message: &str) -> i32 {
    let _ = writeln!(err, "{message}");
    let _ = writeln!(
        err,
        "Usage: scryrs route publish <PATH> [--server-url <URL>] [--repository-id <ID>]"
    );
    let _ = writeln!(err, "See `scryrs --help`");
    2
}

fn write_route_publish_help(out: &mut impl Write) -> std::io::Result<()> {
    writeln!(
        out,
        "scryrs route publish — publish generated route manifest to live server\n\n\
USAGE\n\
  scryrs route publish <PATH> [--server-url <URL>] [--repository-id <ID>]\n\n\
DESCRIPTION\n\
  Reads <PATH>/.scryrs/routes.json without changing local route generation,\n\
  binds its publication copy to repository identity, and uploads it to the\n\
  authenticated latest-manifest endpoint. Transient transport and 5xx failures\n\
  are retried once; server idempotency makes replay safe.\n\n\
CONFIGURATION\n\
  Server URL and repository ID resolve from flags, environment, .scryrs/.env,\n\
  then scryrs.json remote. Authentication token resolves only from\n\
  SCRYRS_ROUTE_PUBLISH_TOKEN or .scryrs/.env; it is never accepted as a flag\n\
  or committed to scryrs.json.\n\n\
OUTPUT\n\
  Single-line publication metadata JSON with repositoryId, schemaVersion,\n\
  contentSha256, publisherId, publishedAt, and unchanged.\n\n\
EXIT CODES\n\
  0    Published or idempotent replay\n\
  1    Network, server, response-contract, serialization, or output failure\n\
  2    Usage/config error, missing artifact, malformed artifact, schema mismatch"
    )
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
        outcomes: RefCell<VecDeque<Result<RouteManifestPublicationResponse, PublishError>>>,
        calls: RefCell<usize>,
    }

    impl FakePublisher {
        fn new(outcomes: Vec<Result<RouteManifestPublicationResponse, PublishError>>) -> Self {
            Self {
                outcomes: RefCell::new(outcomes.into()),
                calls: RefCell::new(0),
            }
        }
    }

    impl RoutePublisher for FakePublisher {
        fn publish(
            &self,
            _server_url: &str,
            _repository_id: &str,
            _token: &str,
            _manifest_json: &[u8],
            _timeout_ms: u64,
        ) -> Result<RouteManifestPublicationResponse, PublishError> {
            *self.calls.borrow_mut() += 1;
            self.outcomes
                .borrow_mut()
                .pop_front()
                .unwrap_or_else(|| panic!("missing fake outcome"))
        }
    }

    fn success() -> RouteManifestPublicationResponse {
        RouteManifestPublicationResponse {
            repository_id: "repo-a".into(),
            schema_version: scryrs_types::ROUTE_SCHEMA_VERSION.into(),
            content_sha256: "a".repeat(64),
            publisher_id: "ci".into(),
            published_at: "2026-07-26T12:00:00Z".into(),
            unchanged: false,
        }
    }

    #[test]
    fn successful_publication_returns_metadata() {
        let publisher = FakePublisher::new(vec![Ok(success())]);
        let response =
            publish_with_retry(&publisher, "http://server", "repo-a", "token", b"{}", 100)
                .unwrap_or_else(|error| panic!("publish: {error}"));

        assert_eq!(response, success());
        assert_eq!(*publisher.calls.borrow(), 1);
    }

    #[test]
    fn transient_failure_is_retried_once() {
        let publisher = FakePublisher::new(vec![
            Err(PublishError::Connection("reset".into())),
            Ok(success()),
        ]);
        let response =
            publish_with_retry(&publisher, "http://server", "repo-a", "token", b"{}", 100)
                .unwrap_or_else(|error| panic!("publish: {error}"));

        assert_eq!(response, success());
        assert_eq!(*publisher.calls.borrow(), 2);
    }

    #[test]
    fn client_failure_is_not_retried() {
        let publisher = FakePublisher::new(vec![Err(PublishError::HttpStatus {
            status: 422,
            body: "schema mismatch".into(),
        })]);
        let error = publish_with_retry(&publisher, "http://server", "repo-a", "token", b"{}", 100)
            .err()
            .unwrap_or_else(|| panic!("422 must fail"));

        assert!(error.to_string().contains("HTTP 422"));
        assert_eq!(*publisher.calls.borrow(), 1);
    }

    #[test]
    fn path_segment_encoding_preserves_repository_identity() {
        assert_eq!(
            encode_path_segment("https://github.com/acme/repo"),
            "https%3A%2F%2Fgithub.com%2Facme%2Frepo"
        );
    }

    #[test]
    fn parse_args_requires_path_and_rejects_unknown_flags() {
        assert!(parse_args(&[]).is_err());
        assert!(parse_args(&["--token".into(), "secret".into()]).is_err());
    }

    #[test]
    fn schema_mismatch_artifact_is_rejected_before_publication() {
        let dir = tempfile::tempdir().unwrap_or_else(|error| panic!("tempdir: {error}"));
        std::fs::create_dir_all(dir.path().join(".scryrs"))
            .unwrap_or_else(|error| panic!("create artifact dir: {error}"));
        std::fs::write(
            dir.path().join(".scryrs/routes.json"),
            r#"{"schemaVersion":"99.0.0","metadata":{},"routes":[]}"#,
        )
        .unwrap_or_else(|error| panic!("write artifact: {error}"));
        let mut error = Vec::new();
        let result = load_route_manifest(&mut error, COMMAND, dir.path());

        assert!(result.is_err());
        assert!(String::from_utf8_lossy(&error).contains("schema version mismatch"));
    }
}
