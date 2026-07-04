## ADDED Requirements

### Requirement: Proposal review commands do not trigger publish side effects

`scryrs proposals list`, `scryrs proposals accept`, and `scryrs proposals reject` SHALL remain review and ledger operations only. They SHALL NOT write generic Markdown publish output, SHALL NOT update Rspress `accepted-knowledge/` pages, and SHALL NOT modify Rspress `_nav.json`. Publication requires a separate explicit `scryrs publish ...` invocation.

#### Scenario: Accept remains ledger-only

- **GIVEN** a valid pending proposal exists under `.scryrs/proposals/`
- **WHEN** a caller invokes `scryrs proposals accept <PATH> <ID> ...`
- **THEN** the command writes only the accepted review decision under `.scryrs/accepted/`
- **AND** no generic Markdown publish output is created
- **AND** no Rspress docs output is created or updated

#### Scenario: Reject remains ledger-only

- **GIVEN** a valid pending proposal exists under `.scryrs/proposals/`
- **WHEN** a caller invokes `scryrs proposals reject <PATH> <ID> ...`
- **THEN** the command writes only the rejected review decision under `.scryrs/rejected/`
- **AND** no generic Markdown publish output is created
- **AND** no Rspress docs output is created or updated