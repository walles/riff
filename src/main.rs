#[macro_use]
extern crate lazy_static;

use diffus::{
    edit::{self, string},
    Diffable,
};
use regex::Regex;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::str;

const ADD: &str = "\x1b[32m"; // Green
const REMOVE: &str = "\x1b[31m"; // Red
const HUNK_HEADER: &str = "\x1b[36m"; // Cyan

const NO_EOF_NEWLINE_COLOR: &str = "\x1b[2m"; // Faint
const NO_EOF_NEWLINE_MARKER: &str = "\\ No newline at end of file";

const INVERSE_VIDEO: &str = "\x1b[7m";
const NOT_INVERSE_VIDEO: &str = "\x1b[27m";

const BOLD: &str = "\x1b[1m";

const NORMAL: &str = "\x1b[0m";

lazy_static! {
    static ref STATIC_HEADERS: Vec<(Regex, &'static str)> = vec![
        (Regex::new("^diff ").unwrap(), BOLD),
        (Regex::new("^index ").unwrap(), BOLD),
        (Regex::new("^--- ").unwrap(), BOLD),
        (Regex::new("^\\+\\+\\+ ").unwrap(), BOLD),
        (Regex::new("^@@ ").unwrap(), HUNK_HEADER),
    ];
}

enum LastLineKind {
    Initial,
    Add,
    Remove,
}

/// Format add and remove lines in ADD and REMOVE colors.
///
/// No intra-line refinement.
#[must_use]
fn simple_format_adds_and_removes(adds: &String, removes: &String) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for remove_line in removes.lines() {
        lines.push(format!("{}-{}{}", REMOVE, remove_line, NORMAL));
    }
    if (!removes.is_empty()) && removes.chars().last().unwrap() != '\n' {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    for add_line in adds.lines() {
        lines.push(format!("{}+{}{}", ADD, add_line, NORMAL))
    }
    if (!adds.is_empty()) && adds.chars().last().unwrap() != '\n' {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    return lines;
}

#[must_use]
fn format_adds_and_removes(adds: &String, removes: &String) -> Vec<String> {
    if adds.is_empty() {
        return simple_format_adds_and_removes(adds, removes);
    }

    if removes.is_empty() {
        return simple_format_adds_and_removes(adds, removes);
    }

    // Find diffs between adds and removals
    let mut highlighted_adds = String::new();
    let mut highlighted_removes = String::new();
    let mut adds_is_inverse = false;
    let mut removes_is_inverse = false;
    let diff = removes.diff(&adds);
    match diff {
        edit::Edit::Copy(unchanged) => {
            highlighted_adds.push_str(unchanged);
            highlighted_removes.push_str(unchanged);
        }
        edit::Edit::Change(diff) => {
            diff.into_iter()
                .map(|edit| {
                    match edit {
                        string::Edit::Copy(elem) => {
                            if adds_is_inverse {
                                highlighted_adds.push_str(NOT_INVERSE_VIDEO);
                            }
                            adds_is_inverse = false;

                            if removes_is_inverse {
                                highlighted_removes.push_str(NOT_INVERSE_VIDEO);
                            }
                            removes_is_inverse = false;

                            highlighted_adds.push(elem);
                            highlighted_removes.push(elem);
                        }
                        string::Edit::Insert(elem) => {
                            if !adds_is_inverse {
                                highlighted_adds.push_str(INVERSE_VIDEO);
                            }
                            adds_is_inverse = true;

                            if elem == '\n' {
                                // Make sure the highlighted linefeed is visible
                                highlighted_adds.push('⏎');

                                // This will be reset by the linefeed, so we need to re-inverse on the next line
                                adds_is_inverse = false;
                            }
                            highlighted_adds.push(elem);
                        }
                        string::Edit::Remove(elem) => {
                            if !removes_is_inverse {
                                highlighted_removes.push_str(INVERSE_VIDEO);
                            }
                            removes_is_inverse = true;

                            if elem == '\n' {
                                // Make sure the highlighted linefeed is visible
                                highlighted_removes.push('⏎');

                                // This will be reset by the linefeed, so we need to re-inverse on the next line
                                removes_is_inverse = false;
                            }
                            highlighted_removes.push(elem);
                        }
                    };
                })
                .for_each(drop);
        }
    }

    let mut lines: Vec<String> = Vec::new();
    for highlighted_remove in highlighted_removes.lines() {
        lines.push(format!("{}-{}{}", REMOVE, highlighted_remove, NORMAL));
    }
    if (!removes.is_empty()) && removes.chars().last().unwrap() != '\n' {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    for highlighted_add in highlighted_adds.lines() {
        lines.push(format!("{}+{}{}", ADD, highlighted_add, NORMAL));
    }
    if (!adds.is_empty()) && adds.chars().last().unwrap() != '\n' {
        lines.push(format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
        ));
    }

    return lines;
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

        let fixed_highlight = get_fixed_highlight(&line);
        if !fixed_highlight.is_empty() {
            // Drain outstanding adds / removes
            for line in format_adds_and_removes(&adds, &removes) {
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
        for line in format_adds_and_removes(&adds, &removes) {
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
    for line in format_adds_and_removes(&adds, &removes) {
        println(output, &line);
    }
}

fn main() {
    highlight_diff(&mut io::stdin().lock(), &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_simple_format_adds_and_removes() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(
            simple_format_adds_and_removes(&"".to_string(), &"".to_string()),
            empty
        );

        // Test adds-only
        assert_eq!(
            simple_format_adds_and_removes(&"a\n".to_string(), &"".to_string()),
            ["".to_string() + ADD + "+a" + NORMAL]
        );
        assert_eq!(
            simple_format_adds_and_removes(&"a\nb\n".to_string(), &"".to_string()),
            [
                "".to_string() + ADD + "+a" + NORMAL,
                "".to_string() + ADD + "+b" + NORMAL,
            ]
        );

        // Test removes-only
        assert_eq!(
            simple_format_adds_and_removes(&"".to_string(), &"a\n".to_string()),
            ["".to_string() + REMOVE + "-a" + NORMAL]
        );
        assert_eq!(
            simple_format_adds_and_removes(&"".to_string(), &"a\nb\n".to_string()),
            [
                "".to_string() + REMOVE + "-a" + NORMAL,
                "".to_string() + REMOVE + "-b" + NORMAL,
            ]
        );
    }

    #[test]
    fn test_quote_change() {
        assert_eq!(
            format_adds_and_removes(&"[quotes]\n".to_string(), &"<quotes>\n".to_string()),
            [
                format!(
                    "{}-{}<{}quotes{}>{}{}",
                    REMOVE,
                    INVERSE_VIDEO,
                    NOT_INVERSE_VIDEO,
                    INVERSE_VIDEO,
                    NOT_INVERSE_VIDEO,
                    NORMAL
                ),
                format!(
                    "{}+{}[{}quotes{}]{}{}",
                    ADD, INVERSE_VIDEO, NOT_INVERSE_VIDEO, INVERSE_VIDEO, NOT_INVERSE_VIDEO, NORMAL
                ),
            ]
        )
    }

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
