use threadpool::ThreadPool;

use crate::lines_highlighter::LineAcceptance;
use crate::lines_highlighter::{LinesHighlighter, Response};
use crate::refiner::diff;
use crate::string_future::StringFuture;
use crate::token_collector::{
    align_tabs, brighten_filename, lowlight_dev_null, lowlight_git_prefix, lowlight_timestamp,
    render, Style, StyledToken, LINE_STYLE_NEW_FILENAME, LINE_STYLE_OLD_FILENAME,
};

pub(crate) struct PlusMinusHeaderHighlighter {
    /// May or may not end with one or more tabs + a timestamp string.
    old_name: String,

    /// May or may not end with one or more tabs + a timestamp string.
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
        let (mut old_tokens, mut new_tokens) = diff(&self.old_name, &self.new_name);

        // New file
        if self.old_name == "/dev/null" {
            // Don't diff highlight vs "/dev/null"
            for token in &mut new_tokens {
                token.style = Style::Context;
            }
        }
        let new_prefix = if self.old_name == "/dev/null" {
            Some(StyledToken::new("NEW ".to_string(), Style::Bright))
        } else {
            None
        };

        // Deleted file
        if self.new_name == "/dev/null" {
            // Don't diff highlight vs "/dev/null"
            for token in &mut old_tokens {
                token.style = Style::Context;
            }
        }
        let old_prefix = if self.new_name == "/dev/null" {
            Some(StyledToken::new("DELETED ".to_string(), Style::Bright))
        } else {
            None
        };

        brighten_filename(&mut old_tokens);
        brighten_filename(&mut new_tokens);

        lowlight_dev_null(&mut old_tokens);
        lowlight_dev_null(&mut new_tokens);

        lowlight_timestamp(&mut old_tokens);
        lowlight_timestamp(&mut new_tokens);

        align_tabs(&mut old_tokens, &mut new_tokens);

        lowlight_git_prefix(&mut old_tokens);
        lowlight_git_prefix(&mut new_tokens);

        if let Some(prefix) = new_prefix {
            new_tokens.insert(0, prefix);
        }

        if let Some(prefix) = old_prefix {
            old_tokens.insert(0, prefix);
        }

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
    use crate::constants::*;

    use super::*;

    const NOT_INVERSE_VIDEO: &str = "\x1b[27m";
    const DEFAULT_COLOR: &str = "\x1b[39m";

    fn highlight_header_lines(old_line: &str, new_line: &str) -> String {
        let mut test_me = PlusMinusHeaderHighlighter::from_line(old_line).unwrap();
        let mut response = test_me.consume_line(new_line, &ThreadPool::new(1)).unwrap();
        assert_eq!(LineAcceptance::AcceptedDone, response.line_accepted);
        assert_eq!(1, response.highlighted.len());

        let highlighted = response.highlighted[0].get().to_string();
        return highlighted;
    }

    #[test]
    fn test_align_timestamps() {
        let highlighted = highlight_header_lines(
            "--- x.txt\t2023-12-15 15:43:29",
            "+++ /Users/johan/src/riff/README.md\t2024-01-29 14:56:40",
        );
        let highlighted_bytes = highlighted.clone().into_bytes();
        let plain = String::from_utf8(without_ansi_escape_codes(&highlighted_bytes)).unwrap();

        assert_eq!(
            "--- x.txt                            2023-12-15 15:43:29\n\
            +++ /Users/johan/src/riff/README.md  2024-01-29 14:56:40\n",
            plain.as_str()
        );
    }

    #[test]
    fn test_brighten_filename() {
        let highlighted = highlight_header_lines("--- a/x/y/z.txt", "+++ b/x/y/z.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {NORMAL_INTENSITY}{FAINT}a/{NORMAL}x/y/{BOLD}z.txt{NORMAL}\n\
                {BOLD}+++ {NORMAL_INTENSITY}{FAINT}b/{NORMAL}x/y/{BOLD}z.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_brighten_filename_without_path() {
        let highlighted = highlight_header_lines("--- z.txt", "+++ z.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- z.txt{NORMAL}\n\
                {BOLD}+++ z.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_brighten_file_rename() {
        let highlighted = highlight_header_lines("--- x.txt", "+++ y.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {INVERSE_VIDEO}{NORMAL_INTENSITY}{OLD}x{NOT_INVERSE_VIDEO}{BOLD}{DEFAULT_COLOR}.txt{NORMAL}\n\
                {BOLD}+++ {INVERSE_VIDEO}{NORMAL_INTENSITY}{GREEN}y{NOT_INVERSE_VIDEO}{BOLD}{DEFAULT_COLOR}.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_new_file_header() {
        let highlighted = highlight_header_lines("--- /dev/null", "+++ b/newfile.txt");
        assert_eq!(
            format!(
                "\
                {BOLD}--- {NORMAL_INTENSITY}{FAINT}/dev/null{NORMAL}\n\
                {BOLD}+++ NEW {NORMAL_INTENSITY}{FAINT}b/{NORMAL_INTENSITY}{BOLD}newfile.txt{NORMAL}\n"
            ),
            highlighted
        );
    }

    #[test]
    fn test_deleted_file_header() {
        let highlighted = highlight_header_lines("--- a/oldfile.txt", "+++ /dev/null");
        assert_eq!(
            format!(
                "\
                {BOLD}--- DELETED {NORMAL_INTENSITY}{FAINT}a/{NORMAL_INTENSITY}{BOLD}oldfile.txt{NORMAL}\n\
                {BOLD}+++ {NORMAL_INTENSITY}{FAINT}/dev/null{NORMAL}\n"
            ),
            highlighted
        );
    }
}
