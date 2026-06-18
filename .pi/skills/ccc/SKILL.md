---
name: ccc
description: "This skill should be used after you queried the docs to find code, files, or understand where functionality lives in the codebase. Prefer `ccc search` over `grep` for exploring the codebase — it does semantic search, understands concepts (not just text patterns), and returns relative file paths with line ranges. Use it to discover relevant files before reading them. Load this skill before running any `ccc` commands. Trigger phrases: 'find code', 'search the codebase', 'where is', 'look for', 'ccc', 'codesearch'."
---

# ccc - Semantic Code Search & Indexing

`ccc` is the CLI for CocoIndex Code, providing semantic search over the current codebase and index management.

## Runtime Context

There are two execution contexts for the ccc embedding backend. Pick exactly one `OPENAI_API_BASE` URL first, then configure `~/.cocoindex_code/global_settings.yml` with the matching value.

- **Local developer runtime:** `http://127.0.0.1:8081/v1` — ccc runs natively on the host, connects directly to llama.cpp on the same machine
- **Docker/container runtime:** `http://host.docker.internal:8081/v1` — ccc (or the agent calling it) runs inside a Docker container and needs to reach the host's baremetal llama.cpp instance

On macOS both resolve identically. Inside Linux Docker containers, only `host.docker.internal` reaches the host.

**Hard rule:** never use the Docker base URL when running on the host, or vice versa. The daemon must connect to the correct endpoint for its runtime environment.

## Embedding Configuration

The embedding backend is configured in `~/.cocoindex_code/global_settings.yml`. The setup below uses LiteLLM (built into `cocoindex-code`) to connect to a baremetal [llama.cpp](https://github.com/ggml-org/llama.cpp) server running an embedding model:

```yaml
embedding:
  provider: litellm
  model: openai/embedder
  min_interval_ms: 5

envs:
  OPENAI_API_BASE: <BASE_URL>   # from Runtime Context above
  OPENAI_API_KEY: dummy
```

llama.cpp is started as:

```bash
llama-server \
  -hf nomic-ai/nomic-embed-code-GGUF:Q8_0 \
  --host 127.0.0.1 \
  --port 8081 \
  --embedding \
  --ctx-size 2048 \
  --n-gpu-layers 999
```

After configuring, run `ccc doctor` to verify the model responds, then `ccc index` to build the index.

## Searching the Codebase

To perform a semantic search:

```bash
ccc search <query terms>
```

The query should describe the concept, functionality, or behavior to find, not exact code syntax. For example:

```bash
ccc search database connection pooling
ccc search user authentication flow
ccc search error handling retry logic
```

### Filtering Results

- **By language** (`--lang`, repeatable): restrict results to specific languages.

  ```bash
  ccc search --lang python --lang markdown database schema
  ```

- **By path** (`--path`): restrict results to a glob pattern relative to project root. If omitted, defaults to the current working directory (only results under that subdirectory are returned).

  ```bash
  ccc search --path 'src/api/*' request validation
  ```

### Pagination

Results default to the first page. To retrieve additional results:

```bash
ccc search --offset 5 --limit 5 database schema
```

If all returned results look relevant, use `--offset` to fetch the next page — there are likely more useful matches beyond the first page.

### Working with Search Results

Search results include file paths and line ranges. To explore a result in more detail:

- Use the editor's built-in file reading capabilities (e.g., the `Read` tool) to load the matched file and read lines around the returned range for full context.
- When working in a terminal without a file-reading tool, use `sed -n '<start>,<end>p' <file>` to extract a specific line range.
