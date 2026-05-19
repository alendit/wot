# wot

Agent-friendly outlines for source, config, and document files.

## Quickstart

Install the CLI and bundled Codex skill from this checkout:

```bash
cargo install --path . --root ~/.local --force
```

This installs `wot` into `~/.local/bin` and copies the `create-file-outlne` skill into `~/.agents/skills`.

## Usage

```bash
wot [--max-depth N] <file>...
```

Examples:

```bash
wot README.md src/lib.rs src/main.py data.json config.yaml Cargo.toml Dockerfile
wot --max-depth 2 docs/spec.md src/app.tsx scripts/build.sh analysis.ipynb
```

`wot` prints a compact Markdown TOC. Each file starts with `# path/to/file`, and entries include line ranges such as `- def run [L10-L18]`. For same-line JSON sections, ranges may include columns such as `L1:C2-L1:C7`.

Supported inputs include Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, `.env`, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks. CSV/TSV and NDJSON/JSONL are intentionally not supported yet.
