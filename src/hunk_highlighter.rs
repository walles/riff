use threadpool::ThreadPool;

use crate::conflicts_highlighter::ConflictsHighlighter;
use crate::constants::NORMAL;
use crate::constants::NO_EOF_NEWLINE_COLOR;
use crate::hunk_header::HunkHeader;
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::plusminus_lines_highlighter::PlusMinusLinesHighlighter;
use crate::string_future::StringFuture;

#[derive(Debug)]
pub(crate) struct HunkLinesHighlighter {
    lines_highlighter: Option<Box<dyn LinesHighlighter>>,

    // This will have to be rendered at the top of our returned result.
    hunk_header: Option<String>,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_line_counts: Vec<usize>,
}

impl LinesHighlighter for HunkLinesHighlighter {
    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        let mut return_me = vec![];

        // Always start by rendering the hunk header
        if let Some(hunk_header) = &self.hunk_header {
            return_me.push(StringFuture::from_string(hunk_header.to_string() + "\n"));
            self.hunk_header = None;
        }

        let prefix_length = self.expected_line_counts.len() - 1;
        let spaces_only = " ".repeat(prefix_length);
        let prefix = if line.len() >= prefix_length {
            line.split_at(prefix_length).0
        } else {
            spaces_only.as_str()
        };

        if line.starts_with('\\') {
            return_me.append(&mut self.consume_line_internal(line, thread_pool)?);
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: return_me,
            });
        }

        if !self.more_lines_expected() {
            if let Some(ref mut lines_highlighter) = self.lines_highlighter {
                let mut result = lines_highlighter.consume_eof(thread_pool)?;
                return_me.append(&mut result);
            }

            return Ok(Response {
                line_accepted: LineAcceptance::RejectedDone,
                highlighted: return_me,
            });
        }

        self.decrease_expected_line_counts(prefix)?;

        // It wasn't a nnaeof line, and we're still expecting more lines.
        return_me.append(&mut self.consume_line_internal(line, thread_pool)?);
        return Ok(Response {
            line_accepted: LineAcceptance::AcceptedWantMore,
            highlighted: return_me,
        });
    }

    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        if self.more_lines_expected() {
            return Err(format!(
                "Still expecting more lines, but got EOF: {}",
                self.describe_expected_line_counts()
            ));
        }

        return self.drain(thread_pool);
    }
}

impl HunkLinesHighlighter {
    /// Create a new LinesHighlighter from a line of input.
    ///
    /// Returns None if this line doesn't start a new LinesHighlighter.
    pub(crate) fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(hunk_header) = HunkHeader::parse(line) {
            return Some(HunkLinesHighlighter {
                hunk_header: Some(hunk_header.render()),
                expected_line_counts: hunk_header.linecounts,
                lines_highlighter: None,
            });
        }

