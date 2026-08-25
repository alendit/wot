---
name: create-file-outline
description: Use when you need to decide which parts of source, config, docs, notebooks, or candidate files are worth reading before reading them in detail; use for compact outlines, TOCs, section maps, line ranges, overviews, comparisons, navigation maps, and architecture understanding with the wot CLI.
---

# Create File Outline

Use `wot` to produce compact Markdown outlines of supported files for agent context. It supports Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Org mode, Python, JSON, YAML, TOML, INI, `.env`, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.

## Install

Install or refresh the tool from the local checkout:

```bash
cd /Users/dimitrivorona/projects/rust/wot
cargo install --path . --root /Users/dimitrivorona/.local --force
```

Install or refresh this skill in the current project:

```bash
wot setup
```

Use `wot setup -g` for `/Users/dimitrivorona/.agents/skills/create-file-outline/SKILL.md`.
Add `--claude` to also install into `.claude/skills/create-file-outline/SKILL.md`
or `/Users/dimitrivorona/.claude/skills/create-file-outline/SKILL.md`.

Add `--hooks` to install PreToolUse rewrites that replace eligible broad shell
reads with `wot` outlines before execution:

```bash
wot setup --hooks
wot setup --claude --hooks
```

For Codex, `--hooks` writes `.codex/hooks.json` or
`/Users/dimitrivorona/.codex/hooks.json`. With `--claude`, it also writes
`.claude/settings.json` or `/Users/dimitrivorona/.claude/settings.json`.

## Usage

Run:

```bash
wot [OPTIONS] <path>...
```

Prefer `wot --help` for current flag details and `wot --list-supported` for the
installed language ids, extensions, special filenames, and parser backends. The
notes below capture the behavior this skill relies on, but the CLI help is the
source of truth.

Defaults:

- `--max-depth` defaults to `3`.
- `--walk-depth` defaults to `3`; directory inputs recurse automatically with
  the target directory at depth `0`.
- `--max-items` defaults to `200`.
- `--min-lines` defaults to `40`; recognized files at or below that line count print verbatim.
- Inputs may be explicit files or directories; `--stdin` reads stdin and requires `--language`.
- Output is Markdown by default. Use `--format json` for machine-readable output.
- Markdown file headers are off by default. Use `--header` to print `# path/to/file`.
- Directory Markdown is an integrated tree with file summaries nested beneath
  each path and explicit markers when `--walk-depth` stops expansion.
- Directory discovery honors ignore rules, includes non-ignored hidden paths,
  skips `.git`, symlinks, and unsupported descendants, and is deterministically
  ordered.
- Outline items render as `- label [Lstart-Lend]`.
- Same-line JSON siblings can use `Lx:Cy-Lx:Cz` ranges.
- Discovered `.env*` files always use outline mode so secret-like values remain
  redacted in labels.
- JSON keeps `files` and `errors` and adds directory-tree metadata in
  `directories`.
- `--list-supported` prints recognized languages/extensions and parser backend.
- `--lenient` enables safe partial parsing for YAML, TOML, HCL, and XML.

Setup hooks:

- `wot setup --hooks` installs PreToolUse command-rewrite hooks for Codex.
- `wot setup --claude --hooks` also installs them for Claude.
- The hooks run `wot hook-check`.
- For explicit supported files, the hook rewrites whole-file `cat`, `nl`, and
  `sed -n '1,$p'` displays to `wot --header`; the normal 40-line threshold keeps
  small files verbatim.
- It handles eligible segments in simple `&&`, `;`, or newline command lists.
- It preserves bounded numeric ranges and never rewrites `AGENTS.md`,
  `CLAUDE.md`, or `SKILL.md`.
- It leaves pipelines, redirects, expansions, substitutions, transformations,
  `head`/`tail`, stdin, and unsupported files unchanged.
- Codex Bash rewrites add no model-visible reminder. Claude full-file `Read`
  calls retain the short advisory because they cannot be replaced with Bash.
- The hooks never block tool use, ask for approval, or replace these skill
  instructions.

## When To Use

Use `wot` when you need to decide what parts of one or more files are worth
reading before reading them in detail.

Strong signals:

- You are orienting in an unfamiliar file or repo.
- You are comparing projects or architectures.
- You have file candidates but do not yet know which sections matter.
- You are about to skim with broad `sed`, `cat`, or editor reads just to find
  structure.
- You need section, function, class, key, or cell names with line ranges before
  choosing exact reads.
- The user asks for an overview, summary, comparison, navigation map, or
  architecture understanding.

Skip `wot` when:

- You already know the exact small section to read.
- You are searching for a specific string or symbol; use `rg` first.
- The file is short enough that reading it verbatim is the outline.
- The task depends on exact full content rather than structure.

## Workflow

1. Use `wot` to reduce selection uncertainty and get structure with line ranges.
2. Read exact source ranges with normal file tools after `wot` identifies the
   relevant sections.
3. Use `wot --min-lines 0` when line-numbered outlines are required even for
   small files.
4. Use `wot --max-depth 2` for quick overviews and `--max-depth 3` or higher
   when the user needs more structure.
5. Use `wot DIRECTORY` for a repository or subtree overview, and raise
   `--walk-depth` only when visible depth-limit markers hide relevant paths.
6. Pass multiple file or directory roots in the order the user should read them.
7. If a file fails, read stderr; `wot` continues processing later files and exits
   nonzero when any file fails.
8. For unsupported explicit files, use `--language LANG` when the intended
   language is known. Do not combine `--language` with directory inputs.

## Upstream Or Comparison Workflow

For cloned upstream projects or project comparisons:

1. Run `wot --walk-depth 2 DIRECTORY` for a compact project map.
2. Raise the walk depth or pass selected files when the tree identifies relevant
   subtrees and entry points.
3. Read only the relevant ranges with normal file tools.

## Examples

```bash
wot .
wot --walk-depth 4 src tests
wot README.md notes.org src/lib.rs src/main.py package.json config.yaml Cargo.toml Dockerfile
wot --max-depth 2 docs/spec.md src/app.tsx scripts/build.sh analysis.ipynb
wot --format json --min-lines 0 src/lib.rs
wot --stdin --language python --min-lines 0
wot setup --claude
wot setup --claude --hooks
```
