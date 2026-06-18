# Opsx Refinement Proposal Synthesis

Use this prompt when the Refinement Room runs the `proposal_synthesis` phase.

Synthesize the validated room evidence into draft OpenSpec artifacts. You are not the publisher: return structured drafts and traceability; the deterministic artifact broker validates and writes canonical files.

## Required output discipline

- Produce complete draft contents for `proposal.md`, `design.md`, `tasks.md`, and at least one `specs/<capability>/spec.md` file.
- Use exact strict OpenSpec heading levels and labels only:
  - `## ADDED Requirements`
  - `## MODIFIED Requirements`
  - `## REMOVED Requirements`
  - `## RENAMED Requirements`
  - `### Requirement: <name>`
  - `#### Scenario: <name>`
- Every proposed requirement, task, and material design decision must trace to task input, the exploration dossier, accepted decisions, validated round outputs, blockers/overrides, or current artifact evidence.
- Include the current artifact base reference exactly as provided by the controller.
- List unresolved gaps instead of inventing unsupported scope.
- If repairing validation-format defects, you must preserve task substance, the existing accepted decisions, and traceability.
- You must not invent requirements or implementation scope merely to make placeholder/test tasks look substantive.
- Do not edit canonical files directly. Return only the JSON object requested by the room controller.

## Strict OpenSpec Spec Example

Every spec file MUST follow this exact structure. Use `SHALL` or `MUST` for normative requirements and include at least one `#### Scenario:` block per requirement.

```markdown
## ADDED Requirements

### Requirement: System SHALL validate input before processing
The system SHALL validate all user input before passing it to downstream processing. Invalid input MUST be rejected with a descriptive error that identifies the failing field.

#### Scenario: Valid input passes through
- **WHEN** the user submits a well-formed request
- **THEN** the system processes the request normally

#### Scenario: Invalid input is rejected
- **WHEN** the user submits a malformed request
- **THEN** the system returns a validation error with the specific field that failed
```

- Delta section headers are EXACT: `## ADDED Requirements`, `## MODIFIED Requirements`, `## REMOVED Requirements`, `## RENAMED Requirements`.
- Every requirement heading MUST be `### Requirement: <name>`.
- Every scenario heading MUST be `#### Scenario: <name>`.
- Normative keywords `SHALL` and `MUST` are REQUIRED in requirement bodies.
- Every scenario MUST contain `WHEN` and `THEN` bullets.

## Structured-Output Checklist

Before calling `submit_room_phase_result`, verify:

- [ ] `proposal` is non-empty markdown (Why, What Changes, Impact).
- [ ] `design` is non-empty markdown (Context, Goals/Non-Goals, Decisions, Risks).
- [ ] `tasks` is non-empty markdown with checkboxes.
- [ ] `specs` contains at least one file with non-empty content, valid delta headers, at least one `SHALL`/`MUST` requirement, and at least one `#### Scenario:` block.
- [ ] `traceability` is non-empty and references only bounded evidence (task, dossier, decisions, round outputs, artifact snapshot).
- [ ] `artifactBaseReference` is an exact copy of the room-provided value — do not modify or infer.
- [ ] No artifacts depend on hidden conversation history.

## Proposal quality bar

The drafts must be implementation-ready for `/opsx-apply`: scoped, testable, traceable, and free of hidden dependencies on the room transcript.
