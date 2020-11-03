#[macro_use]
extern crate lazy_static;

use diffus::{
    edit::{self, string},
    Diffable,
};
use regex::Regex;
use std::io::{self, BufRead};

const ADD: &str = "\x1b[32m"; // Green
const REMOVE: &str = "\x1b[31m"; // Red
const HUNK_HEADER: &str = "\x1b[36m"; // Cyan

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

/// Format add and remove lines in ADD and REMOVE colors.
///
/// No intra-line refinement.
#[must_use]
fn simple_format_adds_and_removes(adds: &[String], removes: &[String]) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for remove_line in removes {
        lines.push(format!("{}{}{}", REMOVE, remove_line, NORMAL));
    }

    for add_line in adds {
        lines.push(format!("{}{}{}", ADD, add_line, NORMAL))
    }

    return lines;
}

/// Joins multiple lines into a single string.
///
/// The first character of each line is skipped, because it is assumed to be
/// either a `+` or a `-` in a diff.
///
/// Between each joined line a `\n` linefeed character is inserted.
#[must_use]
fn join_skip_first(lines: &[String]) -> String {
    let mut joined = String::new();
    for line in lines {
        if !joined.is_empty() {
            joined.push_str("\n")
        }
        joined.push_str(&line[1..]);
    }

    return joined;
}

#[must_use]
fn format_adds_and_removes(adds: &[String], removes: &[String]) -> Vec<String> {
    if adds.is_empty() {
        return simple_format_adds_and_removes(adds, removes);
    }

    if removes.is_empty() {
        return simple_format_adds_and_removes(adds, removes);
    }

    // Join inputs by linefeeds into strings
    let adds = join_skip_first(adds);
    let removes = join_skip_first(removes);

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

                            highlighted_adds.push(elem);
                        }
                        string::Edit::Remove(elem) => {
                            if !removes_is_inverse {
                                highlighted_removes.push_str(INVERSE_VIDEO);
                            }
                            removes_is_inverse = true;

                            highlighted_removes.push(elem);
                        }
                    };
                })
                .for_each(drop);
        }
    }

    let mut lines: Vec<String> = Vec::new();
    for highlighted_remove in highlighted_removes.lines() {
        lines.push(format!("{}-{}", REMOVE, highlighted_remove));
    }
    for highlighted_add in highlighted_adds.lines() {
        lines.push(format!("{}+{}", ADD, highlighted_add));
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

fn main() {
    let stdin = io::stdin();
    let mut adds: Vec<String> = Vec::new();
    let mut removes: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        let line = line.unwrap();

        let fixed_highlight = get_fixed_highlight(&line);
        if !fixed_highlight.is_empty() {
            // Drain outstanding adds / removes
            for line in format_adds_and_removes(&adds, &removes) {
                println!("{}", line);
            }
            adds.clear();
            removes.clear();

            println!("{}{}{}", fixed_highlight, line, NORMAL);
            continue;
        }

        // Collect adds / removes
        if !line.is_empty() && line.chars().next().unwrap() == '+' {
            adds.push(line);
            continue;
        } else if !line.is_empty() && line.chars().next().unwrap() == '-' {
            removes.push(line);
            continue;
        }

        // Drain outstanding adds / removes
        for line in format_adds_and_removes(&adds, &removes) {
            println!("{}", line);
        }
        adds.clear();
        removes.clear();

        // Print current line
        println!("{}", line);
    }
    for line in format_adds_and_removes(&adds, &removes) {
        println!("{}", line);
    }

    print!("{}", NORMAL);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_join_skip_first() {
        assert_eq!(
            join_skip_first(&["Xa".to_string(), "Xb".to_string()]),
            "a\nb"
        );
    }

    #[test]
    fn test_simple_format_adds_and_removes() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(simple_format_adds_and_removes(&empty, &empty), empty);

        // Test adds-only
        assert_eq!(
            simple_format_adds_and_removes(&["+a".to_string()], &empty),
            ["".to_string() + ADD + "+a" + NORMAL]
        );
        assert_eq!(
            simple_format_adds_and_removes(&["+a".to_string(), "+b".to_string()], &empty),
            [
                "".to_string() + ADD + "+a" + NORMAL,
                "".to_string() + ADD + "+b" + NORMAL,
            ]
        );

        // Test removes-only
        assert_eq!(
            simple_format_adds_and_removes(&empty, &["-a".to_string()]),
            ["".to_string() + REMOVE + "-a" + NORMAL]
        );
        assert_eq!(
            simple_format_adds_and_removes(&empty, &["-a".to_string(), "-b".to_string()]),
            [
                "".to_string() + REMOVE + "-a" + NORMAL,
                "".to_string() + REMOVE + "-b" + NORMAL,
            ]
        );
    }
}
