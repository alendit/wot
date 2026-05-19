use std::fs;
use std::path::PathBuf;

use clap::Parser;

use crate::error::{Error, Result};
use crate::parsers;
use crate::renderer::render_markdown;

const DEFAULT_MAX_DEPTH: usize = 3;

#[derive(Debug, Parser)]
#[command(
    name = "wot",
    about = "Create compact outlines from source, config, and document files",
    long_about = "Create compact Markdown table-of-contents style outlines from source, config, and document files.\n\nSupported inputs include Markdown, Python, JSON, YAML, TOML, INI, .env, XML/SVG/plist, HCL/Terraform, Dockerfile/Containerfile, and Jupyter notebooks.\n\nRanges are 1-based inclusive line ranges. When line-only ranges would be ambiguous, wot prints 1-based start-inclusive/end-exclusive columns as Lx:Cy-Lx:Cz."
)]
struct Args {
    #[arg(long, default_value_t = DEFAULT_MAX_DEPTH)]
    max_depth: usize,

    #[arg(required = true)]
    files: Vec<PathBuf>,
}

pub fn run() -> Result<()> {
    let args = Args::parse();
    run_with_args(args.files, args.max_depth)
}

pub fn run_with_args(files: Vec<PathBuf>, max_depth: usize) -> Result<()> {
    if files.is_empty() {
        return Err(Error::NoInput);
    }

    let mut had_error = false;
    let mut first_output = true;

    for path in files {
        match outline_path(&path, max_depth) {
            Ok(rendered) => {
                if !first_output {
                    println!();
                }
                print!("{rendered}");
                first_output = false;
            }
            Err(error) => {
                eprintln!("{error}");
                had_error = true;
            }
        }
    }

    if had_error {
        Err(Error::Parse {
            path: PathBuf::from("wot"),
            message: "one or more files failed".into(),
        })
    } else {
        Ok(())
    }
}

fn outline_path(path: &PathBuf, max_depth: usize) -> Result<String> {
    if path.is_dir() {
        return Err(Error::Directory { path: path.clone() });
    }

    let source = fs::read_to_string(path).map_err(|source| Error::Io {
        path: path.clone(),
        source,
    })?;
    let outline = parsers::parse_file(path, &source, max_depth)?;
    Ok(render_markdown(&outline))
}
