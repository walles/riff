#[macro_use]
extern crate lazy_static;

use backtrace::Backtrace;
use constants::*;
use git_version::git_version;
use isatty::{stdin_isatty, stdout_isatty};
use refiner::Refiner;
use regex::Regex;
use std::env;
use std::fs;
use std::io::{self, BufRead, BufReader, BufWriter, Write};
use std::panic;
use std::process::exit;
use std::process::{Command, Stdio};
use std::str;

mod constants;
mod refiner;
mod tokenizer;

const HELP_TEXT: &str = r#"
Usage: diff ... | riff

Colors diff and highlights what parts of changed lines have changed.

Git integration:
    git config --global pager.diff riff
    git config --global pager.show riff
    git config --global interactive.filter riff

Options:
    --help:    Print this text
    --version: Print version number
"#;

const HELP_TEXT_FOOTER: &str = r#"
Report issues at <https://github.com/walles/riff>.
"#;

const CRASH_FOOTER: &str = r#"
Please copy all of the above up to the --- RIFF CRASHED --- marker and report it at one of:
* <https://github.com/walles/riff/issues> (preferred)
* <johan.walles@gmail.com>
"#;

const HUNK_HEADER: &str = "\x1b[36m"; // Cyan
const PAGER_FORKBOMB_STOP: &str = "_RIFF_IGNORE_PAGER";

const GIT_VERSION: &str = git_version!();

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
#[must_use]
fn try_pager(pager_name: &str) -> bool {
    let mut command = Command::new(pager_name);

    if env::var(PAGER_FORKBOMB_STOP).is_ok() {
        // Try preventing fork bombing if $PAGER is set to riff
        return false;
    }
    command.env(PAGER_FORKBOMB_STOP, "1");

    if !env::var("LESS").is_ok() {
        // Set by git when paging
        command.env("LESS", "FRX");
    }

    if !env::var("LV").is_ok() {
        // Set by git when paging
        command.env("LV", "-c");
    }

    command.stdin(Stdio::piped());

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
    output.write(HELP_TEXT.trim().as_bytes()).unwrap();
    output.write(b"\n").unwrap();
    output.write(b"\n").unwrap();

    let self_path = env::args().into_iter().next().unwrap();
    let self_path = fs::canonicalize(self_path).unwrap();
    let self_path = self_path.as_path().to_str().unwrap();

    // FIXME: Do this only if we aren't already in the $PATH
    output
        .write("Installing riff in the $PATH:\n".as_bytes())
        .unwrap();
    output
        .write(&format!("  * sudo cp {} /usr/local/bin\n", self_path).as_bytes())
        .unwrap();
    output.write(b"\n").unwrap();

    output.write(HELP_TEXT_FOOTER.trim().as_bytes()).unwrap();
    output.write(b"\n").unwrap();
}

fn panic_handler(panic_info: &panic::PanicInfo) {
    let stderr: &mut dyn Write = &mut io::stderr();
    let stderr = &mut BufWriter::new(stderr);
    println(stderr, "\n\n------------ RIFF CRASHED -------------------");

    // Panic message
    if let Some(s) = panic_info.payload().downcast_ref::<&str>() {
        println(stderr, &format!("Panic message: <{:?}>", s));
        println(stderr, "");
    }

    // Backtrace
    // FIXME: Ditch the Backtrace-internal frames from this backtrace
    // FIXME: Ditch the panic internal frames at the end of the backtrace
    println(stderr, &format!("{:?}", Backtrace::new()));

    println(stderr, &format!("Riff version: {}", GIT_VERSION));

    println(stderr, CRASH_FOOTER);
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
        println!("");
        println!("Source code available at <https://github.com/walles/riff>.");
        return;
    }

    if stdin_isatty() {
        eprintln!("ERROR: Expected input from a pipe");
        eprintln!("");
        print_help(&mut io::stderr());
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

    // No pager found, wth?
    highlight_diff(&mut io::stdin().lock(), &mut io::stdout());
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

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

            // FIXME: Assert that the highlighting output matches the contents of .riff-output
            assert_eq!(actual_result, expected_result);
        }
    }
}
