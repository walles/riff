use threadpool::ThreadPool;

use crate::constants::*;
use crate::lines_highlighter::LineAcceptance;
use crate::lines_highlighter::{LinesHighlighter, Response};
use crate::refiner::to_highlighted_tokens;
use crate::string_future::StringFuture;
use crate::token_collector::{
    align_tabs, lowlight_timestamp, render, unhighlight_git_prefix, LINE_STYLE_NEW_FILENAME,
    LINE_STYLE_OLD_FILENAME,
};

pub(crate) struct PlusMinusHeaderHighlighter {
    old_name: String,
    new_name: String,
}

impl LinesHighlighter for PlusMinusHeaderHighlighter {
    fn consume_line(&mut self, line: &str, _thread_pool: &ThreadPool) -> Result<Response, String> {
        assert!(!self.old_name.is_empty());
        assert!(self.new_name.is_empty());

        if let Some(new_name) = line.strip_prefix("+++ ") {
            self.new_name.push_str(new_name);
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedDone,
                highlighted: vec![StringFuture::from_string(self.highlighted())],
            });
        }

        return Err("--- was not followed by +++".to_string());
    }

    fn consume_eof(&mut self, _thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        return Err("Input ended early, --- should have been followed by +++".to_string());
    }
}

impl PlusMinusHeaderHighlighter {
    /// Create a new LinesHighlighter from a line of input.
    ///
    /// Returns None if this line doesn't start a new LinesHighlighter.
    pub(crate) fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if !line.starts_with("--- ") {
            return None;
        }

        let highlighter = PlusMinusHeaderHighlighter {
            old_name: line.strip_prefix("--- ").unwrap().to_string(),
            new_name: String::new(),
        };

        return Some(highlighter);
    }

    fn highlighted(&self) -> String {
        if self.old_name == "/dev/null" {
            // New file added
            let mut highlighted = String::new();

            highlighted.push_str(BOLD);
            highlighted.push_str("--- ");
            highlighted.push_str(NORMAL_INTENSITY);
            highlighted.push_str(FAINT);
            highlighted.push_str("/dev/null");
            highlighted.push_str(NORMAL);
            highlighted.push('\n');

            highlighted.push_str(BOLD);
            highlighted.push_str("+++ NEW ");
            if let Some(no_prefix) = self.new_name.strip_prefix("b/") {
                highlighted.push_str(NORMAL_INTENSITY);
                highlighted.push_str(FAINT);
                highlighted.push_str("b/");
                highlighted.push_str(NORMAL_INTENSITY);
                highlighted.push_str(BOLD);
                highlighted.push_str(no_prefix);
            } else {
                highlighted.push_str(&self.new_name);
            }
            highlighted.push_str(NORMAL);
            highlighted.push('\n');

            return highlighted;
        }

        if self.new_name == "/dev/null" {
            // File deleted
            let mut highlighted = String::new();

            highlighted.push_str(BOLD);
            highlighted.push_str("--- DELETED ");
            if let Some(no_prefix) = self.old_name.strip_prefix("a/") {
                highlighted.push_str(NORMAL_INTENSITY);
                highlighted.push_str(FAINT);
                highlighted.push_str("a/");
                highlighted.push_str(NORMAL_INTENSITY);
                highlighted.push_str(BOLD);
                highlighted.push_str(no_prefix);
            } else {
                highlighted.push_str(&self.old_name);
            }
            highlighted.push_str(NORMAL);
            highlighted.push('\n');

            highlighted.push_str(BOLD);
            highlighted.push_str("+++ ");
            highlighted.push_str(NORMAL_INTENSITY);
            highlighted.push_str(FAINT);
            highlighted.push_str("/dev/null");
            highlighted.push_str(NORMAL);
            highlighted.push('\n');

            return highlighted;
        }

        let (mut old_tokens, mut new_tokens, _, _) =
            to_highlighted_tokens(&self.old_name, &self.new_name, false);

        lowlight_timestamp(&mut old_tokens);
        lowlight_timestamp(&mut new_tokens);

        align_tabs(&mut old_tokens, &mut new_tokens);

        unhighlight_git_prefix(&mut old_tokens);
        unhighlight_git_prefix(&mut new_tokens);

        let old_filename = render(&LINE_STYLE_OLD_FILENAME, "--- ", &old_tokens);
        let new_filename = render(&LINE_STYLE_NEW_FILENAME, "+++ ", &new_tokens);

        let mut highlighted = String::new();
        highlighted.push_str(&old_filename);
        highlighted.push('\n');
        highlighted.push_str(&new_filename);
        highlighted.push('\n');

        return highlighted;
    }
}

#[cfg(test)]
mod tests {
    use crate::ansi::without_ansi_escape_codes;

    use super::*;

    #[test]
    fn test_align_timestamps() {
        let mut test_me =
            PlusMinusHeaderHighlighter::from_line("--- x.txt\t2023-12-15 15:43:29").unwrap();
        let mut response = test_me
            .consume_line(
                "+++ /Users/johan/src/riff/README.md\t2024-01-29 14:56:40",
                &ThreadPool::new(1),
            )
            .unwrap();
        assert_eq!(LineAcceptance::AcceptedDone, response.line_accepted);
        assert_eq!(1, response.highlighted.len());

        let highlighted = response.highlighted[0].get().to_string().into_bytes();
        let plain = String::from_utf8(without_ansi_escape_codes(&highlighted)).unwrap();

        assert_eq!(
            "--- x.txt                            2023-12-15 15:43:29\n\
            +++ /Users/johan/src/riff/README.md  2024-01-29 14:56:40\n",
            plain.as_str()
        );
    }
}
