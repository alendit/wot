use std::path::Path;

use crate::model::Language;

/// Rewrite broad, explicit-file shell reads into compact outlines.
///
/// The policy is deliberately conservative about shell semantics: it only
/// rewrites command-list segments whose arguments can be understood without a
/// shell. Pipelines, redirects, expansions, substitutions, and subshells are
/// left untouched.
pub(crate) fn rewrite_bash_command(command: &str) -> Option<String> {
    let command_list = split_command_list(command)?;
    let mut rewritten_any = false;
    let mut output = String::new();

    for (index, segment) in command_list.segments.iter().enumerate() {
        if index > 0 {
            output.push_str(&command_list.separators[index - 1]);
        }

        if let Some(rewritten) = rewrite_segment(segment) {
            output.push_str(&rewritten);
            rewritten_any = true;
        } else {
            output.push_str(segment.trim());
        }
    }

    rewritten_any.then_some(output)
}

struct CommandList {
    segments: Vec<String>,
    separators: Vec<String>,
}

fn split_command_list(command: &str) -> Option<CommandList> {
    let mut segments = Vec::new();
    let mut separators = Vec::new();
    let mut quote = None;
    let mut escaped = false;
    let mut segment_start = 0;
    let mut chars = command.char_indices().peekable();

    while let Some((index, ch)) = chars.next() {
        if escaped {
            escaped = false;
            continue;
        }

        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            continue;
        }

        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            }
            continue;
        }

        match ch {
            '\'' | '"' => quote = Some(ch),
            ';' | '\n' => {
                push_segment(command, segment_start, index, &mut segments)?;
                separators.push(if ch == ';' { "; " } else { "\n" }.to_owned());
                segment_start = index + ch.len_utf8();
            }
            '&' => {
                let Some((next_index, '&')) = chars.peek().copied() else {
                    return None;
                };
                chars.next();
                push_segment(command, segment_start, index, &mut segments)?;
                separators.push(" && ".to_owned());
                segment_start = next_index + 1;
            }
            '|' | '<' | '>' | '`' | '(' | ')' | '#' => return None,
            '$' if chars.peek().is_some_and(|(_, next)| *next == '(') => return None,
            _ => {}
        }
    }

    if quote.is_some() || escaped {
        return None;
    }
    push_segment(command, segment_start, command.len(), &mut segments)?;

    Some(CommandList {
        segments,
        separators,
    })
}

fn push_segment(command: &str, start: usize, end: usize, segments: &mut Vec<String>) -> Option<()> {
    let segment = command[start..end].trim();
    if segment.is_empty() {
        return None;
    }
    segments.push(segment.to_owned());
    Some(())
}

fn rewrite_segment(segment: &str) -> Option<String> {
    let words = shell_words(segment)?;
    let words = words.as_slice();
    let executable = Path::new(words.first()?).file_name()?.to_str()?;
    let arguments = &words[1..];

    let files = match executable {
        "cat" => positional_files(arguments, &[])?,
        "nl" => positional_files(
            arguments,
            &["-b", "-d", "-f", "-h", "-i", "-l", "-n", "-s", "-v", "-w"],
        )?,
        "sed" => sed_files(arguments)?,
        _ => return None,
    };
    if !supported_literal_files(&files) {
        return None;
    }

    let mut rewritten = String::new();
    rewritten.push_str("wot --header");
    for file in files {
        rewritten.push(' ');
        rewritten.push_str(&shell_quote(&file));
    }
    Some(rewritten)
}

fn positional_files(arguments: &[String], value_options: &[&str]) -> Option<Vec<String>> {
    let mut files = Vec::new();
    let mut literal_arguments = false;
    let mut index = 0;

    while index < arguments.len() {
        let argument = &arguments[index];
        if !literal_arguments && argument == "--" {
            literal_arguments = true;
        } else if !literal_arguments && value_options.contains(&argument.as_str()) {
            index += 1;
            arguments.get(index)?;
        } else if !literal_arguments && argument.starts_with('-') {
            if argument == "-" {
                return None;
            }
        } else {
            files.push(argument.clone());
        }
        index += 1;
    }

    (!files.is_empty()).then_some(files)
}

fn sed_files(arguments: &[String]) -> Option<Vec<String>> {
    let mut print_only = false;
    let mut expression = None;
    let mut files = Vec::new();
    let mut literal_arguments = false;
    let mut index = 0;

    while index < arguments.len() {
        let argument = &arguments[index];
        if !literal_arguments && argument == "--" {
            literal_arguments = true;
        } else if !literal_arguments && matches!(argument.as_str(), "-n" | "--quiet" | "--silent") {
            print_only = true;
        } else if !literal_arguments && matches!(argument.as_str(), "-E" | "-r") {
        } else if !literal_arguments && matches!(argument.as_str(), "-e" | "--expression") {
            index += 1;
            if expression.replace(arguments.get(index)?.clone()).is_some() {
                return None;
            }
        } else if !literal_arguments && argument.starts_with("--expression=") {
            if expression
                .replace(argument.strip_prefix("--expression=")?.to_owned())
                .is_some()
            {
                return None;
            }
        } else if !literal_arguments && argument.starts_with('-') {
            return None;
        } else if expression.is_none() {
            expression = Some(argument.clone());
        } else {
            files.push(argument.clone());
        }
        index += 1;
    }

    if !print_only || !is_full_print_expression(expression.as_deref()?) || files.is_empty() {
        return None;
    }
    Some(files)
}

