use threadpool::ThreadPool;

use crate::line_collector::LinesHighlighter;
use crate::string_future::StringFuture;

struct HunkLinesHighlighter<'a> {
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

    fn consume_line(&mut self, line: &str) -> Result<(), String> {
        todo!()
    }

    fn get_highlighted_if_done(&mut self) -> Option<StringFuture> {
        todo!()
    }
}
