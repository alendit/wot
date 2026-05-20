# wot

[![Crates.io](https://img.shields.io/crates/v/wot-cli.svg)](https://crates.io/crates/wot-cli)
[![CI](https://github.com/alendit/wot/actions/workflows/ci.yml/badge.svg)](https://github.com/alendit/wot/actions/workflows/ci.yml)

Agent-friendly outlines for source, config, and document files.

## Quickstart

Install the CLI from crates.io:

```bash
cargo install wot-cli --root ~/.local --force
```

This installs `wot` into `~/.local/bin`.

Or install directly from the GitHub repository:

```bash
cargo install --git https://github.com/alendit/wot --root ~/.local --force
```

Install or refresh the bundled agent skill in the current project:

```bash
wot setup
```

Use `wot setup -g` for `~/.agents`, and add `--claude` to also install into
`.claude` or `~/.claude`.

Add `--hooks` to install non-blocking PreToolUse reminders that nudge agents to
use `wot` before broad file reads when they are still deciding which sections
matter:

```bash
wot setup --hooks
wot setup --claude --hooks
```

For Codex, `--hooks` writes `.codex/hooks.json` or `~/.codex/hooks.json`. With
`--claude`, it also writes `.claude/settings.json` or
`~/.claude/settings.json`.

## Usage

```bash
wot [OPTIONS] <file>...
```

`wot --help` is the authoritative command reference for flags and defaults.
Use `wot --list-supported` to see the exact recognized language ids, file
extensions, special filenames, and parser backends for the installed build.

Examples:

```bash
wot README.md src/lib.rs src/main.py data.json config.yaml Cargo.toml Dockerfile
wot --max-depth 2 docs/spec.md src/app.tsx scripts/build.sh analysis.ipynb
wot --format json --min-lines 0 src/lib.rs
wot --stdin --language python --min-lines 0
wot --list-supported
wot setup --claude
wot setup --claude --hooks
```

Example output:

```console
$ wot --min-lines 0 README.md
- wot [L1-L67]
  - Quickstart [L5-L23]
  - Usage [L24-L67]
```

By default, `wot` prints recognized files of 40 lines or fewer verbatim. Larger files print compact Markdown outline items such as `- def run [L10-L18]`; pass `--min-lines 0` to force outlines for every file. Same-line JSON sections may include columns such as `L1:C2-L1:C7`.

Useful options:

- `--format markdown|json` chooses text or machine-readable output.
- `--header` prints file headers in Markdown output.
- `--max-items N` caps outline nodes; default is `200`.
- `--min-lines N` prints recognized files at or below `N` lines verbatim; default is `40`.
- `--language LANG` forces parsing as a supported language.
- `--stdin` reads stdin and requires `--language`.
- `--lenient` enables safe partial parsing for YAML, TOML, HCL, and XML.
- `--list-supported` prints languages, recognized names/extensions, and parser backend.

Setup options:

- `wot setup` installs or refreshes the bundled `create-file-outline` skill in `.agents`.
- `wot setup --claude` also installs the skill in `.claude`.
- `wot setup -g` uses user-level roots under `~/.agents`, `~/.codex`, and `~/.claude`.
- `wot setup --hooks` installs advisory PreToolUse hooks for the selected setup targets.

The hook runs `wot hook-check` before supported tool use. It never blocks,
asks for approval, rewrites tool input, or replaces normal skill guidance. When
the pending tool call looks like broad file exploration, it injects a short
model-visible reminder to use `rg --files` for candidates and `wot` for
outlines before broad reads.

## Release

Releases are published from GitHub Actions when a semver-like tag is pushed:

```bash
git tag 0.1.3
git push origin main 0.1.3
```

The publish workflow uses crates.io Trusted Publishing through GitHub Actions
OIDC. Configure crates.io to trust this repository and
`.github/workflows/publish.yml` before relying on automated releases.

## Supported Inputs

Supported inputs include Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, `.env`, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks. CSV/TSV and NDJSON/JSONL are intentionally not supported yet.
