use std::io::{self, BufRead};
use diffus::{edit::{self, collection, string}, Same, Diffable};

const ADDITION: &str = "\x1b[32m"; // Green
const REMOVAL: &str = "\x1b[31m";  // Red
const NORMAL: &str = "\x1b[0m";

fn simple_print_adds_and_removes(adds: &[String], removes: &[String]) {
    for remove_line in removes {
        println!("{}{}", REMOVAL, remove_line)
    }

    for add_line in adds {
        println!("{}{}", ADDITION, add_line)
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
    let diff = adds.diff(&removes);
    match diff {
        edit::Edit::Copy(unchanged) => println!("copy: {}", unchanged),
        edit::Edit::Change(diff) => {
            diff.into_iter().map(|edit| {
                match edit {
                    string::Edit::Copy(elem) => println!("copy: {:?}", elem),
                    string::Edit::Insert(elem) => println!("insert: {:?}", elem),
                    string::Edit::Remove(elem) => println!("remove: {:?}", elem),
                };
            }).collect::<Vec<_>>();
        },
    }

    // FIXME: Print removals with diffus-based highlights

    // FIXME: Print adds with diffus-based highlights

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
