use threadpool::ThreadPool;

use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
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
}
