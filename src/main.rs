// Clippy settings, full list here:
// <https://rust-lang.github.io/rust-clippy/master/index.html>
#![allow(clippy::needless_return)]
//
// Fail build on Clippy warnings
#![deny(warnings)]

#[macro_use]
extern crate lazy_static;

use backtrace::Backtrace;
use clap::CommandFactory;
use clap::Parser;
use clap::ValueEnum;
use git_version::git_version;
use line_collector::LineCollector;
use log::error;
use logging::init_logger;
use std::io::{self, IsTerminal};
use std::panic;
use std::path::{self, PathBuf};
use std::process::exit;
use std::process::{Command, Stdio};
use std::str;
use std::{env, fs::File};

mod ansi;
mod commit_line;
mod conflicts_highlighter;
mod constants;
mod hunk_header;
mod hunk_highlighter;
mod line_collector;
mod lines_highlighter;
mod logging;
mod plusminus_header_highlighter;
mod plusminus_lines_highlighter;
mod refiner;
mod string_future;
mod token_collector;
mod tokenizer;

const HELP_TEXT_FOOTER: &str = r#"Installing riff in the $PATH:
  sudo cp riff /usr/local/bin

Git integration:
  git config --global pager.diff riff
  git config --global pager.show riff
  git config --global pager.log riff
  git config --global interactive.diffFilter "riff --color=on"

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

#[derive(Parser)]
#[command(
    version = GIT_VERSION,
    name = "riff",
    about = "Colors diff output, highlighting the changed parts of every line.",
    after_help = HELP_TEXT_FOOTER,
    override_usage = r#"
  diff ... | riff [options...]
  riff [-b] [-w] [options...] <X1> <X2>
  riff [-b] [-w] [options...] --file <FILE>"#
)]

struct Options {
    /// First file or directory to compare
    #[arg(requires("x2"))]
    x1: Option<String>,

    /// Second file or directory to compare
    #[arg()]
    x2: Option<String>,

    /// Read diff or patch file
    #[arg(long, short, conflicts_with_all = ["x1", "x2"])]
    file: Option<PathBuf>,

    /// Ignore changes in amount of whitespace
    #[arg(long, short('b'), conflicts_with_all = ["ignore_all_space"])]
    ignore_space_change: bool,

    /// Ignore all whitespace
    #[arg(long, short('w'), conflicts_with_all = ["ignore_space_change"])]
    ignore_all_space: bool,

    /// Don't page the result
    #[arg(long)]
    no_pager: bool,

    /// `auto` = color if stdout is a terminal
    #[arg(long)]
    color: Option<ColorOption>,

    #[arg(long, hide(true))]
    please_panic: bool,
}

#[derive(ValueEnum, Clone, Default)]
enum ColorOption {
    On,
    Off,

    #[default]
    Auto,
}

impl ColorOption {
    fn bool_or(self, default: bool) -> bool {
        match self {
            ColorOption::On => true,
            ColorOption::Off => false,
            ColorOption::Auto => default,
        }
    }
}

fn format_error(message: String, line_number: usize, line: &[u8]) -> String {
    return format!(
        "On line {}: {}\n  Line {}: {}",
        line_number,
        message,
        line_number,
        String::from_utf8_lossy(line),
    );
}

fn highlight_diff_or_exit<W: io::Write + Send + 'static>(
    input: &mut dyn io::Read,
    output: W,
    color: bool,
) {
    if let Err(message) = highlight_diff(input, output, color) {
        eprintln!("{}", message);
        exit(1);
    }
}

