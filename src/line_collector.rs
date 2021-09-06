use crate::io::ErrorKind;
use std::io::{self, BufWriter, Write};
use std::process::exit;
use std::sync::mpsc::{channel, Receiver, Sender};

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

pub struct LineCollector<'a> {
    old_text: String,
    new_text: String,
    plain_text: String,
    output: BufWriter<&'a mut dyn Write>,
    queue_putter: Sender<String>,
    queue_getter: Receiver<String>,
}

impl<'a> Drop for LineCollector<'a> {
    fn drop(&mut self) {
        // Flush any outstanding lines. This can be done in any order, at most
        // one of them is going to do anything anyway.
        self.drain_oldnew();
        self.drain_plain();

        // FIXME: As an intermediate step until we have a queue draining thread,
        // just drain the queue here and print everything in it
        while let Ok(print_me) = self.queue_getter.try_recv() {
            print(&mut self.output, &print_me);
        }

        // FIXME: Tell the consumer thread to drain and quit

        // FIXME: Wait for the consumer thread to finish

        // FIXME: Do we need to shut down our executor as well?
    }
}

impl<'a> LineCollector<'a> {
    pub fn new(output: &mut dyn io::Write) -> LineCollector {
        // FIXME: Start an executor here with one thread per logical CPU core

        // FIXME: Start a consumer thread here which takes futures and prints
        // their results

        // Allocate a queue where we can push our futures to the consumer thread
        //
        // FIXME: Once we get another thread consuming this, this thread should
        // be bounded to 2x the number of logical CPUs.
        let (queue_putter, queue_getter): (Sender<String>, Receiver<String>) = channel();

        let output = BufWriter::new(output);
        return LineCollector {
            old_text: String::from(""),
            new_text: String::from(""),
            plain_text: String::from(""),
            output,
            queue_putter,
            queue_getter,
        };
    }

    fn drain_oldnew(&mut self) {
        if self.old_text.is_empty() && self.new_text.is_empty() {
            return;
        }

        // FIXME: This should be enqueued as a future containing the refiner::format() call
        let mut output = String::new();
        for line in refiner::format(&self.old_text, &self.new_text) {
            output.push_str(&line);
            output.push('\n');
        }
        self.queue_putter.send(output).unwrap();

        self.old_text.clear();
        self.new_text.clear();
    }

    fn drain_plain(&mut self) {
        if self.plain_text.is_empty() {
            return;
        }

        // FIXME: Create an already-resolved future returning this text, then
        // store that future in our queue.
        self.queue_putter
            .send(String::from(&self.plain_text))
            .unwrap();

        self.plain_text.clear();
    }

    fn consume_plain_line(&mut self, line: &str) {
        self.drain_oldnew();
        self.plain_text.push_str(line);
        self.plain_text.push('\n');
    }

    fn consume_old_line(&mut self, line: &str) {
        self.drain_plain();
        self.old_text.push_str(&line[1..]);
        self.old_text.push('\n');
    }

    fn consume_new_line(&mut self, line: &str) {
        self.drain_plain();
        self.new_text.push_str(&line[1..]);
        self.new_text.push('\n');
    }

    fn consume_no_eof_newline_marker(&mut self) {
        if !self.new_text.is_empty() {
            // New section comes after old, so if we get in here it's a new
            // section that doesn't end in a newline. Remove its trailing
            // newline.
            assert!(self.new_text.pop().unwrap() == '\n');
            return;
        }

        if !self.old_text.is_empty() {
            // Old text doesn't end in a newline, remove its trailing newline
            assert!(self.old_text.pop().unwrap() == '\n');
            return;
        }

        // It's a piece of unchanged text that doesn't end in a newline, just
        // consume the colorized marker as plain text
        self.consume_plain_line(&format!(
            "{}{}{}",
            NO_EOF_NEWLINE_COLOR, &NO_EOF_NEWLINE_MARKER, NORMAL
        ))
    }

    pub fn consume_line(&mut self, line: String) {
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
