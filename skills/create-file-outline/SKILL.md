---
name: create-file-outline
description: Use before broad file reads when the user asks to inspect, summarize, navigate, or understand source, config, docs, notebooks, or multiple files; use for compact file outlines, table of contents, TOCs, section maps, line ranges, and agent-friendly overviews with the wot CLI.
---

# Create File Outline

Use `wot` to produce compact Markdown outlines of supported files for agent context. It supports Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, `.env`, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.

## Install

Install or refresh the tool and this skill from the local checkout:

```bash
cd /Users/dimitrivorona/projects/rust/wot
cargo install --path . --root /Users/dimitrivorona/.local --force
```

During release builds, Cargo copies this skill to `/Users/dimitrivorona/.agents/skills/create-file-outline/SKILL.md`. Set `WOT_SKIP_SKILL_INSTALL=1` to skip that side effect.

## Usage

Run:

```bash
wot [OPTIONS] <file>...
```

Prefer `wot --help` for current flag details and `wot --list-supported` for the
installed language ids, extensions, special filenames, and parser backends. The
notes below capture the behavior this skill relies on, but the CLI help is the
source of truth.

Defaults:

- `--max-depth` defaults to `3`.
- `--max-items` defaults to `200`.
- `--min-lines` defaults to `40`; recognized files at or below that line count print verbatim.
- Inputs are explicit files by default; `--stdin` reads stdin and requires `--language`.
- Output is Markdown by default. Use `--format json` for machine-readable output.
- Markdown file headers are off by default. Use `--header` to print `# path/to/file`.
- Outline items render as `- label [Lstart-Lend]`.
- Same-line JSON siblings can use `Lx:Cy-Lx:Cz` ranges.
- `.env` secret-like values are redacted in labels.
- `--list-supported` prints recognized languages/extensions and parser backend.
- `--lenient` enables safe partial parsing for YAML, TOML, HCL, and XML.

## Workflow

1. Before opening many supported files or a large supported file with `cat`, `sed`, or an editor, run `wot` first to get the structure and line ranges.
2. Prefer plain `wot <file>` for small-file verbatim context and ranged outlines for larger files.
3. Use `wot --min-lines 0` when line-numbered outlines are required even for small files.
4. Use `wot --max-depth 2` for quick overviews and `--max-depth 3` or higher when the user needs more structure.
5. Read exact source ranges with normal file tools after `wot` identifies the relevant sections.
6. Pass multiple files in the order the user should read them.
7. If a file fails, read stderr; `wot` continues processing later files and exits nonzero when any file fails.
8. For unsupported file types, use `--language LANG` when the intended language is known.

## Examples

```bash
wot README.md src/lib.rs src/main.py package.json config.yaml Cargo.toml Dockerfile
wot --max-depth 2 docs/spec.md src/app.tsx scripts/build.sh analysis.ipynb
wot --format json --min-lines 0 src/lib.rs
wot --stdin --language python --min-lines 0
```