/// Read `diff` output from `input` and write highlighted output to `output`.
/// The actual highlighting is done using a `LineCollector`.
fn highlight_diff<W: io::Write + Send + 'static>(
    input: &mut dyn io::Read,
    output: W,
    color: bool,
) -> Result<(), String> {
    let mut line_collector = LineCollector::new(output, color);

    // Read input line by line, using from_utf8_lossy() to convert lines into
    // strings while handling invalid UTF-8 without crashing
    let mut line: Vec<u8> = Vec::new();
    let mut buf: [u8; 16384] = [0; 16384];
    let mut line_number = 1usize;
    loop {
        let result = input.read(&mut buf);
        if result.is_err() {
            panic!("Error reading input stream: {:?}", result.err().unwrap());
        }

        let read_count = result.unwrap();
        if read_count == 0 {
            // End of stream
            if !line.is_empty() {
                // Stuff found on the last line without a trailing newline
                if let Err(message) = line_collector.consume_line(&line) {
                    error!("{}", format_error(message, line_number, &line));
                }
            }
            break;
        }

        for byte in buf.iter().take(read_count) {
            let byte = *byte;
            if byte == b'\r' {
                // MS-DOS file, LF coming up, just ignore this
                continue;
            }
            if byte != b'\n' {
                // Line contents, store and continue
                line.push(byte);
                continue;
            }

            // Line finished, consume it!
            if let Err(message) = line_collector.consume_line(&line) {
                error!("{}", format_error(message, line_number, &line));
            }
            line.clear();
            line_number += 1;
            continue;
        }
    }

    return Ok(());
}

