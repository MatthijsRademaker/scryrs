# Agent Verification Workflow

You are running inside a Docker container without Go, Node.js, Python, or other SDKs installed.
All verification (build, test, lint) must run through the Docker-backed scripts in `scripts/`.

For container and runtime execution details, see `runtime-environment.md`.

## Recommended workflow

After making code changes, run verification through Docker:

```bash
scripts/precommit-run
```

This runs all pre-commit hooks (cargo fmt, cargo check, cargo clippy, cargo test)
inside Docker containers. It is the authoritative verification path used by CI and worker agents.

For receipt-backed full verification, use `run_development_verification` in-session or
`swarm-agent verification run-development --mode full` outside Pi.

## Individual scripts

To run specific checks in isolation, use the individual scripts:

```bash
scripts/check        # cargo fmt --check, cargo check, cargo clippy
scripts/test         # cargo test --workspace --all-targets
scripts/security     # cargo-deny, cargo-audit (security advisories)
```

## Do not use

- `cargo build`, `cargo test`, `cargo clippy` directly from host (no Rust SDK)
- `make lint`, `make test`, `make build` from `src/` (host SDK convenience, not available to agents)
- Any Node.js or Python tooling directly from host
