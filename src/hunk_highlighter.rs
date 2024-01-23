use threadpool::ThreadPool;

use crate::hunk_header::HunkHeader;
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::refiner;
use crate::string_future::StringFuture;

pub(crate) struct HunkLinesHighlighter {
    // This will have to be rendered at the top of our returned result.
    hunk_header: Option<String>,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_old_lines: usize,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_new_lines: usize,

    /// The old text of a diff, if any. Includes `-` lines only.
    old_text: String,

    /// The new text of a diff, if any. Includes `+` lines only.
    new_text: String,
}

impl<'a> LinesHighlighter<'a> for HunkLinesHighlighter {
    fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(hunk_header) = HunkHeader::parse(line) {
            let expected_old_lines = hunk_header.old_linecount;
            let expected_new_lines = hunk_header.new_linecount;
            let old_text = String::new();
            let new_text = String::new();

            return Some(HunkLinesHighlighter {
                hunk_header: Some(hunk_header.render()),
                expected_old_lines,
                expected_new_lines,
                old_text,
                new_text,
            });
        }

        return None;
    }

    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        let mut return_me = vec![];

        // Always start by returning the hunk header
        if let Some(hunk_header) = &self.hunk_header {
            return_me.push(StringFuture::from_string(hunk_header.to_string() + "\n"));
            self.hunk_header = None;
        }

        // "\ No newline at end of file"
        if line.starts_with('\\') {
            if !self.new_text.is_empty() {
                // New section comes after old, so if we get in here it's a new
                // section that doesn't end in a newline. Remove its trailing
                // newline.
                assert!(self.new_text.pop().unwrap() == '\n');

                // If we got here we're definitely done
                return_me.append(&mut self.drain_old_new(thread_pool));

                return Ok(Response {
                    line_accepted: LineAcceptance::AcceptedDone,
                    highlighted: return_me,
                });
            }

            if !self.old_text.is_empty() {
                // Old text doesn't end in a newline, remove its trailing newline
                assert!(self.old_text.pop().unwrap() == '\n');
                return Ok(Response {
                    line_accepted: LineAcceptance::AcceptedWantMore,
                    highlighted: return_me,
                });
            }

            return Err(
                "Got '\\ No newline at end of file' without any preceding text".to_string(),
            );
        }

        if self.expected_new_lines + self.expected_old_lines == 0 {
            return Ok(Response {
                line_accepted: LineAcceptance::RejectedDone,
                highlighted: return_me,
            });
        }

        if line.starts_with('-') {
            self.expected_old_lines -= 1;
            self.old_text.push_str(&line[1..]);
            self.old_text.push('\n');

            return Ok(Response {
                // Note that even if we don't "expect" any more lines, we could
                // still get "\ No newline at end of file" lines, so we have to
                // ask for more here, and we can't drain_old_new() just yet.
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: return_me,
            });
        }

        if line.starts_with('+') {
            self.expected_new_lines -= 1;
            self.new_text.push_str(&line[1..]);
            self.new_text.push('\n');

            return Ok(Response {
                // Note that even if we don't "expect" any more lines, we could
                // still get "\ No newline at end of file" lines, so we have to
                // ask for more here, and we can't drain_old_new() just yet.
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: return_me,
            });
        }

        // Context lines
        if line.starts_with(' ') || line.is_empty() {
            // FIXME: Consider whether we should be coalescing the plain lines?
            // Maybe that would improve performance? Measure and find out!
            return_me.append(&mut self.drain_old_new(thread_pool));

            self.expected_old_lines -= 1;
            self.expected_new_lines -= 1;

            return_me.push(StringFuture::from_string(line.to_string() + "\n"));

            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: return_me,
            });
        }

        return Err("Hunk line must start with '-' or '+'".to_string());
    }

    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        if self.expected_old_lines != 0 || self.expected_new_lines != 0 {
            return Err(format!(
                "Still expecting {} old lines and {} new lines, but got EOF",
                self.expected_old_lines, self.expected_new_lines
            ));
        }

        return Ok(self.drain_old_new(thread_pool));
    }
}

impl HunkLinesHighlighter {
    fn drain_old_new(&mut self, thread_pool: &ThreadPool) -> Vec<StringFuture> {
        if self.old_text.is_empty() && self.new_text.is_empty() {
            return vec![];
        }

        let old_text = self.old_text.clone();
        let new_text = self.new_text.clone();
        self.old_text.clear();
        self.new_text.clear();
        let return_me = StringFuture::from_function(
            move || {
                let mut result = String::new();
                for line in refiner::format(&old_text, &new_text) {
                    result.push_str(&line);
                    result.push('\n');
                }

                result
            },
            thread_pool,
        );

        return vec![return_me];
    }
}

#[cfg(test)]
mod tests {
    use crate::lines_highlighter::LineAcceptance;

    use super::*;

    /// Based on `testdata/adds-only.diff`
    #[test]
    fn test_happy_path() {
        let thread_pool = ThreadPool::new(1);

        let mut test_me = HunkLinesHighlighter::from_line("@@ -1,2 +1,2 @@").unwrap();

        // First call to consume_line() should get us the hunk header
        let mut result = test_me
            .consume_line("-Hello, my name is Johan", &thread_pool)
            .unwrap();

        // Expect to get the hunk header back immediately, no matter what else
        // we got.
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedWantMore);
        assert_eq!(result.highlighted.len(), 1);
        assert_eq!(
            result.highlighted[0].get(),
            "\u{1b}[36m@@ -1,2 +1,2 @@\u{1b}[0m\n"
        );

        let result = test_me
            .consume_line("+Hello, my first name is Johan", &thread_pool)
            .unwrap();
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedWantMore);
        assert_eq!(result.highlighted.len(), 0);

        let mut result = test_me.consume_line(" I like pie.", &thread_pool).unwrap();
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedDone);

        assert_eq!(result.highlighted.len(), 2);
        assert_eq!(
            result.highlighted[0].get(),
            concat!(
                "\u{1b}[2m\u{1b}[31m-Hello, my name is Johan\u{1b}[0m\n",
                "\u{1b}[2m\u{1b}[32m+\u{1b}[0mHello, my \u{1b}[7m\u{1b}[32mfirst \u{1b}[0mname is Johan\n"
            )
        );
        assert_eq!(result.highlighted[1].get(), " I like pie.\n");
    }
}
