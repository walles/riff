use threadpool::ThreadPool;

use crate::hunk_header::HunkHeader;
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::refiner;
use crate::string_future::StringFuture;

#[derive(Debug)]
pub(crate) struct HunkLinesHighlighter {
    // This will have to be rendered at the top of our returned result.
    hunk_header: Option<String>,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_line_counts: Vec<usize>,

    /// Different versions of the same text. The last version in this vector is
    /// the end result. All versions before that are sources that this version
    /// is based on.
    ///
    /// Each version's diff prefix is stored in `prefixes`.
    texts: Vec<String>,

    /// Line prefixes. Usually `-` or `+`, but could be things line ` -` or
    /// `+++` if this is a merge diff. Each prefix corresponds to one prefix
    /// text (stored in `texts`).
    prefixes: Vec<String>,

    last_seen_prefix: Option<String>,
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
                texts: Vec::new(),
                prefixes: Vec::new(),
                last_seen_prefix: None,
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

        // "\ No newline at end of file"
        if line.starts_with('\\') {
            return self.consume_nnaeof(thread_pool, return_me);
        }

        if !self.more_lines_expected() {
            return Ok(Response {
                line_accepted: LineAcceptance::RejectedDone,
                highlighted: return_me,
            });
        }

        // Context lines
        let spaces_only = " ".repeat(self.expected_line_counts.len() - 1);
        if line.is_empty() || line.starts_with(&spaces_only) {
            return_me.append(&mut self.drain(thread_pool));

            self.expected_line_counts
                .iter_mut()
                .for_each(|expected_line_count| {
                    *expected_line_count -= 1;
                });

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

        assert!(!line.is_empty()); // Handled as a plain line above
        return self.consume_plusminus_line(line, thread_pool, return_me);
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
    /// Expects a non-empty line as input
    fn consume_plusminus_line(
        &mut self,
        line: &str,
        thread_pool: &ThreadPool,
        mut return_me: Vec<StringFuture>,
    ) -> Result<Response, String> {
        assert!(!line.is_empty());

        if line.len() < self.expected_line_counts.len() - 1 {
            // Not enough columns. For example, if we expect two line counts,
            // one of them will have a + prefix and the other one -. So lines
            // need to be at least 1 long in this case.
            return Err(format!(
                "Line too short, expected 0 or at least {} characters",
                self.expected_line_counts.len(),
            ));
        }

        let (prefix, line) = line.split_at(self.expected_line_counts.len() - 1);
        if prefix.chars().any(|c| ![' ', '-', '+'].contains(&c)) {
            return Err(format!(
                "Unexpected character in prefix <{}>, only +, - and space allowed",
                prefix
            ));
        }

        self.last_seen_prefix = Some(prefix.to_string());

        // Keep track of which prefix we're currently on or start a new one if
        // needed
        if prefix.is_empty() {
            return Err("Hunk line must start with '-' or '+'".to_string());
        }
        if prefix != self.current_prefix() {
            if self.current_prefix().contains('+') {
                // Always start anew after any `+` section, there are never more
                // than one of those.
                return_me.extend(self.drain(thread_pool));
            }

            self.prefixes.push(prefix.to_string());
            self.texts.push(String::new());

            assert_eq!(prefix, self.current_prefix());
        }

        // Update the current prefix text with the new line
        let text = self.texts.last_mut().unwrap();
        text.push_str(line);
        text.push('\n');

        self.decrease_expected_line_counts(prefix)?;

        return Ok(Response {
            // Even if we don't expect any more lines, we could still receive a
            // `\ No newline at end of file` line after this one.
            line_accepted: LineAcceptance::AcceptedWantMore,
            highlighted: return_me,
        });
    }

    fn decrease_expected_line_counts(&mut self, prefix: &str) -> Result<(), String> {
        if prefix.contains('+') {
            // Any additions always count towards the last (additions) line
            // count
            let expected_line_count = self.expected_line_counts.last_mut().unwrap();
            if *expected_line_count == 0 {
                return Err("Got more + lines than expected".to_string());
            }
            *expected_line_count -= 1;
            return Ok(());
        }

        for (pos, plus_minus_space) in prefix.chars().enumerate() {
            if plus_minus_space == ' ' {
                continue;
            }

            let expected_line_count = &mut self.expected_line_counts[pos];
            if *expected_line_count == 0 {
                return Err(format!(
                    "Got more lines than expected for version (column) {:?}",
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

    /// Returns `` (the empty string) on no-current-prefix
    fn current_prefix(&self) -> &str {
        if let Some(prefix) = self.prefixes.last() {
            return prefix;
        }
        return "";
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
            "No trailing newline", /* <- Note no \n at the end of the line */
        );

        assert!(!test_me.more_lines_expected());
    }
}
