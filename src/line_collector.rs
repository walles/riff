use crate::ansi::remove_ansi_escape_codes;
use crate::commit_line::format_commit_line;
use crate::hunk_header::HunkHeader;
use crate::io::ErrorKind;
use crate::refiner::to_highlighted_tokens;
use crate::token_collector::{
    lowlight_timestamp, render, unhighlight_git_prefix, LINE_STYLE_NEW_FILENAME,
    LINE_STYLE_OLD_FILENAME,
};
use std::io::{self, BufWriter, Write};
use std::process::exit;
use std::sync::mpsc::{sync_channel, Receiver, SyncSender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

use crate::{constants::*, refiner};
use threadpool::ThreadPool;

lazy_static! {
    static ref STATIC_HEADER_PREFIXES: Vec<(&'static str, &'static str)> = vec![
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
    ];

    /// This is the `\ No newline at end of file` string. But since it can come
    /// in not-English as well as English, we take it from the input and store it
    /// in this variable. None means we don't know yet.
    ///
    /// See also: https://github.com/walles/riff/issues/39
    pub(crate) static ref NO_EOF_NEWLINE_MARKER_HOLDER: std::sync::Arc<
        std::sync::Mutex<std::option::Option<std::string::String>>,
    > = Arc::new(Mutex::<Option<String>>::new(None));
}

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

trait LinesHighlighter {
    /// Consume one line of input.
    ///
    /// In case this call returns an error, this whole object will be invalid.
    /// afterwards.
    fn consume_line(&mut self, line: &str) -> Result<(), String>;

    /// If we're done, return the highlighted result.
    ///
    /// After this call has returned a result, this whole object will be invalid.
    fn get_highlighted_if_done(&mut self) -> Option<StringFuture>;
}

/**
A StringFuture can perform diffing in a background thread.

Doing get() on a future that isn't done yet will block until the result is
available.
*/
struct StringFuture {
    // This field is only valid if we're done with the result_receiver (next
    // field)
    result: String,

    // If available, get() will await a result on this receiver, then populate
    // the result field and return it
    result_receiver: Option<Receiver<String>>,
}

impl StringFuture {
    /// Create an already-finished future
    pub fn from_string(result: String) -> StringFuture {
        return StringFuture {
            result,
            result_receiver: None,
        };
    }

    /// Call get() to get the result of this diff
    pub fn from_oldnew(
        old_text: String,
        new_text: String,
        thread_pool: &ThreadPool,
    ) -> StringFuture {
        // Create a String channel
        let (sender, receiver): (SyncSender<String>, Receiver<String>) = sync_channel(1);

        // Start diffing in a thread
        thread_pool.execute(move || {
            let mut result = String::new();
            for line in refiner::format(&old_text, &new_text) {
                result.push_str(&line);
                result.push('\n');
            }

            // Done, channel the result!
            sender.send(result).unwrap();
        });

        return StringFuture {
            result: "".to_string(),
            result_receiver: Some(receiver),
        };
    }

    #[allow(clippy::wrong_self_convention)]
    pub fn is_empty(&mut self) -> bool {
        return self.get().is_empty();
    }

    pub fn get(&mut self) -> &str {
        // If the result is still pending...
        if let Some(receiver) = &self.result_receiver {
            // ... wait for it
            self.result = receiver.recv().unwrap();
            self.result_receiver = None;
        }

        return &self.result;
    }
}

/// Consume some lines, return some highlighted text
trait LinesHighlighter: Sized {
    /// Create a new LinesHighlighter from a line of input.
    ///
    /// Returns None if this line doesn't start a new LinesHighlighter.
    fn from_line(line: &str) -> Option<Self>;

    /// Consume one line of input.
    ///
    /// In case this call returns an error, this whole object will be invalid.
    /// afterwards.
    fn consume_line(&mut self, line: &str) -> Result<(), String>;

    /// If we're done, return the highlighted result.
    ///
    /// After this call has returned a result, this whole object will be invalid.
    fn get_highlighted_if_done(&mut self) -> Option<StringFuture>;
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
pub struct LineCollector {
    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_old_lines: usize,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_new_lines: usize,

    /// The old text of a diff, if any. Includes `-` lines only.
    old_text: String,

    /// The new text of a diff, if any. Includes `+` lines only.
    new_text: String,

    /// Headers and stuff that we just want printed, not part of a diff
    plain_text: String,

    /// Set to true when we see the first diff line. The second diff line and
    /// onwards will come with highlighted backgrounds, based on this value.
    diff_seen: bool,

    consumer_thread: Option<JoinHandle<()>>,
    diffing_threads: ThreadPool,

    // FIXME: I'd rather have had a SyncSender of some trait here. That would
    // enable us to have two separate result implementations, one which just
    // returns a string and another that does a background computation first.
    // But I failed to figure out how when I tried, more Googling needed!
    queue_putter: SyncSender<StringFuture>,
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
            .send(StringFuture::from_string("".to_string()))
            .unwrap();

        // Wait for the consumer thread to finish
        // https://stackoverflow.com/q/57670145/473672
        self.consumer_thread.take().map(JoinHandle::join);
    }
}

impl LineCollector {
    pub fn new<W: io::Write + Send + 'static>(output: W) -> LineCollector {
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
                        print(&mut output, print_me.get());
                    }
                }
            })
            .unwrap();

        return LineCollector {
            expected_old_lines: 0,
            expected_new_lines: 0,
            old_text: String::from(""),
            new_text: String::from(""),
            plain_text: String::from(""),
            diff_seen: false,

            consumer_thread: Some(consumer),
            diffing_threads: ThreadPool::new(num_cpus::get()),
            queue_putter,
        };
    }

    fn drain_oldnew(&mut self) {
        if self.old_text.is_empty() && self.new_text.is_empty() {
            return;
        }

        self.queue_putter
            .send(StringFuture::from_oldnew(
                self.old_text.clone(),
                self.new_text.clone(),
                &self.diffing_threads,
            ))
            .unwrap();

        self.old_text.clear();
        self.new_text.clear();
    }

    fn drain_plain(&mut self) {
        if self.plain_text.is_empty() {
            return;
        }

        // Enqueue an already-resolved future
        self.queue_putter
            .send(StringFuture::from_string(String::from(&self.plain_text)))
            .unwrap();

        self.plain_text.clear();
    }

    fn consume_plain_line(&mut self, line: &str) {
        self.drain_oldnew();
        self.plain_text.push_str(line);
        self.plain_text.push('\n');
    }

    /// Like consume_plain_line(), but without outputting any trailing linefeed.
    fn consume_plain_linepart(&mut self, linepart: &str) {
        self.drain_oldnew();
        self.plain_text.push_str(linepart);
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

    fn consume_no_eof_newline_marker(&mut self, no_eof_newline_marker: &str) {
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
            NO_EOF_NEWLINE_COLOR, no_eof_newline_marker, NORMAL
        ))
    }

    /// Returns an error message on failure
    #[must_use]
    pub fn consume_plusminus_header(&mut self, line: &str) -> Option<&str> {
        if let Some(old_name) = line.strip_prefix("--- ") {
            self.old_text.clear();
            self.old_text.push_str(old_name);
            return None;
        }

        if let Some(new_name) = line.strip_prefix("+++ ") {
            if self.old_text.is_empty() {
                // We got +++ not preceded by ---, WTF?
                return None;
            }

            self.new_text.clear();
            self.new_text.push_str(new_name);
        } else {
            return Some("Got a plusminus header that doesn't start with --- or +++");
        }

        if self.old_text == "/dev/null" {
            let new_name = self.new_text.clone();
            self.old_text.clear();
            self.new_text.clear();

            self.consume_plain_linepart(FAINT);
            self.consume_plain_linepart("--- /dev/null");
            self.consume_plain_line(NORMAL);

            self.consume_plain_linepart(BOLD);
            self.consume_plain_linepart("+++ ");
            self.consume_plain_linepart(&new_name);
            self.consume_plain_line(NORMAL);
            return None;
        }

        if self.new_text == "/dev/null" {
            let old_name = self.old_text.clone();
            self.old_text.clear();
            self.new_text.clear();

            self.consume_plain_linepart(BOLD);
            self.consume_plain_linepart("--- ");
            self.consume_plain_linepart(&old_name);
            self.consume_plain_line(NORMAL);

            self.consume_plain_linepart(FAINT);
            self.consume_plain_linepart("+++ /dev/null");
            self.consume_plain_line(NORMAL);

            return None;
        }

        let (mut old_tokens, mut new_tokens, _, _) =
            to_highlighted_tokens(&self.old_text, &self.new_text);
        self.old_text.clear();
        self.new_text.clear();

        lowlight_timestamp(&mut old_tokens);
        unhighlight_git_prefix(&mut old_tokens);
        lowlight_timestamp(&mut new_tokens);
        unhighlight_git_prefix(&mut new_tokens);

        let old_filename = render(&LINE_STYLE_OLD_FILENAME, old_tokens);
        let new_filename = render(&LINE_STYLE_NEW_FILENAME, new_tokens);
        self.consume_plain_line(&old_filename);
        self.consume_plain_line(&new_filename);

        return None;
    }

    /// The line parameter is expected *not* to end in a newline.
    ///
    /// Returns an error message on trouble.
    #[must_use]
    pub fn consume_line(&mut self, line: &mut Vec<u8>) -> Option<&str> {
        // Strip out incoming ANSI formatting. This enables us to highlight
        // already-colored input.
        remove_ansi_escape_codes(line);
        let line = String::from_utf8_lossy(line);

        if line.starts_with('\\') {
            {
                // Store the "\ No newline at end of file" string however it is
                // phrased in this particular diff.
                //
                // Note that this must be done before consuming it below so we
                // know it's set before the consumer decides it wants to emit a
                // copy. Otherwise we get a race condition and we don't want
                // that.
                //
                // We do it in a block to release the lock as soon as possible.
                let mut no_eof_newline_marker = NO_EOF_NEWLINE_MARKER_HOLDER.lock().unwrap();
                *no_eof_newline_marker = Some(line.to_string());
            }

            // Consume the marker *after* we just updated our
            // no_eof_newline_marker above. In the other order we'd have a race
            // condition.
            self.consume_no_eof_newline_marker(&line);

            return None;
        }

        if self.expected_old_lines + self.expected_new_lines > 0 {
            if line.starts_with('-') {
                self.expected_old_lines -= 1;
                self.consume_old_line(&line);
                return None;
            }

            if line.starts_with('+') {
                self.expected_new_lines -= 1;
                self.consume_new_line(&line);
                return None;
            }

            if !line.is_empty() && !line.starts_with(' ') {
                return Some("Non-empty line should have started with ' ', '-' or '+'");
            }

            self.expected_old_lines -= 1;
            self.expected_new_lines -= 1;
            self.consume_plain_line(&line);
            return None;
        }

        if let Some(hunk_header) = HunkHeader::parse(&line) {
            self.expected_new_lines = hunk_header.new_linecount;
            self.expected_old_lines = hunk_header.old_linecount;
            self.consume_plain_line(&hunk_header.render());
            return None;
        }

        if line.starts_with("diff") {
            self.diff_seen = true;
        }

        if let Some(fixed_highlight) = get_fixed_highlight(&line) {
            self.consume_plain_linepart(fixed_highlight);
            self.consume_plain_linepart(&line);
            self.consume_plain_line(NORMAL); // consume_plain_line() will add a linefeed to the output
            return None;
        }

        if line.starts_with("commit") {
            self.consume_plain_line(&format_commit_line(&line, self.diff_seen));
            return None;
        }

        if line.starts_with("--- ") || line.starts_with("+++ ") {
            return self.consume_plusminus_header(&line);
        }

        if line.is_empty() {
            self.consume_plain_line("");
            return None;
        }

        self.consume_plain_line(&line);
        return None;
    }
}
