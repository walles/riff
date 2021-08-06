// Clippy settings, full list here:
// <https://rust-lang.github.io/rust-clippy/master/index.html>
#![allow(clippy::needless_return)]
//
// Fail build on Clippy warnings
#![deny(warnings)]

#[macro_use]
extern crate lazy_static;

use backtrace::Backtrace;
use constants::*;
use git_version::git_version;
use io::ErrorKind;
use isatty::{stdin_isatty, stdout_isatty};
use regex::Regex;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::panic;
use std::path;
use std::process::exit;
use std::process::{Command, Stdio};
use std::str;
use std::{env, fs::File};

mod constants;
mod refiner;
mod token_collector;
mod tokenizer;

const HELP_TEXT: &str = r#"
Usage:
  diff ... | riff
  riff [-b] <file1> <file2>
  riff [-b] <directory1> <directory2>

Colors diff output, highlighting the changed parts of every line.

Git integration:
    git config --global pager.diff riff
    git config --global pager.show riff
    git config --global interactive.diffFilter riff

Options:
    -b:        Ignore changes in amount of whitespace
    --help:    Print this text
    --version: Print version number
"#;

const HELP_TEXT_FOOTER: &str = r#"
Report issues at <https://github.com/walles/riff>.
"#;

const CRASH_FOOTER: &str = r#"
Please copy the above crash report and report it at one of:
* <https://github.com/walles/riff/issues> (preferred)
* <johan.walles@gmail.com>
"#;

const HUNK_HEADER: &str = "\x1b[36m"; // Cyan
const PAGER_FORKBOMB_STOP: &str = "_RIFF_IGNORE_PAGER";

const GIT_VERSION: &str = git_version!();

lazy_static! {
    static ref STATIC_HEADER_PREFIXES: Vec<(&'static str, &'static str)> = vec![
        ("diff ", BOLD),
        ("index ", BOLD),
        ("--- ", BOLD),
        ("+++ ", BOLD),
        ("@@ ", HUNK_HEADER),
    ];
    static ref ANSI_COLOR_REGEX: Regex = Regex::new("\x1b[^m]*m").unwrap();
}

enum LastLineKind {
    Initial,
    Old,
    New,
}

#[must_use]
fn get_fixed_highlight(line: &str) -> &str {
    for static_header_prefix in STATIC_HEADER_PREFIXES.iter() {
        let prefix = static_header_prefix.0;
        if line.starts_with(prefix) {
            return static_header_prefix.1;
        }
    }

    return "";
}

fn print(stream: &mut BufWriter<&mut dyn Write>, text: &str) {
    if let Err(error) = stream.write_all(text.as_bytes()) {
        if error.kind() == ErrorKind::BrokenPipe {
            // This is fine, somebody probably just quit their pager before it
            // was done reading our output.
            exit(0);
        }

        panic!("Error writing diff to pager: {:?}", error);
    }
}

fn println(stream: &mut BufWriter<&mut dyn Write>, text: &str) {
    print(stream, text);
    print(stream, "\n");
}

fn highlight_diff(input: &mut dyn io::Read, output: &mut dyn io::Write) {
    let mut old_text = String::new();
    let mut new_text = String::new();
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
            for line in refiner::format(&old_text, &new_text) {
                println(output, &line);
            }
            old_text.clear();
            new_text.clear();

            print(output, fixed_highlight);
            print(output, &line);
            println(output, NORMAL);
            continue;
        }

        // Collect adds / removes
        if !line.is_empty() && line.starts_with('+') {
            new_text.push_str(&line[1..]);
            new_text.push('\n');
            last_line_kind = LastLineKind::New;
            continue;
        } else if !line.is_empty() && line.starts_with('-') {
            old_text.push_str(&line[1..]);
            old_text.push('\n');
            last_line_kind = LastLineKind::Old;
            continue;
        }

        if line == NO_EOF_NEWLINE_MARKER {
            match last_line_kind {
                LastLineKind::Old => {
                    assert!(old_text.pop().unwrap() == '\n');
                    continue;
                }
                LastLineKind::New => {
                    assert!(new_text.pop().unwrap() == '\n');
                    continue;
                }
                LastLineKind::Initial => {
                    // This block intentionally left blank
                }
            }
        }

        last_line_kind = LastLineKind::Initial;

        // Drain outstanding adds / removes
        for line in refiner::format(&old_text, &new_text) {
            println(output, &line);
        }
        old_text.clear();
        new_text.clear();

        // Print current line
        if line == NO_EOF_NEWLINE_MARKER {
            print(output, NO_EOF_NEWLINE_COLOR);
            print(output, &line);
            println(output, NORMAL);
        } else {
            println(output, &line);
        }
    }
    for line in refiner::format(&old_text, &new_text) {
        println(output, &line);
    }
}

