# wot

Agent-friendly outlines for source, config, and document files.

## Quickstart

Install the CLI from this checkout:

```bash
cargo install --path . --root ~/.local --force
```

This installs `wot` into `~/.local/bin`.

Install or refresh the bundled agent skill in the current project:

```bash
wot setup
```

Use `wot setup -g` for `~/.agents`, and add `--claude` to also install into
`.claude` or `~/.claude`.

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

Supported inputs include Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#, shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, `.env`, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks. CSV/TSV and NDJSON/JSONL are intentionally not supported yet.
