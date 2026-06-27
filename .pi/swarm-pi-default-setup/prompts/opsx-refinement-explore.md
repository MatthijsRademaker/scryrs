# Opsx Refinement Exploration

Use this prompt when the Refinement Room runs the `explore` phase.

Investigate the task as an OpenSpec/opsx product-owner pass. Your job is not to publish artifacts. Your job is to return an evidence-backed JSON dossier that gives later room participants a shared factual base.

## Required discipline

- Inspect the task text, relevant repository files, project docs, existing OpenSpec specs, and active changes before asserting affected areas.
- Cite every affected area with concrete evidence from the task, repository, project docs, OpenSpec artifacts, or inspected canonical artifacts.
- Separate goals from non-goals.
- List assumptions and open questions explicitly.
- Recommend participants and model-escalation hints only when evidence justifies them.
- Do not mutate source files or canonical OpenSpec files.
- Return only the JSON object requested by the room controller.

## Required Dossier Fields

All of the following fields MUST be present and non-empty in the submitted dossier:

- `problemFraming` — concise non-empty string describing the core problem.
- `goals` — array of at least one non-empty goal string.
- `nonGoals` — array of at least one non-empty non-goal string.
- `assumptions` — array of at least one non-empty assumption string.
- `openQuestions` — array of at least one non-empty open question string.
- `affectedAreas` — array of at least one area object, each with `area` (non-empty string) and `evidence` (array of at least one non-empty citation string).
- `consultedSources` — array of at least one source object, each with `kind`, `reference`, and `note` (all non-empty strings). Must include the task input.
- `suggestedParticipants` — array of at least one non-empty participant ID string.
- `acceptanceCriteria` — array of at least one non-empty criterion string.
- `initialProposalSketch` — concise non-empty string sketching the proposed solution approach.

## Dossier quality bar

The dossier must be good enough for proposal synthesis to produce `proposal.md`, `design.md`, `tasks.md`, and `specs/**/spec.md` without hidden conversation history.
