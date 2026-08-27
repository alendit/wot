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

Add `--hooks` to install PreToolUse rewrites that replace eligible broad shell
reads with `wot` outlines before execution:

```bash
wot setup --hooks
wot setup --claude --hooks
```

For Codex, `--hooks` writes `.codex/hooks.json` or `~/.codex/hooks.json`. With
`--claude`, it also writes `.claude/settings.json` or
`~/.claude/settings.json`.

## Usage

```bash
wot [OPTIONS] <path>...
```

Run `wot --version` to report the installed package version.

Each path may be a supported file or a directory. Directory inputs are traversed
automatically to a default depth of `3`, with the target directory at depth `0`.
The Markdown output nests each discovered file's outline or short verbatim
content into a deterministic file tree. When a directory is not expanded because
of the depth limit, its tree node says so explicitly.

To estimate the installed Codex hook's token overhead and observational savings
from rollout transcripts, run the dependency-free bulk analyzer:

```bash
python3 scripts/analyze_wot_hook.py ~/.codex/sessions/2026
```

Use `--format json` for machine-readable output. The report uses exact recorded
tool-output token counts when available and labels its pre/post and substitution
figures as estimates because transcripts do not contain the no-hook
counterfactual. Pass `--record-cutoff 2026-08-22T07:00:00Z` to pin a
reproducible historical snapshot while sessions continue to be appended.
Use `--compare-hook-text 'previous exact reminder text'` to compare two wording
variants in equal periods around the first appearance of the primary
`--hook-text`.

For the silent rewrite hook, audit observed rewrites and same-file recovery
reads directly:

```bash
python3 scripts/analyze_wot_hook.py --audit-rewrites ~/.codex/sessions/2026/08/24
```

This mode detects recorded non-wot commands whose output contains wot file
headers; it does not rely on model-visible hook messages.

`wot --help` is the authoritative command reference for flags and defaults.
Use `wot --list-supported` to see the exact recognized language ids, file
extensions, special filenames, and parser backends for the installed build.

Examples:

```bash
wot .
wot --walk-depth 4 src tests
wot README.md notes.org src/lib.rs src/main.py data.json config.yaml Cargo.toml Dockerfile
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
- wot [L1-L122]
  - Quickstart [L8-L45]
  - Usage [L46-L106]
  - Release [L107-L119]
  - Supported Inputs [L120-L122]
```

By default, `wot` prints recognized files of 40 lines or fewer verbatim. Larger files print compact Markdown outline items such as `- def run [L10-L18]`; pass `--min-lines 0` to force outlines for every file. Same-line JSON sections may include columns such as `L1:C2-L1:C7`.

Recursive discovery honors `.gitignore`, `.ignore`, repository excludes, and
global gitignore rules while still considering non-ignored hidden paths such as
`.github`. It never enters `.git` or follows symlinks, and silently skips
unsupported descendants. Discovered `.env*` files always use redacting outline
mode rather than verbatim output. `--language` remains available for explicit
files and stdin, but cannot be combined with a directory input.

JSON output retains the top-level `files` and `errors` arrays and adds a
`directories` array. Directory entries describe the nested path tree and visible
depth truncation; successful file details remain in `files`.

Useful options:

- `--format markdown|json` chooses text or machine-readable output.
- `--header` prints file headers in Markdown output.
- `--walk-depth N` limits recursive filesystem traversal; default is `3`.
- `--max-depth N` limits nesting inside each file outline; default is `3`.
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
- `wot setup --hooks` installs PreToolUse command-rewrite hooks for the selected setup targets.
- `wot setup -g --hooks` installs the same hooks in user-level Codex and agent roots.
- `wot setup --claude --hooks` installs both the agent skill and hooks for Claude.

The hook runs `wot hook-check` before supported tool use. For explicit supported
files, it rewrites whole-file `cat`, `nl`, and `sed -n '1,$p'` displays to
`wot --header`. The normal 40-line threshold keeps small files verbatim. The
hook handles eligible segments in simple `&&`, `;`, or newline command lists.
It preserves every bounded numeric range and never rewrites `AGENTS.md`,
`CLAUDE.md`, or `SKILL.md`. Pipelines, redirects, shell expansions,
substitutions, transformations, `head`/`tail`, stdin, and unsupported files also
remain unchanged.

Codex receives the rewritten Bash input directly, so there is no repeated
model-visible reminder. Claude Bash calls use the same policy; full-file Claude
`Read` calls retain the short advisory because that tool cannot be replaced by
a Bash command rewrite. The hook never blocks or asks for approval.

## Release

Releases are published from GitHub Actions when a semver-like tag is pushed:

```bash
git tag 0.2.0
git push origin main 0.2.0
```

The publish workflow uses crates.io Trusted Publishing through GitHub Actions
OIDC. Configure crates.io to trust this repository and
`.github/workflows/publish.yml` before relying on automated releases.

## Supported Inputs

Supported inputs include Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Org mode, Python, JSON, YAML, TOML, INI, `.env`, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks. CSV/TSV and NDJSON/JSONL are intentionally not supported yet.
