# Swarm extensions

This directory contains project-scoped Pi extensions.

Infrastructure extensions (outcome reporting tools: `report_work_outcome`, `report_review_outcome`, `report_refinement_outcome`) are baked into the Docker image at `/home/devuser/.pi/agent/extensions/outcome-tools.ts` and are not stored in the workspace.

Primary-agent configuration now loads from the `@swarm/swarm-extension` package, not from a legacy project-local agent-config extension file.
