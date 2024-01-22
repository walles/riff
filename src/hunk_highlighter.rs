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

    diffing_threads: &'a ThreadPool,
}

impl<'a> LinesHighlighter for HunkLinesHighlighter<'a> {
    fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        todo!()
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

        self.expected_old_lines = 0;
        self.expected_new_lines = 0;
        return Err("Hunk line must start with '-' or '+'");
    }

    fn get_highlighted_if_done(&mut self) -> Option<StringFuture> {
        todo!()
    }
}
