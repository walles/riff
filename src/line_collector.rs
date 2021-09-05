use crate::io::ErrorKind;
use std::io::{self, BufWriter, Write};
use std::process::exit;

use crate::{constants::*, refiner};
use regex::Regex;

const HUNK_HEADER: &str = "\x1b[36m"; // Cyan

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

pub struct LineCollector<'a> {
    old_text: String,
    new_text: String,
    plain_text: String,
    output: &'a mut BufWriter<&'a mut dyn Write>,
}

impl LineCollector<'_> {
    pub fn new(output: &mut dyn io::Write) -> LineCollector {
        let output = &mut BufWriter::new(output);
        return LineCollector {
            old_text: String::from(""),
            new_text: String::from(""),
            plain_text: String::from(""),
            output,
        };
    }

    fn drop(&mut self) {
        // FIXME: Flush any outstanding lines
    }

    fn drain_oldnew(&self) {
        if self.old_text.is_empty() && self.new_text.is_empty() {
            return;
        }

        for line in refiner::format(&self.old_text, &self.new_text) {
            println(self.output, &line);
        }
        self.old_text.clear();
        self.new_text.clear();
    }

    fn consume_plain_line(&self, line: &str) {
        self.drain_oldnew();
        self.plain_text.push_str(&line[1..]);
        self.plain_text.push('\n');
    }

    fn consume_old_line(&self, line: &str) {
        adgagd
    }

    fn consume_new_line(&self, line: &str) {
        adgagd
    }

    fn consume_no_eof_newline_marker(&self) {
        adgagd
    }

    pub fn consume_line(&self, line: String) {
        // Strip out incoming ANSI formatting. This enables us to highlight
        // already-colored input.
        let line = ANSI_COLOR_REGEX.replace_all(&line, "");

        let fixed_highlight = get_fixed_highlight(&line);
        if !fixed_highlight.is_empty() {
            self.consume_plain_line(&format!("{}{}{}", fixed_highlight, line, NORMAL));
            return;
        }

        if line.is_empty() {
            self.consume_plain_line("");
            return;
        }

        if line.starts_with('-') {
            self.consume_old_line(&line);
            return;
        }

        if line.starts_with('+') {
            self.consume_new_line(&line);
            return;
        }

        if line == NO_EOF_NEWLINE_MARKER {
            self.consume_no_eof_newline_marker();
            return;
        }

        self.consume_plain_line(&line);
    }
}
