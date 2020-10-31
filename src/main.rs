use std::io::{self, BufRead};

const ADDITION: &str = "\x1b[32m";  // Green
const REMOVAL: &str = "\x1b[31m";  // Red
const NORMAL: &str = "\x1b[0m";

fn main() {
    println!("Now reading from stdin and printing to stdout:");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        let line = line.unwrap();
        if !line.is_empty() && line.chars().next().unwrap() == '+' {
            print!("{}", ADDITION)
        }
        if !line.is_empty() && line.chars().next().unwrap() == '-' {
            print!("{}", REMOVAL)
        }
        print!("{}", line);

        // FIXME: Only do this if the line had a prefix
        println!("{}", NORMAL);
    }

    print!("{}", NORMAL);
}
