use std::iter;

use threadpool::ThreadPool;

use crate::hunk_header::HunkHeader;
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::refiner;
use crate::string_future::StringFuture;

pub(crate) struct HunkLinesHighlighter {
    // This will have to be rendered at the top of our returned result.
    hunk_header: Option<String>,

    /// Calculated by HunkHeader::parse(). We'll count this value down as we consume lines.
    expected_line_counts: Vec<usize>,

    /// Different versions of the same text. The last version in this vector is
    /// the end result. All versions before that are sources that this version
    /// is based on.
    prefix_texts: Vec<String>,

    /// Line prefixes. Usually `-` or `+`, but could be things line ` -` or
    /// `+++` if this is a merge diff. Each prefix corresponds to one prefix
    /// text (stored in `prefix_texts`).
    prefixes: Vec<String>,
}

impl LinesHighlighter for HunkLinesHighlighter {
    fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if let Some(hunk_header) = HunkHeader::parse(line) {
            let expected_line_counts = hunk_header.linecounts;
            let mut prefix_texts = Vec::new();
            let mut prefixes = Vec::new();

            return Some(HunkLinesHighlighter {
                hunk_header: Some(hunk_header.render()),
                expected_line_counts,
                prefix_texts,
                prefixes,
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
            // "\ No newline at end of file"
            todo!("Remove trailing newlines from whatever section(s) we are in");
        }

        if !self.more_lines_expected() {
            return Ok(Response {
                line_accepted: LineAcceptance::RejectedDone,
                highlighted: return_me,
            });
        }

        // Context lines
        let spaces_only = iter::repeat(' ')
            .take(self.expected_line_counts.len())
            .collect::<String>();
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

        if line.len() < self.expected_line_counts.len() {
            // Not enough columns
            return Err(format!(
                "Line too short, expected 0 or at least {} characters",
                self.expected_line_counts.len(),
            ));
        }
        let (prefix, line) = line.split_at(self.expected_line_counts.len());

        if prefix.chars().any(|c| ![' ', '-', '+'].contains(&c)) {
            return Err(format!(
                "Unexpected prefix character, only +, - and space allowed"
            ));
        }

        // Keep track of which prefix we're currently on or start a new one if
        // needed
        if prefix.is_empty() {
            return Err("Hunk line must start with '-' or '+'".to_string());
        }
        if prefix != self.current_prefix() {
            if self.current_prefix().contains('+') {
                // Always start anew after any `+` section, there are never more
                // than one of those.
                self.drain(thread_pool);
            }

            self.prefixes.push(prefix.to_string());
            self.prefix_texts.push(String::new());

            assert_eq!(prefix, self.current_prefix());
        }

        // Update the current prefix text with the new line
        let mut current_prefix_text = self.prefix_texts.last_mut().unwrap();
        current_prefix_text.push_str(&line.to_string());
        current_prefix_text.push('\n');

        // Decrease the expected line counts for all non-` ` prefix columns
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
        if self.prefix_texts.iter().all(|flavor| flavor.is_empty()) {
            return vec![];
        }

        let prefix_texts = self.prefix_texts.clone();
        let prefixes = self.prefixes.clone();

        self.prefix_texts.clear();
        self.prefixes.clear();

        let return_me = StringFuture::from_function(
            move || {
                let mut result = String::new();
                for line in refiner::format(&prefixes, &prefix_texts) {
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
