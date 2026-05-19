# wot

Agent-friendly outlines for Markdown, Python, and JSON files.

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
wot README.md src/main.py data.json
wot --max-depth 2 docs/spec.md src/app.py
```

`wot` prints a compact Markdown TOC. Each file starts with `# path/to/file`, and entries include line ranges such as `- def run [L10-L18]`. For same-line JSON sections, ranges may include columns such as `L1:C2-L1:C7`.
