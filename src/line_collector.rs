use crate::ansi::without_ansi_escape_codes;
use crate::commit_line::format_commit_line;
use crate::conflicts_highlighter::ConflictsHighlighter;
use crate::file_highlighter::FileHighlighter;
use crate::hunk_highlighter::HunkLinesHighlighter;
use crate::io::ErrorKind;
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter};
use crate::refiner::Formatter;
use crate::rename_highlighter::RenameHighlighter;
use once_cell::sync::Lazy;
use std::io::{self, BufWriter, Write};
use std::process::{self, exit};
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::{constants::*, string_future::StringFuture};
use threadpool::ThreadPool;

static STATIC_HEADER_PREFIXES: Lazy<Vec<(&'static str, &'static str)>> = Lazy::new(|| {
    vec![
        ("diff ", FAINT),
        ("index ", FAINT),
        ("Binary files ", BOLD),
        ("copy from ", FAINT),
        ("copy to ", BOLD),
        ("rename from ", FAINT),
        ("rename to ", BOLD),
        ("similarity index ", FAINT),
        ("new file mode ", FAINT),
        ("deleted file mode ", FAINT),
    ]
});

/// This is the `\ No newline at end of file` string. But since it can come in
/// not-English as well as English, we take it from the input and store it in
/// this variable. None means we don't know yet.
///
/// See also: https://github.com/walles/riff/issues/39
pub(crate) static NO_EOF_NEWLINE_MARKER_HOLDER: Lazy<Arc<Mutex<Option<String>>>> =
    Lazy::new(|| Arc::new(Mutex::<Option<String>>::new(None)));

#[must_use]
fn get_fixed_highlight(line: &str) -> Option<&str> {
    for static_header_prefix in STATIC_HEADER_PREFIXES.iter() {
        let prefix = static_header_prefix.0;
        if line.starts_with(prefix) {
            return Some(static_header_prefix.1);
        }
    }

    return None;
}

/// Write the string bytes to the stream.
fn print<W: io::Write + Send>(stream: &mut BufWriter<W>, text: &str, strip_color: bool) {
    let result = if strip_color {
        let bytes = text.as_bytes().to_vec();
        let bytes = without_ansi_escape_codes(&bytes);
        stream.write_all(&bytes)
    } else {
        stream.write_all(text.as_bytes())
    };

    if let Err(error) = result {
        if error.kind() == ErrorKind::BrokenPipe {
            // This is fine, somebody probably just quit their pager before it
            // was done reading our output.
            exit(0);
        }

        panic!("Error writing diff to pager: {:?}", error);
    }
}

/**
The way this thing works from the outside is that you initialize it with an
output stream, you pass it one line of input at a time, and it writes
formatted lines to the output stream.

From the inside, it will collect blocks of either diff lines or not-diff-lines.

The not-diff-lines blocks will be enqueued for printing by the printing thread.

The diff lines blocks will also be enqueued for printing, but the actual diffing
will happen in background threads.
*/
pub(crate) struct LineCollector {
    lines_highlighter: Option<Box<dyn LinesHighlighter>>,

    /// Headers and stuff that we just want printed, not part of a diff
    plain_text: String,

    /// Set to true when we see the first diff line. The second diff line and
    /// onwards will come with highlighted backgrounds, based on this value.
    diff_seen: bool,

    consumer_thread: Option<JoinHandle<()>>,
    thread_pool: ThreadPool,

    // FIXME: I'd rather have had a SyncSender of some trait here. That would
    // enable us to have two separate result implementations, one which just
    // returns a string and another that does a background computation first.
    // But I failed to figure out how when I tried, more Googling needed!
    print_queue_putter: SyncSender<StringFuture>,

    formatter: Formatter,
}

impl Drop for LineCollector {
    fn drop(&mut self) {
        // Flush outstanding lines
        self.drain_plain();

        if self.lines_highlighter.is_some() {
            let result = self
                .lines_highlighter
                .as_mut()
                .unwrap()
                .consume_eof(&self.thread_pool);
            if let Err(error) = result {
                self.lines_highlighter = None;
                eprintln!("ERROR at end of input: {error}");
                process::exit(1);
            }

            for highlight in result.unwrap() {
                self.print_queue_putter.send(highlight).unwrap();
            }
        }

        // Tell the consumer thread to drain and quit. Sending an empty string
        // like this is the secret handshake for requesting a shutdown.
        self.print_queue_putter
            .send(StringFuture::from_string("".to_string()))
            .unwrap();

        // Wait for the consumer thread to finish
        // https://stackoverflow.com/q/57670145/473672
        self.consumer_thread.take().map(JoinHandle::join);
    }
}