/// Try paging using the named pager (`$PATH` will be searched).
///
/// Returns `true` if the pager was found, `false` otherwise.
#[must_use]
fn try_pager(input: &mut dyn io::Read, pager_name: &str, color: bool) -> bool {
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
            highlight_diff_or_exit(input, pager_stdin, color);

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

/// Highlight the given stream, paging if stdout is a terminal
fn highlight_stream(input: &mut dyn io::Read, no_pager: bool, color: bool) {
    if !io::stdout().is_terminal() {
        // We're being piped, just do stdin -> stdout
        highlight_diff_or_exit(input, io::stdout(), color);
        return;
    }

    if no_pager {
        highlight_diff_or_exit(input, io::stdout(), color);
        return;
    }

    if let Ok(pager_value) = env::var("PAGER") {
        if try_pager(input, &pager_value, color) {
            return;
        }

        // FIXME: Print warning at the end if $PAGER was set to something that
        // doesn't exist.
    }

    if try_pager(input, "moar", color) {
        return;
    }

    if try_pager(input, "less", color) {
        return;
    }

    // No pager found, wth?
    highlight_diff_or_exit(input, io::stdout(), color);
}

/// `Not found`, `File`, `Directory` or `Not file not dir`
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

/// Run the `diff` binary on the two paths and highlight the output
fn exec_diff_highlight(
    path1: &str,
    path2: &str,
    ignore_space_change: bool,
    ignore_all_space: bool,
    no_pager: bool,
    color: bool,
) {
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

    if ignore_all_space {
        command = command.arg("-w");
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
    highlight_stream(diff_stdout, no_pager, color);

    let diff_result = diff_subprocess.wait().unwrap();
    let diff_exit_code = diff_result.code().unwrap_or(2);
    if diff_exit_code != 0 && diff_exit_code != 1 {
        // diff exit code was neither 0 (comparees identical) or 1 (differences
        // found), this means trouble.
        eprintln!("Exit code {diff_exit_code}: {pretty_command}");
        exit(diff_exit_code);
    }
}

/// Will return the first argument from the command line, followed by any
/// arguments from the `RIFF` environment variable, followed by the rest of the
/// command line arguments.
fn env_and_command_line() -> Vec<String> {
    let mut result = vec![];

    // First argument from the command line
    result.push(env::args().next().unwrap());

    // Arguments from the `RIFF` environment variable
    if let Ok(riff) = env::var("RIFF") {
        result.extend(riff.split_whitespace().map(str::to_string));
    }

    // Rest of the command line arguments
    result.extend(env::args().skip(1));

    return result;
}

fn main() {
    panic::set_hook(Box::new(|panic_info: &panic::PanicInfo| {
        panic_handler(panic_info);
    }));

    let logger = init_logger().unwrap();

    let options = Options::try_parse_from(env_and_command_line());
    if let Err(e) = options {
        let _ = e.print();
        if let Ok(riff) = env::var("RIFF") {
            if e.kind() == clap::error::ErrorKind::DisplayHelp {
                println!();
                println!("Environment:");
                println!("  RIFF={}", riff);
            } else {
                eprintln!();
                eprintln!("Environment:");
                eprintln!("  RIFF={}", riff);
            }
        }

        exit(e.exit_code());
    }
    let options = options.unwrap();

    if options.please_panic {
        panic!("Panicking on purpose");
    }

    if let (Some(file1), Some(file2)) = (options.x1, options.x2) {
        // "riff file1 file2"
        exec_diff_highlight(
            &file1,
            &file2,
            options.ignore_space_change,
            options.ignore_all_space,
            options.no_pager,
            options
                .color
                .unwrap_or(ColorOption::Auto)
                .bool_or(io::stdout().is_terminal()),
        );
        return;
    }

    if let Some(diff_path) = options.file {
        // riff -f file
        if diff_path.is_dir() {
            eprintln!("ERROR: --file cannot be a directory");
            exit(1)
        }

        let mut diff_file = match File::open(diff_path.clone()) {
            Ok(f) => f,
            Err(e) => {
                eprintln!("ERROR: Can't open {}: {}", diff_path.to_string_lossy(), e);
                exit(1);
            }
        };
        highlight_stream(
            &mut diff_file,
            options.no_pager,
            options
                .color
                .unwrap_or(ColorOption::Auto)
                .bool_or(io::stdout().is_terminal()),
        );
        return;
    }

    if io::stdin().is_terminal() {
        eprintln!("ERROR: Expected input from a pipe");
        eprintln!();

        // Print help to stderr
        Options::command().write_help(&mut io::stderr()).unwrap();

        exit(1);
    }

    highlight_stream(
        &mut io::stdin().lock(),
        options.no_pager,
        options
            .color
            .unwrap_or(ColorOption::Auto)
            .bool_or(io::stdout().is_terminal()),
    );

    let logs = logger.get_logs();
    if !logs.is_empty() {
        // FIXME: Print version number and some error reporting header? With
        // links to the GitHub issue tracker?
        eprintln!("{}", logs);
        exit(1);
    }
}

#[cfg(test)]
mod tests {
    use crate::{constants::*, hunk_header::HUNK_HEADER};

    use super::*;
    use std::{collections::HashSet, fs, path::PathBuf};

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    fn new(text: &str) -> String {
        return format!("{NEW}{text}{NORMAL}");
    }

    #[test]
    fn test_trailing_newline_context() {
        let mut input = "@@ -1,1 +1,2 @@\n+bepa\n apa\n\\ No newline at end of file\n".as_bytes();

        let expected = format!(
            "{}{}{}\n{}\n{}\n{}\\ No newline at end of file{}\n",
            HUNK_HEADER,
            "@@ -1,1 +1,2 @@",
            NORMAL,
            new("+bepa"),
            " apa",
            NO_EOF_NEWLINE_COLOR,
            NORMAL
        );

        let file = tempfile::NamedTempFile::new().unwrap();
        if let Err(error) = highlight_diff(&mut input, file.reopen().unwrap(), true) {
            panic!("{}", error);
        }
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

        // Find all .riff-output example files
        let mut riff_output_files: Vec<PathBuf> = vec![];
        let mut diff_files: HashSet<PathBuf> = HashSet::new();
        for riff_output in fs::read_dir(&testdata_path).unwrap() {
            let riff_output = riff_output.unwrap();
            let riff_output = riff_output.path();
            if !riff_output.is_file() {
                continue;
            }

            if riff_output.extension().unwrap() == "diff" {
                diff_files.insert(riff_output);
                continue;
            }

            if riff_output.extension().unwrap() != "riff-output" {
                continue;
            }

            riff_output_files.push(riff_output);
        }
        riff_output_files.sort();
        let example_count = riff_output_files.len();

        // Iterate over all the example output files
        let mut failing_example: Option<String> = None;
        let mut failing_example_expected = vec![];
        let mut failing_example_actual = vec![];
        let mut failure_count = 0;
        for expected_output_file in riff_output_files {
            let without_riff_output_extension =
                expected_output_file.file_stem().unwrap().to_str().unwrap();

            // Find the corresponding .diff file...
            let mut riff_input_file =
                testdata_path.join(format!("{}.diff", without_riff_output_extension));
            // ... or just the corresponding whatever file.
            if !riff_input_file.is_file() {
                // Used by the conflict-markers*.txt.riff-output files
                riff_input_file = testdata_path.join(without_riff_output_extension);
            }
            if !riff_input_file.is_file() {
                if failing_example.is_none() {
                    failing_example = Some(expected_output_file.to_str().unwrap().to_string());
                    failing_example_expected = vec![];
                    failing_example_actual = vec![];
                }

                println!(
                    "FAIL: No riff input file found for {:?}",
                    expected_output_file
                );
                failure_count += 1;
                continue;
            }

            if riff_input_file.extension().unwrap() == "diff" {
                diff_files.remove(&riff_input_file);
            }

            println!(
                "Evaluating example file <{}>...",
                riff_input_file.to_str().unwrap()
            );

            if let Some(failure) = test_testdata_example(&riff_input_file, &expected_output_file) {
                println!("  FAILED: {}", failure.diagnostics);
                failure_count += 1;

                if failing_example.is_some() {
                    continue;
                }

                eprintln!("  FAILED: {}", failure.diagnostics);

                failing_example = Some(riff_input_file.to_str().unwrap().to_string());

                let actual_lines: Vec<String> = failure
                    .actual_result
                    .split('\n')
                    .map(str::to_string)
                    .collect();
                let expected_lines: Vec<String> = failure
                    .expected_result
                    .split('\n')
                    .map(str::to_string)
                    .collect();

                failing_example_expected = expected_lines;
                failing_example_actual = actual_lines;
            }
        }

        println!("\n{}/{} examples failed", failure_count, example_count,);

        if let Some(failing_example) = failing_example {
            println!();
            println!("Example: {}", failing_example);
            println!();
            println!("==> Run \"./testdata-examples.sh\" to visualize changes / failures");
            println!();
            assert_eq!(failing_example_actual, failing_example_expected);

            // Sometimes the previous assert doesn't trigger, so we put this one
            // here as a safety measure. Do not remove it!!
            panic!("Example failed");
        }

        if !diff_files.is_empty() {
            panic!("Some .diff files were never verified: {:?}", diff_files);
        }
    }

    struct ExampleFailure {
        diagnostics: String,
        actual_result: String,
        expected_result: String,
    }

    fn test_testdata_example(
        input_file: &PathBuf,
        expected_output_file: &PathBuf,
    ) -> Option<ExampleFailure> {
        // Run highlighting on the file into a memory buffer
        let file = tempfile::NamedTempFile::new().unwrap();
        if let Err(error) = highlight_diff(
            &mut fs::File::open(input_file).unwrap(),
            file.reopen().unwrap(),
            true,
        ) {
            return Some(ExampleFailure {
                diagnostics: format!("Highlighting failed: {}", error),
                actual_result: "".to_string(),
                expected_result: "".to_string(),
            });
        }

        let actual_result = fs::read_to_string(file.path()).unwrap();

        // Load the corresponding .riff-output file into a string
        let expected_result = fs::read_to_string(expected_output_file).unwrap();

        if actual_result != expected_result {
            return Some(ExampleFailure {
                diagnostics: "Output mismatches".to_string(),
                actual_result,
                expected_result,
            });
        }

        // Test that disabling color results in no escape codes
        let file = tempfile::NamedTempFile::new().unwrap();
        highlight_diff(
            &mut fs::File::open(input_file).unwrap(),
            file.reopen().unwrap(),
            false,
        )
        .unwrap();

        let highlighted = fs::read_to_string(file.path()).unwrap();
        if highlighted.contains('\x1b') {
            return Some(ExampleFailure {
                diagnostics: "Escape codes found in the supposedly non-colored output".to_string(),
                actual_result: highlighted,
                expected_result: "".to_string(),
            });
        }

        return None;
    }
}
