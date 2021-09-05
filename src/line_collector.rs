use crate::{constants::*, consume};
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

pub struct LineCollector {
    old_text: String,
    new_text: String,
    plain_text: String,
}

impl LineCollector {
    pub fn new() -> LineCollector {
        return LineCollector {
            old_text: String::from(""),
            new_text: String::from(""),
            plain_text: String::from(""),
        };
    }

    fn drop(&mut self) {
        // FIXME: Flush any outstanding lines
    }

    fn consume_plain_line(&self, line: &str) {
        adgagd
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