fn is_full_print_expression(expression: &str) -> bool {
    let expression = expression.trim();
    let Some(range) = expression.strip_suffix('p') else {
        return false;
    };
    let Some((start, end)) = range.split_once(',') else {
        return false;
    };
    start.trim() == "1" && end.trim() == "$"
}

fn supported_literal_files(files: &[String]) -> bool {
    files.iter().all(|file| {
        file != "-"
            && !file.chars().any(|ch| "*?[]{}$~\n\r".contains(ch))
            && !is_instruction_file(Path::new(file))
            && Language::from_path(Path::new(file)).is_some()
    })
}

fn is_instruction_file(path: &Path) -> bool {
    path.file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| {
            matches!(
                name.to_ascii_uppercase().as_str(),
                "AGENTS.MD" | "CLAUDE.MD" | "SKILL.MD"
            )
        })
}

fn shell_words(command: &str) -> Option<Vec<String>> {
    let mut words = Vec::new();
    let mut current = String::new();
    let mut quote = None;
    let mut escaped = false;
    let mut word_started = false;

    for ch in command.chars() {
        if escaped {
            current.push(ch);
            escaped = false;
            word_started = true;
            continue;
        }

        if ch == '\\' && quote != Some('\'') {
            escaped = true;
            word_started = true;
            continue;
        }

        if let Some(active_quote) = quote {
            if ch == active_quote {
                quote = None;
            } else {
                current.push(ch);
            }
            word_started = true;
            continue;
        }

        match ch {
            '\'' | '"' => {
                quote = Some(ch);
                word_started = true;
            }
            ch if ch.is_whitespace() => {
                if word_started {
                    words.push(std::mem::take(&mut current));
                    word_started = false;
                }
            }
            _ => {
                current.push(ch);
                word_started = true;
            }
        }
    }

    if quote.is_some() || escaped {
        return None;
    }
    if word_started {
        words.push(current);
    }
    (!words.is_empty()).then_some(words)
}

fn shell_quote(word: &str) -> String {
    if !word.is_empty()
        && word
            .chars()
            .all(|ch| ch.is_ascii_alphanumeric() || "_@%+=:,./-".contains(ch))
    {
        return word.to_owned();
    }
    format!("'{}'", word.replace('\'', "'\"'\"'"))
}

#[cfg(test)]
mod tests {
    use super::rewrite_bash_command;

    #[test]
    fn rewrites_explicit_broad_file_displays() {
        let cases = [
            ("cat src/cli.rs", "wot --header src/cli.rs"),
            ("sed -n '1,$p' src/cli.rs", "wot --header src/cli.rs"),
            ("nl -ba 'docs/a file.md'", "wot --header 'docs/a file.md'"),
            (
                "cat src/lib.rs src/model.rs",
                "wot --header src/lib.rs src/model.rs",
            ),
        ];

        for (input, expected) in cases {
            assert_eq!(rewrite_bash_command(input).as_deref(), Some(expected));
        }
    }

    #[test]
    fn rewrites_eligible_segments_in_command_lists() {
        assert_eq!(
            rewrite_bash_command("git status --short && sed -n '1,120p' src/lib.rs; cat README.md")
                .as_deref(),
            Some("git status --short && sed -n '1,120p' src/lib.rs; wot --header README.md")
        );
    }

    #[test]
    fn leaves_narrow_or_semantically_loaded_commands_untouched() {
        let commands = [
            "sed -n '20,60p' src/cli.rs",
            "sed -n '1,240p' src/cli.rs",
            "cat src/cli.rs | sha256sum",
            "cat notes.txt",
            "head -20 README.md",
            "tail -n +100 README.md",
            "sed 's/foo/bar/' src/cli.rs",
            "sed -i 's/foo/bar/' src/cli.rs",
            "cat $FILE",
            "cat '*.rs'",
            "cat src/{lib,cli}.rs",
            "cat AGENTS.md",
            "cat path/to/CLAUDE.md",
            "cat skills/example/SKILL.md",
            "head -120 README.md",
            "tail -n 120 README.md",
            "wot README.md",
            "rg --files",
            "cat src/cli.rs > /tmp/cli.rs",
            "cat src/cli.rs &",
        ];

        for command in commands {
            assert_eq!(rewrite_bash_command(command), None, "{command}");
        }
    }
}