/// Try paging using the named pager (`$PATH` will be searched).
///
/// Returns `true` if the pager was found, `false` otherwise.
#[must_use]
fn try_pager(input: &mut dyn io::Read, pager_name: &str) -> bool {
    let mut command = Command::new(pager_name);

    if env::var(PAGER_FORKBOMB_STOP).is_ok() {
        // Try preventing fork bombing if $PAGER is set to riff
        return false;
    }
    command.env(PAGER_FORKBOMB_STOP, "1");

    if env::var("LESS").is_err() {
        // Set by git when paging
        command.env("LESS", "FRX");
    }

    if env::var("LV").is_err() {
        // Set by git when paging
        command.env("LV", "-c");
    }

    command.stdin(Stdio::piped());

    match command.spawn() {
        Ok(mut pager) => {
            let pager_stdin = pager.stdin.as_mut().unwrap();
            highlight_diff(input, pager_stdin);

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

/// If `option` is found in `argv`, all instances of `option` will be removed
/// from `argv`.
///
/// Returns `true` if `option` was found and consumed, false otherwise.
#[must_use]
fn consume(option: &str, argv: &mut Vec<String>) -> bool {
    if !argv.contains(&option.to_string()) {
        // Not found
        return false;
    }

    argv.retain(|x| x != option);
    return true;
}

fn print_help(output: &mut dyn io::Write) {
    output.write_all(HELP_TEXT.trim().as_bytes()).unwrap();
    output.write_all(b"\n").unwrap();
    output.write_all(b"\n").unwrap();

    output
        .write_all(b"Installing riff in the $PATH:\n")
        .unwrap();
    output
        .write_all(b"    sudo cp riff /usr/local/bin\n")
        .unwrap();
    output.write_all(b"\n").unwrap();

    output
        .write_all(HELP_TEXT_FOOTER.trim().as_bytes())
        .unwrap();
    output.write_all(b"\n").unwrap();
}

fn panic_handler(panic_info: &panic::PanicInfo) {
    eprintln!("\n\n-v-v-v----------- RIFF CRASHED ---------------v-v-v-\n",);

    // Panic message
    eprintln!("Panic message: <{:#?}>", panic_info);
    eprintln!();

    // Backtrace
    eprintln!("{:?}", Backtrace::new());

    eprintln!("Riff version: {}", GIT_VERSION);

    eprintln!();
    eprintln!("Command line arguments: {:?}", env::args());

    eprintln!("\n-^-^-^------- END OF RIFF CRASH REPORT -------^-^-^-\n",);

    eprintln!("{}", CRASH_FOOTER);
}

fn highlight_stream(input: &mut dyn io::Read) {
    if !stdout_isatty() {
        // We're being piped, just do stdin -> stdout
        highlight_diff(input, &mut io::stdout());
        return;
    }

    if let Ok(pager_value) = env::var("PAGER") {
        if try_pager(input, &pager_value) {
            return;
        }

        // FIXME: Print warning at the end if $PAGER was set to something that
        // doesn't exist.
    }

    if try_pager(input, "moar") {
        return;
    }

    if try_pager(input, "less") {
        return;
    }

    // No pager found, wth?
    highlight_diff(input, &mut io::stdout());
}

pub fn type_string(path: &path::Path) -> &str {
    if !path.exists() {
        return "Not found";
    }
    if path.is_file() {
        return "File";
    }
    if path.is_dir() {
        return "Directory";
    }
    return "Not file not dir";
}

fn ensure_readable(path: &path::Path) {
    if let Err(why) = File::open(path) {
        eprintln!("ERROR: {}: {}", why, path.to_string_lossy());
        exit(1);
    };
}

fn ensure_listable(path: &path::Path) {
    if let Err(why) = std::fs::read_dir(path) {
        eprintln!("ERROR: {}: {}", why, path.to_string_lossy());
        exit(1);
    }
}

fn exec_diff_highlight(path1: &str, path2: &str, ignore_space_change: bool) {
    let path1 = path::Path::new(path1);
    let path2 = path::Path::new(path2);
    let both_paths_are_files = path1.is_file() && path2.is_file();
    let both_paths_are_dirs = path1.is_dir() && path2.is_dir();

    if !(both_paths_are_files || both_paths_are_dirs) {
        eprintln!("Can only compare file to file or directory to directory, not like this:",);
        eprintln!("  {:<9}: {}", type_string(path1), path1.to_string_lossy());
        eprintln!("  {:<9}: {}", type_string(path2), path2.to_string_lossy());
        exit(1);
    }

    if both_paths_are_files {
        ensure_readable(path1);
        ensure_readable(path2);
    } else {
        ensure_listable(path1);
        ensure_listable(path2);
    }

    // Run "diff -ur file1 file2"
    let mut command: &mut Command = &mut Command::new("diff");

    if ignore_space_change {
        command = command.arg("-b");
    }

    let command = command
        .arg("-ur") // "-u = unified diff, -r = recurse subdirectories"
        .arg(path1)
        .arg(path2)
        .stdout(Stdio::piped());

    let pretty_command = format!("{:#?}", command);
    let mut diff_subprocess = command.spawn().expect(&pretty_command);
    let diff_stdout = diff_subprocess.stdout.as_mut().unwrap();
    highlight_stream(diff_stdout);

    let diff_result = diff_subprocess.wait().unwrap();
    let diff_exit_code = diff_result.code().unwrap_or(2);
    if diff_exit_code != 0 && diff_exit_code != 1 {
        // diff exit code was neither 0 (comparees identical) or 1 (differences
        // found), this means trouble.
        eprintln!("Exit code {}: {}", diff_exit_code, pretty_command);
        exit(diff_exit_code);
    }
}

fn main() {
    panic::set_hook(Box::new(|panic_info: &panic::PanicInfo| {
        panic_handler(panic_info);
    }));

    let mut args: Vec<String> = env::args().collect();
    if consume("--help", &mut args) || consume("-h", &mut args) {
        print_help(&mut io::stdout());
        return;
    }

    if consume("--version", &mut args) {
        println!("riff {}", GIT_VERSION);
        println!();
        println!("Source code available at <https://github.com/walles/riff>.");
        return;
    }

    let ignore_space_change = consume("-b", &mut args);

    if consume("--please-panic", &mut args) {
        panic!("Panicking on purpose");
    }

    if args.len() == 3 {
        // "riff file1 file2"
        exec_diff_highlight(
            args.get(1).unwrap(),
            args.get(2).unwrap(),
            ignore_space_change,
        );
        return;
    }

    if ignore_space_change {
        eprintln!(
            "ERROR: -b is only supported when diffing two named paths (\"riff -b a.txt b.txt\")"
        );
        eprintln!();
        print_help(&mut io::stderr());
        exit(1);
    }

    if args.len() != 1 {
        eprintln!("ERROR: Unknown command line: {:?}", args);
        eprintln!();
        print_help(&mut io::stderr());
        exit(1);
    }

    if stdin_isatty() {
        eprintln!("ERROR: Expected input from a pipe");
        eprintln!();
        print_help(&mut io::stderr());
        exit(1);
    }

    highlight_stream(&mut io::stdin().lock());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    fn old(text: &str) -> String {
        return format!("{}{}{}", OLD, text, NORMAL);
    }

    fn new(text: &str) -> String {
        return format!("{}{}{}", NEW, text, NORMAL);
    }

    #[test]
    fn test_remove_trailing_newline() {
        let mut input = "-hejhopp\n\
            +hejhopp\n\
            \\ No newline at end of file\n\
            "
        .as_bytes();

        let expected = format!(
            "{}\n{}\n{}\n",
            old(&format!("-hejhopp{}⏎", INVERSE_VIDEO)),
            new("+hejhopp"),
            format!(
                "{}\\ No newline at end of file{}",
                NO_EOF_NEWLINE_COLOR, NORMAL
            )
        );

        let mut actual: Vec<u8> = Vec::new();
        highlight_diff(&mut input, &mut actual);
        // collect()ing into line vectors inside of this assert() statement
        // splits test failure output into lines, making it easier to digest.
        assert_eq!(
            std::str::from_utf8(&actual)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected.lines().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_trailing_newline_context() {
        let mut input = "+bepa\n apa\n\\ No newline at end of file\n".as_bytes();

        let expected = format!(
            "{}\n{}\n{}\n",
            new("+bepa"),
            " apa",
            format!(
                "{}\\ No newline at end of file{}",
                NO_EOF_NEWLINE_COLOR, NORMAL
            )
        );

        let mut output: Vec<u8> = Vec::new();
        highlight_diff(&mut input, &mut output);
        // collect()ing into line vectors inside of this assert() statement
        // splits test failure output into lines, making it easier to digest.
        assert_eq!(
            std::str::from_utf8(&output)
                .unwrap()
                .lines()
                .collect::<Vec<_>>(),
            expected.lines().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_testdata_examples() {
        // Example value: `/Users/johan/src/riff/target/debug/deps/riff-7a8916c06b0d3d6c`
        let exe_path = std::env::current_exe().unwrap();

        // Example value: `/Users/johan/src/riff`
        let project_path = exe_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();

        // Example value: `/Users/johan/src/riff/testdata`
        let testdata_path = &project_path.join("testdata");
        assert!(testdata_path.is_dir());

        // Iterate all files in there
        for diff in fs::read_dir(testdata_path).unwrap() {
            let diff = diff.unwrap();
            let diff = diff.path();
            let diff = diff.as_path();
            if !diff.is_file() {
                continue;
            }

            if diff.extension().unwrap() != "diff" {
                continue;
            }

            println!("Evaluating example file <{}>...", diff.to_str().unwrap());

            // Run highlighting on the file into a memory buffer
            let mut actual_result: Vec<u8> = Vec::new();
            highlight_diff(&mut fs::File::open(diff).unwrap(), &mut actual_result);
            let actual_result = str::from_utf8(&actual_result).unwrap();

            // Load the corresponding .riff-output file into a string
            let basename = diff.file_stem().unwrap().to_str().unwrap();
            let expected_path = format!(
                "{}/{}.riff-output",
                testdata_path.to_str().unwrap(),
                basename
            );
            let expected_result = fs::read_to_string(expected_path).unwrap();

            // Assert that the highlighting output matches the contents of .riff-output
            let actual_lines: Vec<&str> = actual_result.split('\n').collect();
            let expected_lines: Vec<&str> = expected_result.split('\n').collect();
            assert_eq!(actual_lines, expected_lines);
        }
    }
}