        return None;
    }

    fn consume_line_internal(
        &mut self,
        line: &str,
        thread_pool: &ThreadPool,
    ) -> Result<Vec<StringFuture>, String> {
        let mut return_me = vec![];

        // The `- 1` here is because there's one line count per column, plus
        // one for the result. So `- 1` gives us the prefix length.
        let prefix_length = self.expected_line_counts.len() - 1;

        let spaces_only = " ".repeat(prefix_length);

        if let Some(lines_highlighter) = &mut self.lines_highlighter {
            let mut result = lines_highlighter.consume_line(line, thread_pool)?;
            return_me.append(&mut result.highlighted);
            match result.line_accepted {
                LineAcceptance::AcceptedWantMore => { /* Just keep going */ }
                LineAcceptance::AcceptedDone => {
                    self.lines_highlighter = None;
                }
                LineAcceptance::RejectedDone => {
                    // Drop our lines_highlighter and retry the line
                    self.lines_highlighter = None;
                    return_me.append(&mut self.consume_line_internal(line, thread_pool)?);
                    return Ok(return_me);
                }
            }
            return Ok(return_me);
        }

        if prefix_length == 2 {
            if let Some(highlighter) = ConflictsHighlighter::from_line(line) {
                self.lines_highlighter = Some(Box::new(highlighter));
                return Ok(return_me);
            }
        }
        if let Some(highlighter) = PlusMinusLinesHighlighter::from_line(line, prefix_length) {
            self.lines_highlighter = Some(Box::new(highlighter));
            return Ok(return_me);
        }

        // It wasn't a plusminus line (including nnaeof lines), and we're still
        // expecting more lines. It must be a context line.

        // Context lines
        if line.is_empty() || line.starts_with(&spaces_only) {
            return_me.append(&mut self.drain(thread_pool)?);

            // FIXME: Consider whether we should be coalescing the plain lines?
            // Maybe that would improve performance? Measure and find out!
            return_me.push(StringFuture::from_string(line.to_string() + "\n"));

            return Ok(return_me);
        }

        // All other cases should have been handled above
        assert!(
            line.starts_with('\\'),
            "Expected line to start with \"\\\\\": {:?}",
            line
        );

        return_me.push(StringFuture::from_string(format!(
            "{NO_EOF_NEWLINE_COLOR}{line}{NORMAL}\n"
        )));
        return Ok(return_me);
    }

    fn decrease_expected_line_counts(&mut self, prefix: &str) -> Result<(), String> {
        if prefix.contains('+') || prefix.chars().all(|c| c == ' ') {
            // Any additions always count towards the last (additions) line
            // count. Context lines (space only) also count towards the last
            // count.
            let expected_line_count = self.expected_line_counts.last_mut().unwrap();
            if *expected_line_count == 0 {
                return Err("Got more + lines than expected".to_string());
            }
            *expected_line_count -= 1;

            for (pos, plus_or_space) in prefix.chars().enumerate() {
                if plus_or_space != ' ' {
                    continue;
                }

                let expected_line_count = &mut self.expected_line_counts[pos];
                if *expected_line_count == 0 {
                    return Err(format!(
                        "Got more lines than expected for version (+ context column) {:?}",
                        pos + 1,
                    ));
                }

                *expected_line_count -= 1;
            }

            return Ok(());
        }

        // Now, also decrease any `-` columns
        for (pos, minus_or_space) in prefix.chars().enumerate() {
            if minus_or_space != '-' {
                continue;
            }

            let expected_line_count = &mut self.expected_line_counts[pos];
            if *expected_line_count == 0 {
                return Err(format!(
                    "Got more lines than expected for version (minus / space column) {:?}",
                    pos + 1,
                ));
            }

            *expected_line_count -= 1;
        }

        return Ok(());
    }

    /// Return something along the lines of `[<± > done, < ±> 3 more expected, <++> 1 more expected]`
    fn describe_expected_line_counts(&self) -> String {
        let mut return_me = String::new();
        for (pos, expected_line_count) in self.expected_line_counts.iter().enumerate() {
            let prefix = if pos == 0 { "" } else { ", " };

            let description = if pos == self.expected_line_counts.len() - 1 {
                "+".repeat(self.expected_line_counts.len() - 1)
            } else {
                let before = " ".repeat(pos);
                let after = " ".repeat(self.expected_line_counts.len() - pos - 2);
                format!("{before}±{after}")
            };

            let expectation = if *expected_line_count == 0 {
                "done".to_string()
            } else {
                // One or more additional lines expected
                format!("{} more expected", expected_line_count)
            };

            return_me.push_str(&format!("{prefix}<{description}> {expectation}"));
        }
        return format!("[{return_me}]");
    }

    fn more_lines_expected(&self) -> bool {
        for expected_line_count in &self.expected_line_counts {
            if *expected_line_count != 0 {
                return true;
            }
        }
        return false;
    }

    fn drain(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        let return_me = if let Some(lines_highlighter) = &mut self.lines_highlighter {
            lines_highlighter.consume_eof(thread_pool)?
        } else {
            vec![]
        };

        self.lines_highlighter = None;
        return Ok(return_me);
    }
}

