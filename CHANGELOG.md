# Changelog

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
