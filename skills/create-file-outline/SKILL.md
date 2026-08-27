---
name: create-file-outline
description: Use wot before exploratory reading when candidate source, config, documentation, or notebook files are known but the relevant symbols or ranges are not. It produces compact outlines, file summaries, directory maps, and line ranges for choosing what to read next. Do not use it for exact string searches or already-known narrow ranges.
---

# Create File Outline

Use `wot` to decide which files and source ranges deserve detailed reading. An
outline is a navigation step, not a substitute for inspecting the exact code or
text needed as evidence.

## Route The Read

- If an exact string or symbol is known, use `rg`.
- If an exact narrow line range is known, read that range directly.
- If candidate files are known but the relevant parts are not, run
  `wot --header FILE...` before broad reads.
- If a repository or subtree needs orientation, run `wot DIRECTORY...` before
  opening files individually.
- If exact full-file content is genuinely required, read it directly.

Use `wot` before broad `cat`, `nl`, `sed`, or editor reads whose purpose is to
discover structure. This is especially useful when comparing several files or
when `rg` has found candidate files but not the relevant sections.

## Workflow

1. Discover candidate files with `rg`, `rg --files`, or existing project
   knowledge.
2. Outline the candidates together so their structures are easy to compare:

   ```bash
   wot --header src/cli.rs src/discovery.rs tests/cli_tests.rs
   ```

3. Use the reported line ranges to read only the sections needed for the task.
4. Return to `wot` with selected directories or a greater depth only when the
   first outline leaves a concrete navigation question unanswered.

## Useful Controls

- File inputs produce compact outlines; recognized files of 40 lines or fewer
  are shown verbatim by default.
- Directory inputs recurse automatically. `--walk-depth` controls directory
  depth and defaults to `3`; output visibly marks branches stopped by the
  limit.
- `--max-depth` controls outline nesting and defaults to `3`.
- `--header` labels file output, which is useful for multiple explicit files.
- `--min-lines 0` forces outline mode for small files.
- `--format json` is available when structured output is actually needed.
- `wot --list-supported` reports supported languages and filenames.
- `wot --help` is the source of truth for current CLI options.

## Examples

```bash
wot --header src/lib.rs src/model.rs src/renderer.rs
wot --walk-depth 2 src tests
wot --max-depth 2 docs/spec.md src/app.tsx
```
