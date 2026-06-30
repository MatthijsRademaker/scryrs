## ADDED Requirements

### Requirement: CLI reference distinguishes workspace bootstrap from repository packaging

The CLI reference docs SHALL describe consumer live setup in terms of workspace-local bootstrap artifacts and SHALL explicitly distinguish those artifacts from the scryrs repository's own packaging and maintainer-oriented Docker files.

#### Scenario: CLI reference points users to `.scryrs/` bootstrap artifacts

- **WHEN** a reader follows the live setup guidance in the CLI reference docs
- **THEN** they are directed to `.scryrs/.env` and `.scryrs/compose.yml` as the consumer-facing live bootstrap artifacts
- **AND** they are not told that checking out the scryrs source repository is a prerequisite for ordinary consumer live setup

#### Scenario: CLI reference names the external network endpoint contract

- **WHEN** a reader inspects the documented live ingest URL and networking guidance
- **THEN** the docs describe the live server joining an existing external agent network
- **AND** the documented in-network endpoint is `http://scryrs:8081`
- **AND** the docs explain that this contract is for container-attached agents on that shared network