#[cfg(test)]
mod tests {
    use crate::{line_collector::NO_EOF_NEWLINE_MARKER_HOLDER, lines_highlighter::LineAcceptance};

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
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedWantMore);

        assert_eq!(result.highlighted.len(), 2);
        assert_eq!(
            result.highlighted[0].get(),
            concat!(
                "\u{1b}[2m\u{1b}[31m-Hello, my name is Johan\u{1b}[0m\n",
                "\u{1b}[2m\u{1b}[32m+\u{1b}[0mHello, my \u{1b}[7m\u{1b}[32mfirst \u{1b}[0mname is Johan\n"
            )
        );
        assert_eq!(result.highlighted[1].get(), " I like pie.\n");

        let result = test_me.consume_eof(&thread_pool).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_decrease_expected_line_count() {
        let mut test_me = HunkLinesHighlighter::from_line("@@ -1,2 +1,2 @@").unwrap();
        assert_eq!(test_me.expected_line_counts, vec![2, 2]);

        test_me.decrease_expected_line_counts("+").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![2, 1],
            "With a + line, we should decrease the last line count"
        );

        test_me.decrease_expected_line_counts("-").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![1, 1],
            "With a - line, we should decrease the first line count"
        );
    }

    // `\ No newline at end of file` test, based on
    // `testdata/add-remove-trailing-newline.diff`.
    #[test]
    fn test_nnaeol() {
        {
            let mut no_eof_newline_marker = NO_EOF_NEWLINE_MARKER_HOLDER.lock().unwrap();
            *no_eof_newline_marker = Some("\\ No newline at end of file".to_string());
        }

        let mut test_me = HunkLinesHighlighter::from_line("@@ -1,1 +1,2 @@").unwrap();
        assert_eq!(test_me.expected_line_counts, vec![1, 2]);

        let thread_pool = ThreadPool::new(1);
        test_me.consume_line(" Hello", &thread_pool).unwrap();
        let result = test_me
            .consume_line("+No trailing newline", &thread_pool)
            .unwrap();

        // We should now be expecting the `\ No newline at end of file` line
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedWantMore);
        assert_eq!(result.highlighted.len(), 0);

        assert_eq!(test_me.expected_line_counts, vec![0, 0]);

        let mut result = test_me
            .consume_line("\\ No newline at end of file", &thread_pool)
            .unwrap();
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedWantMore);
        assert_eq!(result.highlighted.len(), 1);
        assert_eq!(
            result.highlighted[0].get(),
            "\u{1b}[32m+No trailing newline\u{1b}[31m\u{1b}[7m⏎\u{1b}[0m\n\u{1b}[2m\\ No newline at end of file\u{1b}[0m\n",
        );

        assert!(!test_me.more_lines_expected());

        let result = test_me.consume_eof(&thread_pool).unwrap();
        assert!(result.is_empty());
    }

    #[test]
    fn test_decrease_expected_line_count_merge() {
        let mut test_me = HunkLinesHighlighter::from_line("@@@ -1,5 -1,5 +1,5 @@@").unwrap();
        assert_eq!(test_me.expected_line_counts, vec![5, 5, 5]);

        test_me.decrease_expected_line_counts(" -").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![5, 4, 5],
            "Minus counts should have gone down"
        );

        test_me.decrease_expected_line_counts("- ").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![4, 4, 5],
            "Minus counts should have gone down"
        );

        test_me.decrease_expected_line_counts("--").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![3, 3, 5],
            "Minus and space counts should have gone down"
        );

        test_me.decrease_expected_line_counts("++").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![3, 3, 4],
            "With a + line, we should decrease the last line count"
        );

        test_me.decrease_expected_line_counts(" +").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![2, 3, 3],
            "First count should have gone down because of the space in its column. Last count should have gone down because of the + in its column."
        );

        test_me.decrease_expected_line_counts("  ").unwrap();
        assert_eq!(
            test_me.expected_line_counts,
            vec![1, 2, 2],
            "All counts should drop on context (space only) lines"
        );
    }
}
