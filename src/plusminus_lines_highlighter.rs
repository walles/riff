use threadpool::ThreadPool;

use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::refiner;
use crate::string_future::StringFuture;

#[derive(Debug)]
pub(crate) struct PlusMinusLinesHighlighter {
    prefix_length: usize,

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

impl LinesHighlighter for PlusMinusLinesHighlighter {
    /// Expects a non-empty line as input
    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        assert!(!line.is_empty());

        if line.starts_with('\\') {
            return self.consume_nnaeof(thread_pool);
        }

        if line.len() < self.prefix_length {
            // Not enough columns. For example, if we expect two line counts,
            // one of them will have a + prefix and the other one -. So lines
            // need to be at least 1 long in this case.
            return Err(format!(
                "Line too short, expected 0 or at least {} characters",
                self.prefix_length,
            ));
        }

        let (prefix, line) = line.split_at(self.prefix_length);
        if prefix.chars().any(|c| ![' ', '-', '+'].contains(&c)) {
            return Err(format!(
                "Unexpected character in prefix <{prefix}>, only +, - and space allowed: <{}>",
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
                return Ok(Response {
                    line_accepted: LineAcceptance::RejectedDone,
                    highlighted: self.drain(thread_pool),
                });
            }

            self.prefixes.push(prefix.to_string());
            self.texts.push(String::new());

            assert_eq!(prefix, self.current_prefix());
        }

        // Update the current prefix text with the new line
        let text = self.texts.last_mut().unwrap();
        text.push_str(line);
        text.push('\n');

        return Ok(Response {
            // Even if we don't expect any more lines, we could still receive a
            // `\ No newline at end of file` line after this one.
            line_accepted: LineAcceptance::AcceptedWantMore,
            highlighted: vec![],
        });
    }

    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        if self.last_seen_prefix.is_none() {
            return Err("Got EOF without any lines".to_string());
        }

        return Ok(self.drain(thread_pool));
    }
}

impl PlusMinusLinesHighlighter {
    #[must_use]
    pub(crate) fn from_line(line: &str, prefix_length: usize) -> Option<Self> {
        if line.len() < prefix_length {
            return None;
        }

        let (prefix, _) = line.split_at(prefix_length);
        if !prefix.chars().any(|c| ['-', '+'].contains(&c)) {
            // Only whitespace in the prefix, this is not us
            return None;
        }

        return Some(PlusMinusLinesHighlighter {
            prefix_length,
            texts: vec![],
            prefixes: vec![prefix.to_string()],
            last_seen_prefix: None,
        });
    }

    /// Returns `` (the empty string) on no-current-prefix
    fn current_prefix(&self) -> &str {
        if let Some(prefix) = self.prefixes.last() {
            return prefix;
        }
        return "";
    }

    /// Consume a `\ No newline at end of file` line.
    ///
    /// Strip trailing newlines from the relevant texts, as decided by
    /// self.last_seen_prefix.
    fn consume_nnaeof(&mut self, thread_pool: &ThreadPool) -> Result<Response, String> {
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
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedDone,
                highlighted: self.drain(thread_pool),
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

        // We handled some - columns, now there could be more + lines incoming
        return Ok(Response {
            line_accepted: LineAcceptance::AcceptedWantMore,
            highlighted: vec![],
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
}

#[cfg(test)]
mod tests {
    use threadpool::ThreadPool;

    use crate::{
        line_collector::NO_EOF_NEWLINE_MARKER_HOLDER, lines_highlighter::LineAcceptance,
        plusminus_lines_highlighter::PlusMinusLinesHighlighter,
    };

    // `\ No newline at end of file` test, based on
    // `testdata/add-remove-trailing-newline.diff`.
    #[test]
    fn test_nnaeol() {
        {
            let mut no_eof_newline_marker = NO_EOF_NEWLINE_MARKER_HOLDER.lock().unwrap();
            *no_eof_newline_marker = Some("\\ No newline at end of file".to_string());
        }

        let mut test_me = PlusMinusLinesHighlighter::from_line("+No trailing newline", 1).unwrap();
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
}
