// Clippy settings, full list here:
// <https://rust-lang.github.io/rust-clippy/master/index.html>
#![allow(clippy::needless_return)]
//
// Fail build on Clippy warnings
#![deny(warnings)]

#[macro_use]
extern crate lazy_static;

use backtrace::Backtrace;
use git_version::git_version;
use line_collector::LineCollector;
use std::io::{self, IsTerminal};
use std::panic;
use std::path;
use std::process::exit;
use std::process::{Command, Stdio};
use std::str;
use std::{env, fs::File};

mod commit_line;
mod constants;
mod line_collector;
mod refiner;
mod token_collector;
mod tokenizer;

const HELP_TEXT: &str = r#"
Usage:
  diff ... | riff
  riff [-b] [--no-pager] <file1> <file2>
  riff [-b] [--no-pager] <directory1> <directory2>

Colors diff output, highlighting the changed parts of every line.

Git integration:
    git config --global pager.diff riff
    git config --global pager.show riff
    git config --global pager.log riff
    git config --global interactive.diffFilter riff

Options:
    -b:         Ignore changes in amount of whitespace
    --no-pager: Don't page the result

    --help:     Print this text
    --version:  Print version number
"#;

const HELP_TEXT_FOOTER: &str = r#"
Report issues at <https://github.com/walles/riff>.
"#;

const CRASH_FOOTER: &str = r#"
Please copy the above crash report and report it at one of:
* <https://github.com/walles/riff/issues> (preferred)
* <johan.walles@gmail.com>
"#;

const PAGER_FORKBOMB_STOP: &str = "_RIFF_IGNORE_PAGER";

// The empty cargo_prefix makes us use the Cargo.toml version number if we
// cannot get it from git.
//
// Ref: https://github.com/walles/riff/issues/26#issuecomment-1120294897
const GIT_VERSION: &str = git_version!(cargo_prefix = "");

fn highlight_diff<W: io::Write + Send + 'static>(input: &mut dyn io::Read, output: W) {
    let mut line_collector = LineCollector::new(output);

    // Read input line by line, using from_utf8_lossy() to convert lines into
    // strings while handling invalid UTF-8 without crashing
    let mut line_bytes: Vec<u8> = Vec::new();
    let mut buf: [u8; 1] = [0];
    loop {
        let result = input.read(&mut buf);
        if result.is_err() {
            panic!("Error reading input stream: {:?}", result.err().unwrap());
        }

        if result.unwrap() == 0 {
            // End of stream
            if !line_bytes.is_empty() {
                // Stuff found on the last line without a trailing newline
                line_collector.consume_line(String::from_utf8_lossy(&line_bytes).to_string());
            }
            break;
        }

        let single_byte = buf[0];
        if single_byte as char == '\r' {
            // MS-DOS file, LF coming up, just ignore this
            continue;
        }
        if single_byte as char != '\n' {
            // Line contents, store and continue
            line_bytes.push(single_byte);
            continue;
        }

        // Line finished, consume it!
        line_collector.consume_line(String::from_utf8_lossy(&line_bytes).to_string());
        line_bytes.clear();
        continue;
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
            let pager_stdin = pager.stdin.unwrap();
            pager.stdin = None;
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
    eprintln!("Panic message: <{panic_info:#?}>");
    eprintln!();

    // Backtrace
    eprintln!("{:?}", Backtrace::new());

    eprintln!("Riff version: {GIT_VERSION}");

    eprintln!();
    eprintln!("Command line arguments: {:?}", env::args());

    eprintln!("\n-^-^-^------- END OF RIFF CRASH REPORT -------^-^-^-\n",);

    eprintln!("{CRASH_FOOTER}");
}

