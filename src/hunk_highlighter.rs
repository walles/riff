use threadpool::ThreadPool;

use crate::hunk_header::HunkHeader;
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::plusminus_lines_highlighter::PlusMinusLinesHighlighter;
use crate::refiner;
use crate::string_future::StringFuture;

#[derive(Debug)]
pub(crate) struct HunkLinesHighlighter {
    lines_highlighter: Option<PlusMinusLinesHighlighter>,

    // This will have to be rendered at the top of our returned result.
    hunk_header: Option<String>,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_line_counts: Vec<usize>,
}

impl LinesHighlighter for HunkLinesHighlighter {
    fn from_line(line: &str) -> Option<Self>
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

    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        let mut return_me = vec![];

        // Always start by rendering the hunk header
        if let Some(hunk_header) = &self.hunk_header {
            return_me.push(StringFuture::from_string(hunk_header.to_string() + "\n"));
            self.hunk_header = None;
        }

        if let Some(lines_highlighter) = &mut self.lines_highlighter {
            let mut result = lines_highlighter.consume_line(line, thread_pool)?;
            return_me.append(&mut result.highlighted);
            if result.line_accepted != LineAcceptance::AcceptedWantMore {
                self.lines_highlighter = None;
            }
        } else if line.starts_with('\\') {
            return self.consume_nnaeof(thread_pool, return_me);
        }

        if !self.more_lines_expected() {
            return_me.append(&mut self.drain(thread_pool));
            return Ok(Response {
                line_accepted: LineAcceptance::RejectedDone,
                highlighted: return_me,
            });
        }

        let prefix_length = self.expected_line_counts.len() - 1;
        let spaces_only = " ".repeat(prefix_length);
        let prefix = if line.len() >= prefix_length {
            line.split_at(prefix_length).0
        } else {
            spaces_only.as_str()
        };
        self.decrease_expected_line_counts(prefix)?;

        // It wasn't a plusminus line, it wasn't a nnaeof line, and we're still
        // expecting more lines. It must be a context line.

        // Context lines
        if line.is_empty() || line.starts_with(&spaces_only) {
            return_me.append(&mut self.drain(thread_pool));

            // FIXME: Consider whether we should be coalescing the plain lines?
            // Maybe that would improve performance? Measure and find out!
            return_me.push(StringFuture::from_string(line.to_string() + "\n"));

            let acceptance = if self.more_lines_expected() {
                LineAcceptance::AcceptedWantMore
            } else {
                LineAcceptance::AcceptedDone
            };

            return Ok(Response {
                line_accepted: acceptance,
                highlighted: return_me,
            });
        }

        return Err("Unhandled line".to_string());
    }

    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        if self.more_lines_expected() {
            return Err(format!(
                "Still expecting more lines, but got EOF: {:?}",
                self.expected_line_counts
            ));
        }

        return Ok(self.drain(thread_pool));
    }
}

impl HunkLinesHighlighter {
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

    /// Consume a `\ No newline at end of file` line.
    ///
    /// Strip trailing newlines from the relevant texts, as decided by
    /// self.last_seen_prefix.
    fn consume_nnaeof(
        &mut self,
        thread_pool: &ThreadPool,
        mut return_me: Vec<StringFuture>,
    ) -> Result<Response, String> {
        if self.last_seen_prefix.is_none() {
            return Err(
                "Got '\\ No newline at end of file' without being in a +/- section".to_string(),
            );
        }

        let prefix = self.last_seen_prefix.as_ref().unwrap();

        // Additions are always about the last text
        if prefix.contains('+') {
            let text = self.texts.last_mut().unwrap();

            if let Some(without_newline) = text.strip_suffix('\n') {
                *text = without_newline.to_string();
            } else {
                return Err(
                    "Got + '\\ No newline at end of file' without any newline to remove"
                        .to_string(),
                );
            }

            // `\ No newline at end of file` is always the last line of +
            // section, and the + sections always come last, so we're done.
            return_me.extend(self.drain(thread_pool));
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedDone,
                highlighted: return_me,
            });
        }

        // Remove trailing newlines from all texts with a `-` in their column
        for (pos, plus_minus_space) in prefix.chars().enumerate() {
            if plus_minus_space == ' ' {
                continue;
            }

            let text = self.texts[pos].strip_suffix('\n');
            if let Some(text) = text {
                self.texts[pos] = text.to_string();
            } else {
                return Err(
                    "Got - '\\ No newline at end of file' without any newline to remove"
                        .to_string(),
                );
            }
        }

        let acceptance = if self.more_lines_expected() {
            // We're still waiting for + lines, or for lines of other - sections
            LineAcceptance::AcceptedWantMore
        } else {
            LineAcceptance::AcceptedDone
        };
        return Ok(Response {
            line_accepted: acceptance,
            highlighted: return_me,
        });
    }

    #[must_use]
    fn drain(&mut self, thread_pool: &ThreadPool) -> Vec<StringFuture> {
        // Return nothing if all flavors are empty
        if self.texts.iter().all(|flavor| flavor.is_empty()) {
            return vec![];
        }

        let texts = self.texts.clone();
        let prefixes = self.prefixes.clone();

        self.texts.clear();
        self.prefixes.clear();

        let return_me = StringFuture::from_function(
            move || {
                let mut result = String::new();
                for line in refiner::format(
                    &prefixes.iter().map(String::as_ref).collect(),
                    &texts.iter().map(String::as_ref).collect(),
                ) {
                    result.push_str(&line);
                    result.push('\n');
                }

                result
            },
            thread_pool,
        );

        return vec![return_me];
    }

    fn more_lines_expected(&self) -> bool {
        for expected_line_count in &self.expected_line_counts {
            if *expected_line_count != 0 {
                return true;
            }
        }
        return false;
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
        assert_eq!(test_me.prefixes, vec!["+"]);
        assert_eq!(test_me.texts, vec!["No trailing newline\n"]);

        let mut result = test_me
            .consume_line("\\ No newline at end of file", &thread_pool)
            .unwrap();
        assert_eq!(result.line_accepted, LineAcceptance::AcceptedDone);
        assert_eq!(result.highlighted.len(), 1);
        assert_eq!(
            result.highlighted[0].get(),
            "\u{1b}[32m+No trailing newline\u{1b}[31m\u{1b}[7m⏎\u{1b}[0m\n\u{1b}[2m\\ No newline at end of file\u{1b}[0m\n",
        );

        assert!(!test_me.more_lines_expected());
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
