---
name: create-file-outlne
description: Use when the user asks for a compact file outline, table of contents, TOC, section map, line ranges, or agent-friendly overview of source, config, or document files using the wot CLI.
---

# Create File Outline

Use `wot` to produce compact Markdown outlines of supported files for agent context. It supports Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, `.env`, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.

## Install

Install or refresh the tool and this skill from the local checkout:

```bash
cd /Users/dimitrivorona/projects/rust/wot
cargo install --path . --root /Users/dimitrivorona/.local --force
```

During release builds, Cargo copies this skill to `/Users/dimitrivorona/.agents/skills/create-file-outlne/SKILL.md`. Set `WOT_SKIP_SKILL_INSTALL=1` to skip that side effect.

## Usage

Run:

```bash
wot [--max-depth N] <file>...
```

Defaults:

- `--max-depth` defaults to `3`.
- Inputs must be explicit files, not directories or stdin.
- Output is Markdown TOC text.
- Each file starts with `# path/to/file`.
- Items render as `- label [Lstart-Lend]`.
- Same-line JSON siblings can use `Lx:Cy-Lx:Cz` ranges.
- `.env` secret-like values are redacted in labels.

## Workflow

1. Prefer `wot --max-depth 2` for a quick overview and `--max-depth 3` or higher when the user needs more structure.
2. Pass multiple files in the order the user should read them.
3. If a file fails, read stderr; `wot` continues processing later files and exits nonzero when any file fails.
4. For unsupported file types, either ask for a supported source file or use another inspection method.

## Examples

```bash
wot README.md src/lib.rs src/main.py package.json config.yaml Cargo.toml Dockerfile
wot --max-depth 2 docs/spec.md src/app.tsx scripts/build.sh analysis.ipynb
```
