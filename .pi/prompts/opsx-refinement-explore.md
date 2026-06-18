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

## Dossier quality bar

The dossier must be good enough for proposal synthesis to produce `proposal.md`, `design.md`, `tasks.md`, and `specs/**/spec.md` without hidden conversation history.
