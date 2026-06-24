## ADDED Requirements

### Requirement: Repository root is available to all dashboard views

The dashboard backend SHALL expose the active repository root path through a `GET /api/meta` endpoint returning a JSON object containing a `repositoryPath` string, so that views which do not fetch the hotspot report can still resolve in-repo versus external subjects. The `repositoryPath` field SHALL also be present on the `HotspotsReport` returned by `GET /api/hotspots`.

#### Scenario: Meta endpoint returns repository root

- **WHEN** a client requests `GET /api/meta`
- **THEN** the response is `200 OK` with a JSON body containing `repositoryPath` set to the dashboard's configured repository root

#### Scenario: Hotspots report exposes repository root

- **WHEN** a client requests `GET /api/hotspots` and a report exists
- **THEN** the response body includes a `repositoryPath` field equal to the repository root used to generate the report

### Requirement: In-repo file subjects display as repo-relative paths

The dashboard SHALL display a `file` subject whose absolute path is located under the repository root as the path relative to the repository root, with the repository-root prefix removed and no leading separator.

#### Scenario: File inside the repository root

- **WHEN** the subject is `/Users/me/repos/scryrs/.devagent/doc_build/architecture.md` and the repository root is `/Users/me/repos/scryrs`
- **THEN** the displayed label is `.devagent/doc_build/architecture.md`

#### Scenario: Repository root with trailing separator

- **WHEN** the repository root is provided with a trailing path separator
- **THEN** the displayed relative path still has no leading separator

### Requirement: External file subjects display with an EXTERNAL badge and shortened tail

The dashboard SHALL display a `file` subject whose absolute path is not located under the repository root using an `EXTERNAL` badge marker followed by the last two segments of the path.

#### Scenario: File outside the repository root

- **WHEN** the subject is `/Users/me/repos/dignitas/cl-sessions/dignitas-agentic-docs/openspec/changes/agentic-docs-presentation/proposal.md` and the repository root is `/Users/me/repos/scryrs`
- **THEN** the displayed label shows an `EXTERNAL` badge followed by `agentic-docs-presentation/proposal.md`

#### Scenario: External path with a single segment

- **WHEN** an external subject path has fewer than two segments
- **THEN** the displayed label shows the `EXTERNAL` badge followed by the available segment(s) without error

### Requirement: Non-file and empty subjects are displayed unchanged

The dashboard SHALL render subjects whose kind is not `file`, and absent or lifecycle subjects, without applying path shortening or the EXTERNAL badge.

#### Scenario: Non-file subject kind

- **WHEN** the subject kind is `routing` with subject value `routing`
- **THEN** the displayed label is `routing` with no badge and no path stripping

#### Scenario: Absent subject

- **WHEN** an event has no subject value
- **THEN** the existing lifecycle placeholder is displayed unchanged

#### Scenario: Repository root unknown

- **WHEN** the repository root has not yet been resolved
- **THEN** the subject is displayed as its raw value without error

### Requirement: Full absolute path is revealed on demand

The dashboard SHALL make the full absolute subject path discoverable wherever a shortened label is shown. Every shortened label SHALL carry a hover tooltip containing the full absolute path, and the Subject detail view SHALL display the full absolute path in its heading.

#### Scenario: Hover reveals full path in lists and rows

- **WHEN** a shortened subject label is rendered in the Hotspots list/table, Events view, or Session detail view
- **THEN** hovering the label reveals the full absolute subject path via its `title` tooltip

#### Scenario: Subject detail heading shows full path

- **WHEN** the Subject detail view is opened for a shortened subject
- **THEN** the heading displays the full absolute subject path
