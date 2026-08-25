# Repository Instructions

## Tool Rejection Rule

- If an operation is rejected, treat that rejection as a hard stop for that action.
- Do not circumvent the rejection by using a different tool, path, or mechanism that happens to still allow the same action.
- Wait for explicit user permission or a clarified request before trying again.

## Approval And Allowlist Guidance

- Heavily prefer operations that already fit an existing allowlisted command prefix when doing routine git, uv, Home Assistant, Drone, Gogs, Portainer, or host-maintenance work.
- Prefer direct commands over wrapper shells.
- Do not use `zsh -lc` or similar wrappers unless shell features are actually required.
- For HTTP API work, prefer direct `curl` calls over `zsh -lc 'curl ...'`.
- Do not tunnel `curl` through `ssh` when the same HTTP call can be made directly.
- If a narrow allowlisted prefix already exists for the intended operation, use it instead of inventing a new command shape that will trigger another approval request.
- Treat broad host-scoped rules carefully.
- A broad `ssh <host>` rule should only be used for commands that are truly about that host.
- Prefer narrower ssh prefixes for specific services on that host such as container inspection or container exec.

## Git Write Safety

- Never run git write operations in parallel.
- Serialize `git add`, `git commit`, `git push`, and any other command that writes the index or refs.
- Do not bundle dependent git writes into the same parallel tool call.

## Context7 Documentation Rule

Use the `ctx7` CLI to fetch current documentation whenever the user asks about a library, framework, SDK, API, CLI tool, or cloud service. This includes API syntax, configuration, version migration, library-specific debugging, setup instructions, and CLI tool usage.

Do not use Context7 for refactoring, writing scripts from scratch, debugging business logic, code review, or general programming concepts.

Workflow:

1. Resolve library: `npx ctx7@latest library <name> "<user's question>"`.
2. Pick the best match by exact name, description relevance, snippet count, source reputation, and benchmark score.
3. Fetch docs: `npx ctx7@latest docs <libraryId> "<user's question>"`.
4. Answer using the fetched documentation.

You must call `library` first unless the user provides an ID in `/org/project` format. Do not run more than three Context7 commands per question. Do not include secrets in queries. Run Context7 CLI requests outside Codex's default sandbox; if a Context7 command fails with DNS or network errors, rerun it outside the sandbox instead of retrying inside the sandbox. If a quota error occurs, tell the user and suggest `npx ctx7@latest login` or setting `CONTEXT7_API_KEY`.

## Project Guidance

- `wot` is a small Rust CLI that emits compact, agent-friendly outlines for supported files.
- Keep the CLI shell in `src/cli.rs` thin: argument parsing, source IO, per-file error reporting, and exit behavior belong there; bounded directory discovery belongs in `src/discovery.rs`.
- Keep parsing behavior in `src/parsers/*`, the shared outline model in `src/model.rs`, source-position mapping in `src/source_map.rs`, and Markdown/JSON output formatting in `src/renderer.rs`.
- Preserve deterministic stdout. Output shape is user-facing behavior, so update parser, renderer, CLI tests, README examples, and the bundled skill together when changing it.
- Positional inputs may be explicit files or automatically recursive directory roots. Keep directory walking bounded, gitignore-aware, deterministic, and visibly truncated at its configured depth; do not add implicit current-directory scanning.
- The bundled Codex skill in `skills/create-file-outline/SKILL.md` is part of the product surface. Keep it aligned with CLI flags, supported languages, defaults, install behavior, and examples.
- Prefer focused tests near the changed behavior:
  - parser tests for syntax extraction and ranges
  - source-map tests for line and column math
  - renderer tests for output formatting
  - CLI tests for process behavior and stderr/stdout contracts

## Architecture Guidance

Use this section to evaluate decomposition, dependency direction, side-effect placement, interface design, and testability.

### Hard Constraints

- Shape work packages around useful stopping points that move toward the final direction. If work stopped after the package, the project should be better off than not building it; this does not need to hold for every internal slice.
- Treat available information explicitly. Make likely changes easy, keep uncertain decisions local and reversible, and model stable behavior directly. Use small module, adapter, function, data-mapping, or config boundaries to keep uncertain decisions contained.
- Add an abstraction only when the contract is real, stable enough to name, and makes the next likely change cheaper.
- Give each behavior a clear owning component.
- Keep dependencies flowing from unstable code toward stable code.
- Keep core policy isolated from side effects.
- Do not mix unrelated domains into one coordinating component.
- Do not create shared interfaces that implementations can only satisfy by narrowing behavior, ignoring requirements, or throwing.
- Make compatibility expectations explicit. Keep legacy handling or legacy paths only when a real compatibility requirement exists; otherwise remove obsolete paths by default and mention that cleanup explicitly.
- Do not add fallback paths, broad defensive handling, or error swallowing unless that layer can make a correct domain decision. Unexpected errors should surface to the top and fail in obvious ways.

### Required Self-Check For Design-Sensitive Changes

1. If this work package stopped here, would the project be better off than if it had not been built?
2. What final direction does this move toward?
3. Which decisions are likely to change, uncertain, or stable, and are uncertain ones local and reversible instead of hidden behind speculative abstraction?
4. What component owns this behavior, and why?
5. Does this change increase or reduce coupling?
6. Did any dependency start pointing the wrong way?
7. Could any side effect be moved outward into an adapter or shell?
8. Are the abstractions and interfaces semantically real?
9. What compatibility expectations apply, and were obsolete paths removed unless required?
10. What legacy code can we remove now?
11. Where are expected errors handled, and where do unexpected errors surface?
12. What tests prove the core behavior independently of the full system? If the design changed, would tests change narrowly, or would unrelated tests need rewrites?
