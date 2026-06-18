# Agent Verification Workflow

You are running inside a Docker container without Go, Node.js, Python, or other SDKs installed.
All verification (build, test, lint) must run through the Docker-backed scripts in `scripts/`.

For container and runtime execution details, see `runtime-environment.md`.

## Recommended workflow

After making code changes, run verification through Docker:

```bash
scripts/precommit-run --all-files
```

This runs pre-commit through a pinned verifier image and local hooks through the shared
Docker verification harness. Tool caches live in project-scoped Docker named volumes;
the repository workspace is only for source files and explicit outputs.

For receipt-backed full verification, use `run_development_verification` in-session or
`swarm-agent verification run-development --mode full` outside Pi.

## Individual scripts

To run specific checks in isolation, use the individual scripts instead of the full pre-commit run:

```bash
scripts/check-go          # go vet
scripts/format-go         # go formatting/import verification
scripts/lint-go           # golangci-lint backend profile
scripts/vuln-go           # govulncheck
scripts/test-go           # go test -v ./...
scripts/build-go          # go build -v ./...
scripts/check-dashboard   # dashboard type-check
scripts/lint-dashboard    # dashboard ESLint
scripts/fallow-dashboard  # dashboard dead-code/health audit
scripts/depcruise-dashboard # dashboard dependency graph rules
scripts/audit-dashboard   # dashboard package security audit
scripts/build-dashboard   # dashboard production build
scripts/generate-proto    # buf generate
```

## Cache and permission contract

- Do not create or depend on `.cache/go-build`, `.cache/go-mod`, `.cache/npm`, or `.cache/pre-commit`.
  Those repo-local tool-cache paths are obsolete for normal verification.
- If stale root-owned files from old scripts block work, run `scripts/repair-workspace-permissions`
  explicitly. Normal verification must not rely on workspace-wide `chown` cleanup.
- Use `scripts/verification-cache list` and `scripts/verification-cache rm` for project-scoped
  Docker verification cache volumes.

## Do not use

- `go build`, `go test`, `go vet` directly from host (no Go SDK)
- `make lint`, `make test`, `make build` from `src/` (host SDK convenience, not available to agents)
- Any Node.js or Python tooling directly from host
- Repository-local tool caches under `.cache/go-build`, `.cache/go-mod`, `.cache/npm`, or `.cache/pre-commit`
