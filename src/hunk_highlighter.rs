use threadpool::ThreadPool;

use crate::hunk_header::HunkHeader;
use crate::line_collector::LinesHighlighter;
use crate::refiner;
use crate::string_future::StringFuture;

pub(crate) struct HunkLinesHighlighter<'a> {
    // This will have to be rendered at the top of our returned result.
    hunk_header: HunkHeader,

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

impl<'a> LinesHighlighter<'a> for HunkLinesHighlighter<'a> {
    fn from_line(line: &str, thread_pool: &'a ThreadPool) -> Option<Self>
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

    fn consume_line(&mut self, line: &str) -> Result<(), String> {
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

            return Err(
                "Got '\\ No newline at end of file' without any preceding text".to_string(),
            );
        }

        self.expected_old_lines = 0;
        self.expected_new_lines = 0;
        return Err("Hunk line must start with '-' or '+'".to_string());
    }

    fn get_highlighted_if_done(&mut self) -> Option<StringFuture> {
        if self.expected_new_lines + self.expected_old_lines == 0 {
            return None;
        }

        let header = self.hunk_header.render();
        let old = self.old_text.clone();
        let new = self.new_text.clone();
        return Some(StringFuture::from_function(
            move || {
                let mut result = String::new();
                result.push_str(&header);
                result.push('\n');

                for line in refiner::format(&old, &new) {
                    result.push_str(&line);
                    result.push('\n');
                }

                result
            },
            self.thread_pool,
        ));
    }
}
