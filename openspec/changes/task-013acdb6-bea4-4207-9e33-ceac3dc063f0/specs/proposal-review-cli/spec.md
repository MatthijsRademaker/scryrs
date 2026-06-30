## MODIFIED Requirements

### Requirement: Proposal review commands are registered and discoverable

The CLI SHALL register a grouped `proposals` root command with `list`, `accept`, and `reject` subcommands. When built with the non-default `curator-llm` feature, the same grouped root command SHALL also expose an `assist` subgroup with `draft` and `group` subcommands. The final review command surface SHALL be:

- `scryrs proposals list <PATH> [--state pending|accepted|rejected|all]`
- `scryrs proposals accept <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- `scryrs proposals reject <PATH> <ID> --reviewer <NAME> --rationale <TEXT> --decided-at <RFC3339>`
- `scryrs proposals assist draft <PATH> <ID> --model <MODEL_ID> [--include-hotspots] [--include-graph] [--max-input-chars <N>] [--max-hotspots <N>] [--max-graph-nodes <N>] [--max-proposals <N>] [--max-documents <N>] [--max-output-tokens <N>] [--timeout-ms <N>] [--write]`
- `scryrs proposals assist group <PATH> --model <MODEL_ID> [--max-input-chars <N>] [--max-hotspots <N>] [--max-graph-nodes <N>] [--max-proposals <N>] [--max-documents <N>] [--max-output-tokens <N>] [--timeout-ms <N>] [--write]`

The command group SHALL appear in human-readable help, feature-aware machine-readable `--help-json`, snapshots, and CLI documentation. Command help for the assist surface SHALL expose the default `EvidencePackConfig` budgets (`max_input_chars = 32000`, `max_hotspots = 50`, `max_graph_nodes = 100`, `max_proposals = 20`, `max_documents = 20`) and SHALL explain that returned evidence must resolve to cited input-pack entries.

#### Scenario: Feature-disabled builds omit the assist surface

- **GIVEN** the CLI is built without the `curator-llm` feature
- **WHEN** a caller invokes `scryrs --help` or `scryrs --help-json`
- **THEN** the `proposals` command still exposes `list`, `accept`, and `reject`
- **AND** no `assist` subcommand is advertised

#### Scenario: Feature-enabled builds advertise assist commands and required model selection

- **GIVEN** the CLI is built with the `curator-llm` feature
- **WHEN** a caller invokes `scryrs proposals --help`
- **THEN** the output lists `assist draft` and `assist group`
- **AND** both assist commands require `--model <MODEL_ID>`
- **AND** the help text shows the default EvidencePack budgets and citation rule

#### Scenario: Help-json represents the conditional assist surface

- **WHEN** a caller invokes `scryrs --help-json`
- **THEN** the JSON surface document contains a grouped `proposals` command entry
- **AND** that entry exposes nested `list`, `accept`, and `reject` subcommands in all builds
- **AND** it exposes nested `assist`, `draft`, and `group` entries only when the CLI is built with `curator-llm`

## ADDED Requirements

### Requirement: Draft assist emits reviewable replacement proposals without mutating the source proposal

`scryrs proposals assist draft <PATH> <ID>` SHALL load `.scryrs/proposals/{id}.json`, require that it deserialize as a valid `ProposalDocument`, and invoke model-assisted drafting through the bounded `scryrs-curator-llm` contract. The command SHALL require `--model <MODEL_ID>`, build its `EvidencePack` from the source proposal evidence by default, and widen draft evidence scope only when `--include-hotspots` and/or `--include-graph` are explicitly provided. The command SHALL emit a validated replacement `ProposalDocument` JSON to stdout by default. It SHALL write a new `.scryrs/proposals/{newId}.json` file only when `--write` is provided. The source proposal file SHALL remain unchanged, and the contract SHALL NOT add `replaces` or source-link lifecycle fields to `ProposalDocument`.

#### Scenario: Draft assist previews a replacement proposal on stdout

- **GIVEN** a valid pending proposal and a valid bounded EvidencePack
- **WHEN** a caller invokes `scryrs proposals assist draft <PATH> <ID> --model demo-model`
- **THEN** the command exits with code `0`
- **AND** stdout is a validated replacement `ProposalDocument` JSON document
- **AND** no new proposal file is written
- **AND** the source `.scryrs/proposals/{id}.json` file is not mutated

#### Scenario: Draft assist writes a new content-addressed inbox proposal only when requested

- **GIVEN** a valid pending proposal and a successful model-assisted draft
- **WHEN** a caller invokes `scryrs proposals assist draft <PATH> <ID> --model demo-model --write`
- **THEN** the command exits with code `0`
- **AND** a new `.scryrs/proposals/{newId}.json` file is created for the replacement draft
- **AND** `{newId}` is derived from the replacement proposal content rather than reused from the source proposal
- **AND** the source `.scryrs/proposals/{id}.json` file is unchanged

### Requirement: Group assist emits review-only semantic grouping proposal candidates

`scryrs proposals assist group <PATH>` SHALL load deterministic graph and hotspot artifacts from the repository path, build a bounded `EvidencePack`, and invoke model-assisted grouping through `scryrs-curator-llm`. The command SHALL require `--model <MODEL_ID>`. It SHALL emit zero or more validated `semantic_graph_grouping` `ProposalDocument` candidates to stdout by default. It SHALL write new `.scryrs/proposals/{proposalId}.json` files only when `--write` is provided. Group assist SHALL NOT mutate `.scryrs/graph.json`, `.scryrs/routes.json`, `.scryrs/accepted/`, or `.scryrs/rejected/`.

#### Scenario: Group assist returns review-only grouping candidates

- **GIVEN** deterministic graph nodes and hotspot evidence exist for the repository
- **WHEN** a caller invokes `scryrs proposals assist group <PATH> --model demo-model`
- **THEN** stdout contains zero or more validated `semantic_graph_grouping` proposal candidates
- **AND** each candidate preserves exact `sourceNodeIds` from the evidence pack
- **AND** no graph or route artifact is mutated

#### Scenario: Group assist writes only new pending proposal files

- **GIVEN** a successful grouping run that yields one or more valid candidates
- **WHEN** a caller invokes `scryrs proposals assist group <PATH> --model demo-model --write`
- **THEN** each written artifact is a new `.scryrs/proposals/{proposalId}.json` file
- **AND** no files under `.scryrs/accepted/`, `.scryrs/rejected/`, `.scryrs/graph.json`, or `.scryrs/routes.json` are created, modified, or deleted

### Requirement: Assist failures are fail-whole-run and leave no partial writes

Malformed model output, unknown citations, unknown source node IDs, empty evidence, content-shape mismatch, over-budget input or output, missing required deterministic inputs, unavailable provider integration, or write failures SHALL fail the entire assist run. The assist commands SHALL NOT return partial success sets, SHALL NOT skip invalid candidates, and SHALL NOT leave partial proposal inbox writes behind.

#### Scenario: One invalid grouping candidate aborts the entire grouping run

- **GIVEN** a grouping response contains multiple candidates
- **AND** one candidate references unknown evidence or an unknown source node ID
- **WHEN** `scryrs proposals assist group <PATH> --model demo-model --write` validates the response
- **THEN** the command exits non-zero
- **AND** no new `.scryrs/proposals/*.json` file is written from that run
- **AND** stdout does not emit a partial success result

#### Scenario: Invalid draft output fails before any proposal file is created

- **GIVEN** a draft response is malformed, uncited, over-budget, or changes target/content shape
- **WHEN** a caller invokes `scryrs proposals assist draft <PATH> <ID> --model demo-model --write`
- **THEN** the command exits non-zero
- **AND** no replacement proposal file is created
- **AND** the source proposal file remains unchanged
- **AND** no accepted, rejected, graph, route, or docs artifacts are mutated

#### Scenario: Missing provider integration fails loudly without mutation

- **GIVEN** the assist surface is built but no usable provider-backed `ModelClient` integration is available
- **WHEN** a caller invokes an assist command
- **THEN** the command exits non-zero with an unavailable diagnostic
- **AND** no proposal, accepted, rejected, graph, route, or docs artifact is created or mutated
