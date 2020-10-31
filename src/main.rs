use std::io::{self, BufRead};

fn main() {
    println!("Now reading from stdin and printing to stdout:");

    let stdin = io::stdin();
    for line in stdin.lock().lines() {
        println!("{}", line.unwrap());
    }
}
