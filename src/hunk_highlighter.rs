use threadpool::ThreadPool;

use crate::hunk_header::HunkHeader;
use crate::line_collector::LinesHighlighter;
use crate::string_future::StringFuture;

pub(crate) struct HunkLinesHighlighter<'a> {
    // This will have to be rendered at the top of our returned result.
    hunk_header: HunkHeader<'a>,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_old_lines: usize,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_new_lines: usize,

    /// The old text of a diff, if any. Includes `-` lines only.
    old_text: String,

    /// The new text of a diff, if any. Includes `+` lines only.
    new_text: String,

    thread_pool: &'a ThreadPool,
}

impl<'a> LinesHighlighter for HunkLinesHighlighter<'a> {
    fn from_line(line: &str, thread_pool: &ThreadPool) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(hunk_header) = HunkHeader::parse(line) {
            let expected_old_lines = hunk_header.old_linecount;
            let expected_new_lines = hunk_header.new_linecount;
            let old_text = String::new();
            let new_text = String::new();

            return Some(HunkLinesHighlighter {
                hunk_header,
                expected_old_lines,
                expected_new_lines,
                old_text,
                new_text,
                thread_pool,
            });
        }

        return None;
    }

    fn consume_line(&mut self, line: &str) -> Result<(), &str> {
        if self.expected_old_lines + self.expected_new_lines == 0 {
            panic!("No more lines expected")
        }

        if line.starts_with('-') {
            self.expected_old_lines -= 1;
            self.old_text.push_str(&line[1..]);
            self.old_text.push('\n');
            return Ok(());
        }

        if line.starts_with('+') {
            self.expected_new_lines -= 1;
            self.new_text.push_str(&line[1..]);
            self.new_text.push('\n');
            return Ok(());
        }

        // "\ No newline at end of file"
        if line.starts_with('\\') {
            if !self.new_text.is_empty() {
                // New section comes after old, so if we get in here it's a new
                // section that doesn't end in a newline. Remove its trailing
                // newline.
                assert!(self.new_text.pop().unwrap() == '\n');
                return Ok(());
            }

            if !self.old_text.is_empty() {
                // Old text doesn't end in a newline, remove its trailing newline
                assert!(self.old_text.pop().unwrap() == '\n');
                return Ok(());
            }

            return Err("Got '\\ No newline at end of file' without any preceding text");
        }

        self.expected_old_lines = 0;
        self.expected_new_lines = 0;
        return Err("Hunk line must start with '-' or '+'");
    }

    fn get_highlighted_if_done(&mut self) -> Option<StringFuture> {
        todo!()
    }
}
