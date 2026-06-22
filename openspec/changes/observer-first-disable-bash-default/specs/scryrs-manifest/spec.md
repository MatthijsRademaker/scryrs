## MODIFIED Requirements

### Requirement: Manifest enumerates supported harnesses with default and debug-only metadata

The manifest SHALL use a `supported_harnesses` array containing one entry per supported harness. The harnesses SHALL be `claude-code` and `pi`. Each entry SHALL include `referenceSource`, `interceptedTools`, and `capturedEventFamilies` describing default observer-first behavior. Each entry SHALL also include `debugOnlyInterceptedTools` and `debugOnlyCapturedEventFamilies` describing Bash/`CommandExecuted` support that is enabled only when `SCRYRS_DEBUG` is set to a non-empty value.

#### Scenario: Claude Code entry lists default observer-first tools

- **WHEN** the `claude-code` entry is inspected
- **THEN** `interceptedTools` contains exactly: `read`, `grep`, `glob`, `edit`, `write`, `notebookedit`, `web_search`, `web_fetch`
- **AND** `debugOnlyInterceptedTools` contains exactly: `bash`

#### Scenario: Pi entry lists default observer-first tools

- **WHEN** the `pi` entry is inspected
- **THEN** `interceptedTools` contains exactly: `read`, `ast_grep_search`, `lsp_navigation`, `edit`, `write`
- **AND** `debugOnlyInterceptedTools` contains exactly: `bash`

#### Scenario: Default event families exclude command capture

- **WHEN** each harness entry is inspected
- **THEN** `capturedEventFamilies` lists only event families emitted by default observer-first capture
- **AND** `debugOnlyCapturedEventFamilies` contains exactly: `CommandExecuted`

#### Scenario: Manifest explains debug-only Bash support as observational metadata

- **WHEN** a developer inspects either harness entry
- **THEN** the manifest makes clear that debug-only Bash support is observational metadata controlled by `SCRYRS_DEBUG`
- **AND** no field implies scryrs rewrites commands or becomes callable execution middleware
