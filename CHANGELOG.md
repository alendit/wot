# Changelog

## 0.6.0 - 2026-08-27

- Add a top-level `--version` flag that reports the installed `wot` package
  version, with process-level coverage and usage documentation.

## 0.5.0 - 2026-08-27

- Refocus the bundled `create-file-outline` skill on autonomous read routing:
  use `rg` for exact searches, direct reads for known narrow ranges, and `wot`
  before exploratory broad or multi-file reads.
- Remove installation and hook mechanics from the model-facing skill so its
  selection boundary and outline-first workflow are visible immediately.

## 0.4.0 - 2026-08-25

- Treat directory positional inputs as recursive outline roots by default, with
  deterministic integrated-tree Markdown, additive JSON directory metadata,
  gitignore-aware discovery, and visible `--walk-depth` truncation markers.
- Include non-ignored hidden supported files during directory discovery while
  skipping `.git`, symlinks, unsupported descendants, and verbatim `.env*`
  output that could expose secret-like values.
- Replace repeated advisory context on Codex Bash calls with deterministic
  PreToolUse rewrites from eligible broad explicit-file reads to compact `wot`
  outlines, including eligible segments in simple compound command lists.
- Narrow automatic rewrites to whole-file displays, preserve bounded ranges and
  instruction files, and add deterministic transcript auditing for silent
  rewrites followed by same-file recovery reads.

## 0.3.1 - 2026-08-09

- Shorten the advisory hook reminder to "Use wot for a file overview."

## 0.3.0 - 2026-05-23

- Add heading-only Org mode outline support for `.org` files and
  `--language org`.

## 0.2.0 - 2026-05-20

- Add `wot setup` to explicitly install the bundled skill into project-local or
  global agent skill roots, with optional Claude skill installation.
- Stop installing skills as a `cargo install` build side effect.
- Add the MIT license text file for downstream package consumers.

## 0.1.0 - 2026-05-20

Initial public release of `wot`.

- Compact Markdown outlines and machine-readable JSON output for source, config,
  and document files.
- Parser support for Rust, TypeScript/JavaScript, Go, C/C++, Java, Kotlin, C#,
  shell, Clojure, Emacs Lisp, Markdown, Python, JSON, YAML, TOML, INI, `.env`,
  XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.
- CLI controls for supported-language listing, opt-in headers, node budgets,
  small-file verbatim mode, forced language parsing, stdin input, and lenient
  partial parsing where supported.
- Bundled Codex skill for agent-friendly file outline workflows.
- GitHub Actions CI for formatting, Clippy, and tests.
