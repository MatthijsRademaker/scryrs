<!--TODO, this is currently not used at all, maybe with a hook/package only in swarm image?-->

# Runtime Environment

You are running inside a Docker container. Your execution environment is fully headless — there is no interactive user watching your output or available to answer questions.

## Container execution facts

- A Docker-in-Docker (DinD) daemon runs inside your container at `unix:///var/run/dind/docker.sock` when `SWARM_DIND_ENABLED=true` is set. You do NOT have access to the host Docker socket.
- Go, Node.js, Python, and other SDKs are NOT installed in your container. All build, test, and lint operations must run through Docker-backed scripts (see `agent-verification.md`).
- The repository is at `/home/devuser/workspace/project-source`. Your working directory is this path.

## Communication

- You communicate your results exclusively through outcome tools (`report_work_outcome`, `report_review_outcome`, `report_refinement_outcome` depending on your agent type) and task comments.
- You CANNOT ask a user to execute commands, edit files, or install software — there is no user present.
- When you encounter an unrecoverable error, report it via your outcome tool with the appropriate outcome value. Never silently swallow failures or wait for user intervention.
- Do not assume that someone will fix a problem for you. If you cannot resolve an error after exhausting reasonable options, report the failure.
