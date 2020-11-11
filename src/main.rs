#[macro_use]
extern crate lazy_static;

use constants::*;
use isatty::{stdin_isatty, stdout_isatty};
use refiner::Refiner;
use regex::Regex;
use std::env;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::process::exit;
use std::process::{Command, Stdio};
use std::str;

mod constants;
mod refiner;
mod tokenizer;

const HUNK_HEADER: &str = "\x1b[36m"; // Cyan

lazy_static! {
    static ref STATIC_HEADERS: Vec<(Regex, &'static str)> = vec![
        (Regex::new("^diff ").unwrap(), BOLD),
        (Regex::new("^index ").unwrap(), BOLD),
        (Regex::new("^--- ").unwrap(), BOLD),
        (Regex::new("^\\+\\+\\+ ").unwrap(), BOLD),
        (Regex::new("^@@ ").unwrap(), HUNK_HEADER),
    ];
    static ref ANSI_COLOR_REGEX: Regex = Regex::new("\x1b[^m]*m").unwrap();
}

enum LastLineKind {
    Initial,
    Add,
    Remove,
}

#[must_use]
fn get_fixed_highlight(line: &str) -> &str {
    for static_header in STATIC_HEADERS.iter() {
        let re = &static_header.0;
        if re.is_match(line) {
            return static_header.1;
        }
    }

    return "";
}

fn print(stream: &mut BufWriter<&mut dyn Write>, text: &str) {
    stream.write(text.as_bytes()).unwrap();
}

fn println(stream: &mut BufWriter<&mut dyn Write>, text: &str) {
    print(stream, &text);
    print(stream, "\n");
}

fn highlight_diff(input: &mut dyn io::Read, output: &mut dyn io::Write) {
    let mut adds = String::new();
    let mut removes = String::new();
    let input = BufReader::new(input);
    let output = &mut BufWriter::new(output);
    let mut last_line_kind = LastLineKind::Initial;
    for line in input.lines() {
        let line = line.unwrap();

        // Strip out incoming ANSI formatting. This enables us to highlight
        // already-colored input.
        let line = ANSI_COLOR_REGEX.replace_all(&line, "");

        let fixed_highlight = get_fixed_highlight(&line);
        if !fixed_highlight.is_empty() {
            // Drain outstanding adds / removes
            for line in Refiner::create(&adds, &removes).format() {
                println(output, &line);
            }
            adds.clear();
            removes.clear();

            print(output, fixed_highlight);
            print(output, &line);
            println(output, NORMAL);
            continue;
        }

        // Collect adds / removes
        if !line.is_empty() && line.chars().next().unwrap() == '+' {
            adds.push_str(&line[1..]);
            adds.push('\n');
            last_line_kind = LastLineKind::Add;
            continue;
        } else if !line.is_empty() && line.chars().next().unwrap() == '-' {
            removes.push_str(&line[1..]);
            removes.push('\n');
            last_line_kind = LastLineKind::Remove;
            continue;
        }

        if line == NO_EOF_NEWLINE_MARKER {
            match last_line_kind {
                LastLineKind::Add => {
                    assert!(adds.pop().unwrap() == '\n');
                    continue;
                }
                LastLineKind::Remove => {
                    assert!(removes.pop().unwrap() == '\n');
                    continue;
                }
                LastLineKind::Initial => {
                    // This block intentionally left blank
                }
            }
        }

        last_line_kind = LastLineKind::Initial;

        // Drain outstanding adds / removes
        for line in Refiner::create(&adds, &removes).format() {
            println(output, &line);
        }
        adds.clear();
        removes.clear();

        // Print current line
        if line == NO_EOF_NEWLINE_MARKER {
            print(output, NO_EOF_NEWLINE_COLOR);
            print(output, &line);
            println(output, NORMAL);
        } else {
            println(output, &line);
        }
    }
    for line in Refiner::create(&adds, &removes).format() {
        println(output, &line);
    }
}

/// Try paging using the named pager (`$PATH` will be searched).
///
/// Returns `true` if the pager was found, `false` otherwise.
fn try_pager(pager_name: &str) -> bool {
    let mut command = Command::new(pager_name);
    command.stdin(Stdio::piped());

    if !env::var("LESS").is_ok() {
        // Set by git when paging
        command.env("LESS", "FRX");
    }

    if !env::var("LV").is_ok() {
        // Set by git when paging
        command.env("LV", "-c");
    }

    match command.spawn() {
        Ok(mut pager) => {
            let pager_stdin = pager.stdin.as_mut().unwrap();
            highlight_diff(&mut io::stdin().lock(), pager_stdin);

            // FIXME: Report pager exit status if non-zero, together with
            // contents of pager stderr as well if possible.
            pager.wait().expect("Waiting for pager failed");

            return true;
        }
        Err(_) => {
            return false;
        }
    }
}

fn main() {
    if stdin_isatty() {
        eprintln!("Error: Expected input from a pipe");
        exit(1);
    }

    if !stdout_isatty() {
        // We're being piped, just do stdin -> stdout
        highlight_diff(&mut io::stdin().lock(), &mut io::stdout());
        return;
    }

    let pager_env_var = env::var("PAGER");
    if pager_env_var.is_ok() {
        let pager_value = pager_env_var.unwrap();
        if try_pager(&pager_value) {
            return;
        }

        // FIXME: Print warning at the end if $PAGER was set to something that
        // doesn't exist.
    }

    if try_pager("moar") {
        return;
    }

    if try_pager("less") {
        return;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    fn remove(text: &str) -> String {
        return format!("{}{}{}", REMOVE, text, NORMAL);
    }

    fn add(text: &str) -> String {
        return format!("{}{}{}", ADD, text, NORMAL);
    }

    #[test]
    fn test_remove_trailing_newline() {
        let mut input = "-hej\n\
            +hej\n\
            \\ No newline at end of file\n\
            "
        .as_bytes();

        let expected = format!(
            "{}\n{}\n{}\n",
            remove(&format!("-hej{}⏎", INVERSE_VIDEO)),
            add("+hej"),
            format!(
                "{}\\ No newline at end of file{}",
                NO_EOF_NEWLINE_COLOR, NORMAL
            )
        );

        let mut output: Vec<u8> = Vec::new();
        highlight_diff(&mut input, &mut output);
        assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    }

    #[test]
    fn test_trailing_newline_context() {
        let mut input = "+bepa\n apa\n\\ No newline at end of file\n".as_bytes();

        let expected = format!(
            "{}\n{}\n{}\n",
            add("+bepa"),
            " apa",
            format!(
                "{}\\ No newline at end of file{}",
                NO_EOF_NEWLINE_COLOR, NORMAL
            )
        );

        let mut output: Vec<u8> = Vec::new();
        highlight_diff(&mut input, &mut output);
        assert_eq!(std::str::from_utf8(&output).unwrap(), expected);
    }
}