impl LineCollector {
    pub fn new<W: io::Write + Send + 'static>(
        output: W,
        color: bool,
        formatter: Formatter,
    ) -> LineCollector {
        // This is how many entries we can look ahead. An "entry" in this case
        // being either a plain text section or an oldnew section.
        //
        // Benchmark timings with different multipliers on an 8 logical cores
        // machine with a 6.6M lines / 208MB diff:
        //
        // 500x => 5.68s  <-- Not much better than 100x
        // 100x => 5.71s  <-- Somewhat better than 50x
        //  50x => 5.98s
        //  10x >= 7.41s  <-- Much worse than 50x
        let queue_size = num_cpus::get() * 100;

        // Allocate a queue where we can push our futures to the consumer thread
        let (queue_putter, queue_getter): (SyncSender<StringFuture>, Receiver<StringFuture>) =
            sync_channel(queue_size);

        // This thread takes futures and prints their results
        let thread_builder = thread::Builder::new().name("Output Printer Thread".to_string());
        let consumer = thread_builder
            .spawn(move || {
                let mut output = BufWriter::new(output);

                loop {
                    if let Ok(mut print_me) = queue_getter.recv() {
                        if print_me.is_empty() {
                            // Secret handshake received, done!
                            break;
                        }
                        print(&mut output, print_me.get(), !color);
                    }
                }
            })
            .unwrap();

        return LineCollector {
            lines_highlighter: None,
            plain_text: String::from(""),
            diff_seen: false,

            consumer_thread: Some(consumer),
            thread_pool: ThreadPool::new(num_cpus::get()),
            print_queue_putter: queue_putter,

            formatter,
        };
    }

    fn drain_plain(&mut self) {
        if self.plain_text.is_empty() {
            return;
        }

        // Enqueue an already-resolved future
        self.print_queue_putter
            .send(StringFuture::from_string(String::from(&self.plain_text)))
            .unwrap();

        self.plain_text.clear();
    }

    fn consume_plain_line(&mut self, line: &str) {
        assert!(self.lines_highlighter.is_none());
        self.plain_text.push_str(line);
        self.plain_text.push('\n');
    }

    /// Like consume_plain_line(), but without outputting any trailing linefeed.
    fn consume_plain_linepart(&mut self, linepart: &str) {
        assert!(self.lines_highlighter.is_none());
        self.plain_text.push_str(linepart);
    }

    /// The line parameter is expected *not* to end in a newline.
    ///
    /// Returns an error message on trouble.
    pub fn consume_line(
        &mut self,
        raw_line: &[u8],
        strip_incoming_formatting: bool,
    ) -> Result<(), String> {
        let result = self.consume_line_internal(raw_line, strip_incoming_formatting);
        if result.is_ok() {
            return result;
        }

        // Invariant: This was an error

        self.drain_plain();

        // This one just failed, so it's out
        self.lines_highlighter = None;

        let line = without_ansi_escape_codes(raw_line);
        let line = String::from_utf8_lossy(&line).to_string();
        self.print_queue_putter
            .send(StringFuture::from_string(format!(
                "{PARSE_ERROR}{line}{NORMAL}"
            )))
            .unwrap();

        return result;
    }

    fn consume_line_internal(
        &mut self,
        raw_line: &[u8],
        strip_incoming_formatting: bool,
    ) -> Result<(), String> {
        // Strip out incoming ANSI formatting. This enables us to highlight
        // already-colored input.
        let line = if strip_incoming_formatting {
            let stripped = without_ansi_escape_codes(raw_line);
            String::from_utf8_lossy(&stripped).to_string()
        } else {
            String::from_utf8_lossy(raw_line).to_string()
        };

        if line.starts_with('\\') {
            {
                // Store the "\ No newline at end of file" string however it is
                // phrased in this particular diff.
                //
                // Note that this must be done before consuming it below so we
                // know it's set before the consumer decides it wants to emit a
                // copy. Otherwise we get a race condition and we don't want
                // that.
                let mut no_eof_newline_marker = NO_EOF_NEWLINE_MARKER_HOLDER.lock().unwrap();
                *no_eof_newline_marker = Some(line.to_string());
            }
        }

        if let Some(lines_highlighter) = self.lines_highlighter.as_mut() {
            let result = lines_highlighter.consume_line(&line, &self.thread_pool);
            if let Err(error) = result {
                self.lines_highlighter = None;
                return Err(error);
            }

            let response = result.unwrap();
            for highlight in response.highlighted {
                self.print_queue_putter.send(highlight).unwrap();
            }

            match response.line_accepted {
                LineAcceptance::AcceptedWantMore => return Ok(()),
                LineAcceptance::AcceptedDone => {
                    self.lines_highlighter = None;
                    return Ok(());
                }
                LineAcceptance::RejectedDone => {
                    self.lines_highlighter = None;

                    // Do not return, fall back to the no-handler code below
                }
            }
        }

        if let Some(file_highlighter) = FileHighlighter::from_line(&line) {
            self.drain_plain();
            self.lines_highlighter = Some(Box::new(file_highlighter));
            return Ok(());
        }

        if let Some(hunk_highlighter) =
            HunkLinesHighlighter::from_line(&line, self.formatter.clone())
        {
            self.drain_plain();
            self.lines_highlighter = Some(Box::new(hunk_highlighter));
            return Ok(());
        }

        if let Some(conflicts_highlighter) = ConflictsHighlighter::from_line(&line) {
            self.drain_plain();
            self.lines_highlighter = Some(Box::new(conflicts_highlighter));
            return Ok(());
        }

        if let Some(rename_highlighter) = RenameHighlighter::from_line(&line) {
            self.drain_plain();
            self.lines_highlighter = Some(Box::new(rename_highlighter));
            return Ok(());
        }

        if line.starts_with("diff") {
            self.diff_seen = true;
        }

        if let Some(fixed_highlight) = get_fixed_highlight(&line) {
            self.consume_plain_linepart(fixed_highlight);
            self.consume_plain_linepart(&line);
            self.consume_plain_line(NORMAL); // consume_plain_line() will add a linefeed to the output
            return Ok(());
        }

        if line.starts_with("commit") {
            self.consume_plain_line(&format_commit_line(&line, self.diff_seen));
            return Ok(());
        }

        if line.starts_with('\\') {
            // "\ No newline at end of file"
            self.consume_plain_line(&format!("{NO_EOF_NEWLINE_COLOR}{line}{NORMAL}"));
            return Ok(());
        }

        if line.is_empty() {
            self.consume_plain_line("");
            return Ok(());
        }

        self.consume_plain_line(String::from_utf8_lossy(raw_line).as_ref());
        return Ok(());
    }
}
