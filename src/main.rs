use std::io::{self, BufRead};
use diffus::{edit::{self, string}, Diffable};

const ADD: &str = "\x1b[32m"; // Green
const REMOVE: &str = "\x1b[31m";  // Red

const INVERSE_VIDEO: &str = "\x1b[7m";
const NOT_INVERSE_VIDEO: &str = "\x1b[27m";

const NORMAL: &str = "\x1b[0m";

fn simple_print_adds_and_removes(adds: &[String], removes: &[String]) {
    for remove_line in removes {
        println!("{}{}", REMOVE, remove_line)
    }

    for add_line in adds {
        println!("{}{}", ADD, add_line)
    }

    print!("{}", NORMAL);
}

fn print_adds_and_removes(adds: &[String], removes: &[String]) {
    if adds.is_empty() {
        simple_print_adds_and_removes(adds, removes);
        return;
    }

    if removes.is_empty() {
        simple_print_adds_and_removes(adds, removes);
        return;
    }

    // Join inputs by linefeeds into strings
    let adds = adds.join("\n");
    let removes = removes.join("\n");

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
        },
        edit::Edit::Change(diff) => {
            diff.into_iter().map(|edit| {
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
                    },
                    string::Edit::Insert(elem) => {
                        if !adds_is_inverse {
                            highlighted_adds.push_str(INVERSE_VIDEO);
                        }
                        adds_is_inverse = true;

                        highlighted_adds.push(elem);
                    },
                    string::Edit::Remove(elem) => {
                        if !removes_is_inverse {
                            highlighted_removes.push_str(INVERSE_VIDEO);
                        }
                        removes_is_inverse = true;

                        highlighted_removes.push(elem);
                    },
                };
            }).for_each(drop);
        },
    }

    println!("{}{}", REMOVE, highlighted_removes);
    println!("{}{}", ADD, highlighted_adds);
    print!("{}", NORMAL);
}

fn main() {
    println!("Now reading from stdin and printing to stdout:");

    let stdin = io::stdin();
    let mut adds: Vec<String> = Vec::new();
    let mut removes: Vec<String> = Vec::new();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if !line.is_empty() && line.chars().next().unwrap() == '+' {
            adds.push(line);
            continue;
        } else if !line.is_empty() && line.chars().next().unwrap() == '-' {
            removes.push(line);
            continue;
        }

        // Print diff section
        print_adds_and_removes(&adds, &removes);
        adds.clear();
        removes.clear();

        // Print current line
        println!("{}", line);
    }
    print_adds_and_removes(&adds, &removes);

    print!("{}", NORMAL);
}