fn highlight_stream(input: &mut dyn io::Read, no_pager: bool) {
    if !io::stdout().is_terminal() {
        // We're being piped, just do stdin -> stdout
        highlight_diff(input, io::stdout());
        return;
    }

    if no_pager {
        highlight_diff(input, io::stdout());
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
    highlight_diff(input, io::stdout());
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

fn exec_diff_highlight(path1: &str, path2: &str, ignore_space_change: bool, no_pager: bool) {
    let path1 = path::Path::new(path1);
    let path2 = path::Path::new(path2);
    let both_paths_are_non_dirs = !path1.is_dir() && !path2.is_dir();
    let both_paths_are_dirs = path1.is_dir() && path2.is_dir();

    if !(both_paths_are_non_dirs || both_paths_are_dirs) {
        eprintln!("Can only compare directory to directory or not-directory to not-directory, not like this:",);
        eprintln!("  {:<9}: {}", type_string(path1), path1.to_string_lossy());
        eprintln!("  {:<9}: {}", type_string(path2), path2.to_string_lossy());
        exit(1);
    }

    if both_paths_are_non_dirs {
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
        .arg("--show-c-function")
        .arg("--new-file")
        .arg(path1)
        .arg(path2)
        .stdout(Stdio::piped());

    let pretty_command = format!("{command:#?}");
    let mut diff_subprocess: std::process::Child;
    match command.spawn() {
        Ok(subprocess) => diff_subprocess = subprocess,
        Err(err) => {
            eprintln!("ERROR: Spawning diff failed:\n  {pretty_command}\n  {err}\n");
            exit(1);
        }
    }

    let diff_stdout = diff_subprocess.stdout.as_mut().unwrap();
    highlight_stream(diff_stdout, no_pager);

    let diff_result = diff_subprocess.wait().unwrap();
    let diff_exit_code = diff_result.code().unwrap_or(2);
    if diff_exit_code != 0 && diff_exit_code != 1 {
        // diff exit code was neither 0 (comparees identical) or 1 (differences
        // found), this means trouble.
        eprintln!("Exit code {diff_exit_code}: {pretty_command}");
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
        println!("riff {GIT_VERSION}");
        println!();
        println!("Source code available at <https://github.com/walles/riff>.");
        return;
    }

    let ignore_space_change = consume("-b", &mut args);

    if consume("--please-panic", &mut args) {
        panic!("Panicking on purpose");
    }

    let no_pager = consume("--no-pager", &mut args);

    if args.len() == 3 {
        // "riff file1 file2"
        exec_diff_highlight(
            args.get(1).unwrap(),
            args.get(2).unwrap(),
            ignore_space_change,
            no_pager,
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
        eprintln!("ERROR: Unknown command line: {args:?}");
        eprintln!();
        print_help(&mut io::stderr());
        exit(1);
    }

    if io::stdin().is_terminal() {
        eprintln!("ERROR: Expected input from a pipe");
        eprintln!();
        print_help(&mut io::stderr());
        exit(1);
    }

    highlight_stream(&mut io::stdin().lock(), no_pager);
}

#[cfg(test)]
mod tests {
    use crate::constants::*;

    use super::*;
    use std::{fs, path::PathBuf};

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    fn new(text: &str) -> String {
        return format!("{NEW}{text}{NORMAL}");
    }

    #[test]
    fn test_trailing_newline_context() {
        let mut input = "+bepa\n apa\n\\ No newline at end of file\n".as_bytes();

        let expected = format!(
            "{}\n{}\n{}\\ No newline at end of file{}\n",
            new("+bepa"),
            " apa",
            NO_EOF_NEWLINE_COLOR,
            NORMAL
        );

        let file = tempfile::NamedTempFile::new().unwrap();
        highlight_diff(&mut input, file.reopen().unwrap());
        let actual = fs::read_to_string(file.path()).unwrap();
        // collect()ing into line vectors inside of this assert() statement
        // splits test failure output into lines, making it easier to digest.
        assert_eq!(
            actual.lines().collect::<Vec<_>>(),
            expected.lines().collect::<Vec<_>>()
        );
    }

    #[test]
    fn test_testdata_examples() {
        // Example value: `/Users/johan/src/riff/target/debug/deps/riff-7a8916c06b0d3d6c`
        let exe_path = std::env::current_exe().unwrap();

        // Example value: `/Users/johan/src/riff`
        let mut project_path = exe_path
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap();

        // Example value: `/Users/johan/src/riff/testdata`
        let mut testdata_path = project_path.join("testdata");
        if !testdata_path.is_dir() {
            // Might have been built with a target triple, try one more step up:
            // https://github.com/walles/riff/issues/25
            project_path = project_path.parent().unwrap();
            testdata_path = project_path.join("testdata");
        }
        assert!(testdata_path.is_dir());

        // Find all .diff example files
        let mut diff_example_files: Vec<PathBuf> = vec![];
        for diff in fs::read_dir(&testdata_path).unwrap() {
            let diff = diff.unwrap();
            let diff = diff.path();
            if !diff.is_file() {
                continue;
            }

            if diff.extension().unwrap() != "diff" {
                continue;
            }

            diff_example_files.push(diff);
        }
        diff_example_files.sort();

        // Iterate over all the example files
        let mut failing_example: Option<String> = None;
        let mut failing_example_expected = vec![];
        let mut failing_example_actual = vec![];
        for diff in diff_example_files {
            let diff = diff.as_path();
            if !diff.is_file() {
                continue;
            }

            if diff.extension().unwrap() != "diff" {
                continue;
            }

            println!("Evaluating example file <{}>...", diff.to_str().unwrap());

            // Run highlighting on the file into a memory buffer
            let file = tempfile::NamedTempFile::new().unwrap();
            highlight_diff(&mut fs::File::open(diff).unwrap(), file.reopen().unwrap());
            let actual_result = fs::read_to_string(file.path()).unwrap();

            // Load the corresponding .riff-output file into a string
            let basename = diff.file_stem().unwrap().to_str().unwrap();
            let expected_path = format!(
                "{}/{}.riff-output",
                testdata_path.to_str().unwrap(),
                basename
            );
            let expected_result = fs::read_to_string(expected_path).unwrap();

            // Assert that the highlighting output matches the contents of .riff-output
            let actual_lines: Vec<String> = actual_result.split('\n').map(str::to_string).collect();
            let expected_lines: Vec<String> =
                expected_result.split('\n').map(str::to_string).collect();

            if actual_lines != expected_lines {
                if failing_example.is_none() {
                    failing_example = Some(diff.to_str().unwrap().to_string());
                    failing_example_actual = actual_lines;
                    failing_example_expected = expected_lines;
                }

                println!("  FAILED: Output mismatches!");
            }
        }

        if failing_example.is_some() {
            println!();
            println!("Example: {}", failing_example.unwrap());
            assert_eq!(failing_example_actual, failing_example_expected);
        }
    }
}
