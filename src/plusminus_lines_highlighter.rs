use threadpool::ThreadPool;

use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};

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

        return Ok(Response {
            // Even if we don't expect any more lines, we could still receive a
            // `\ No newline at end of file` line after this one.
            line_accepted: LineAcceptance::AcceptedWantMore,
            highlighted: return_me,
        });
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
}
