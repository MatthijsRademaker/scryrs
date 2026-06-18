---
description: Propose a new change - create it and generate all artifacts in one step
swarm: true
agent_types:
  - swarm-refinement-room
---

Propose a new change - create the change and generate all artifacts in one step.

I'll create a change with artifacts:

- proposal.md (what & why)
- design.md (how)
- tasks.md (implementation steps)

When ready to implement, run /opsx-apply

$ARGUMENTS

---

**Input**: `$ARGUMENTS` is the change name for manual non-swarm use. In swarm context, `DEV_SWARM_TASK_ID` takes precedence — the harness always sets it.

**Important naming rule**: Openspec change names must start with a letter. Task UUIDs may start with a digit (e.g. `31b9047a-...`). To satisfy this constraint, swarm task IDs are always prefixed with `task-`.

**Steps**

1. **Determine the change name**

   Check the authoritative source first:
   ```bash
   echo "DEV_SWARM_TASK_ID=${DEV_SWARM_TASK_ID:-}"
   ```

   - If `DEV_SWARM_TASK_ID` is non-empty: use `task-${DEV_SWARM_TASK_ID}` as the change name.
     The `task-` prefix is required because openspec change names must start with a letter,
     and task UUIDs may start with a digit.
   - If `DEV_SWARM_TASK_ID` is empty: use `$ARGUMENTS` directly (if non-empty). This supports manual use on the host.
   - If BOTH are empty: fail immediately — do NOT prompt or ask.

   **CRITICAL**: The change name MUST be `task-<TASK_ID>` in swarm context. The commit_artifacts hook expects artifacts at `openspec/changes/task-<TASK_ID>/`. Using any other name orphans the artifacts.

2. **Create the change directory**

   ```bash
   openspec new change "<name>"
   ```

   This creates a scaffolded change at `openspec/changes/<name>/` with `.openspec.yaml`.

3. **Get the artifact build order**

   ```bash
   openspec status --change "<name>" --json
   ```

   Parse the JSON to get:
   - `applyRequires`: array of artifact IDs needed before implementation (e.g., `["tasks"]`)
   - `artifacts`: list of all artifacts with their status and dependencies

4. **Create artifacts in sequence until apply-ready**

   Use the **TodoWrite tool** to track progress through the artifacts.

   Loop through artifacts in dependency order (artifacts with no pending dependencies first):

   a. **For each artifact that is `ready` (dependencies satisfied)**:
   - Get instructions:
     ```bash
     openspec instructions <artifact-id> --change "<name>" --json
     ```
   - The instructions JSON includes:
     - `context`: Project background (constraints for you - do NOT include in output)
     - `rules`: Artifact-specific rules (constraints for you - do NOT include in output)
     - `template`: The structure to use for your output file
     - `instruction`: Schema-specific guidance for this artifact type
     - `outputPath`: Where to write the artifact
     - `dependencies`: Completed artifacts to read for context
   - Read any completed dependency files for context
   - Create the artifact file using `template` as the structure
   - Apply `context` and `rules` as constraints - but do NOT copy them into the file
   - Show brief progress: "Created <artifact-id>"

   b. **Continue until all `applyRequires` artifacts are complete**
   - After creating each artifact, re-run `openspec status --change "<name>" --json`
   - Check if every artifact ID in `applyRequires` has `status: "done"` in the artifacts array
   - Stop when all `applyRequires` artifacts are done

   c. **If an artifact requires user input** (unclear context):
   - Use **AskUserQuestion tool** to clarify
   - Then continue with creation

5. **Show final status**
   ```bash
   openspec status --change "<name>"
   ```

**Output**

After completing all artifacts, summarize:

- Change name and location
- List of artifacts created with brief descriptions
- What's ready: "All artifacts created! Ready for implementation."
- Prompt: "Run `/opsx-apply` to start implementing."

**Artifact Creation Guidelines**

- Follow the `instruction` field from `openspec instructions` for each artifact type
- The schema defines what each artifact should contain - follow it
- Read dependency artifacts for context before creating new ones
- Use `template` as the structure for your output file - fill in its sections
- **IMPORTANT**: `context` and `rules` are constraints for YOU, not content for the file
  - Do NOT copy `<context>`, `<rules>`, `<project_context>` blocks into the artifact
  - These guide what you write, but should never appear in the output

**Guardrails**

- Create ALL artifacts needed for implementation (as defined by schema's `apply.requires`)
- Always read dependency artifacts before creating a new one
- If context is critically unclear, ask the user - but prefer making reasonable decisions to keep momentum
- If a change with that name already exists, ask if user wants to continue it or create a new one
- After creating the change directory (step 2), verify `openspec/changes/<name>/.openspec.yaml` exists at exactly the expected path. In swarm context the expected path is `openspec/changes/task-<TASK_ID>/`. If it doesn't match, stop and fix the mismatch — do not proceed with artifact creation.
- Verify each artifact file exists after writing before proceeding to next
- **Specs artifact must pass `--strict` validation**: The specs `outputPath` is `specs/**/*.md`. You MUST create proper capability folders under `specs/` with delta headers (`## MODIFIED`, `## ADDED`, or `## REMOVED`) and at least one scenario block — NEVER create a bare `README.md` or a flat file claiming "no spec changes." Even for pure bug fixes or refactors, every requirement referenced in the proposal SHALL have a corresponding capability folder with a delta header. For a bug fix that doesn't change capability semantics, create a `## MODIFIED` delta that documents the current expected behavior plus a verification scenario proving the fix. Use `openspec instructions specs` output as the authoritative structure guide.
