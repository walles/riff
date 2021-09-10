use crate::io::ErrorKind;
use std::io::{self, BufWriter, Write};
use std::process::exit;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::thread::{self, JoinHandle};

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

fn print<W: io::Write + Send>(stream: &mut BufWriter<W>, text: &str) {
    if let Err(error) = stream.write_all(text.as_bytes()) {
        if error.kind() == ErrorKind::BrokenPipe {
            // This is fine, somebody probably just quit their pager before it
            // was done reading our output.
            exit(0);
        }

        panic!("Error writing diff to pager: {:?}", error);
    }
}

// FIXME: Don't call this "Stringinator"!!
struct Stringinator {
    // FIXME: This should be an Option<String>, which only contains the result
    // if the computation is done
    result: String,
}

impl Stringinator {
    pub fn from_string(result: String) -> Stringinator {
        return Stringinator { result };
    }

    pub fn is_empty(&self) -> bool {
        return self.result.is_empty();
    }

    pub fn get(&self) -> &str {
        return &self.result;
    }
}

pub struct LineCollector {
    old_text: String,
    new_text: String,
    plain_text: String,
    consumer_thread: Option<JoinHandle<()>>,

    // FIXME: I'd rather have had a SyncSender of some trait here. That would
    // enable us to have two separate result implementations, one which just
    // returns a string and another that does a background computation first.
    // But I failed to figure out how when I tried, more Googling needed!
    queue_putter: SyncSender<Stringinator>,
}

impl Drop for LineCollector {
    fn drop(&mut self) {
        // Flush any outstanding lines. This can be done in any order, at most
        // one of them is going to do anything anyway.
        self.drain_oldnew();
        self.drain_plain();

        // Tell the consumer thread to drain and quit. Sending an empty string
        // like this is the secret handshake for requesting a shutdown.
        self.queue_putter
            .send(Stringinator::from_string("".to_string()))
            .unwrap();

        // Wait for the consumer thread to finish
        // https://stackoverflow.com/q/57670145/473672
        self.consumer_thread.take().map(JoinHandle::join);
    }
}

impl LineCollector {
    pub fn new<W: io::Write + Send + 'static>(output: W) -> LineCollector {
        // Allocate a queue where we can push our futures to the consumer thread
        //
        // FIXME: The queue should be bounded to 2x the number of logical CPUs.
        // 1x for the entries that need CPU time for diffing, and another 1x
        // that just contain text to print and won't need any processing time.
        let (queue_putter, queue_getter): (SyncSender<Stringinator>, Receiver<Stringinator>) =
            sync_channel(16);

        // This thread takes futures and prints their results
        let consumer = thread::spawn(move || {
            let mut output = BufWriter::new(output);

            loop {
                if let Ok(print_me) = queue_getter.recv() {
                    if print_me.is_empty() {
                        // Secret handshake received, done!
                        break;
                    }
                    print(&mut output, print_me.get());
                }
            }
        });

        return LineCollector {
            old_text: String::from(""),
            new_text: String::from(""),
            plain_text: String::from(""),
            consumer_thread: Some(consumer),
            queue_putter,
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
        self.queue_putter
            .send(Stringinator::from_string(output))
            .unwrap();

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
            .send(Stringinator::from_string(String::from(&self.plain_text)))
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
