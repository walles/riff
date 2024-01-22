use threadpool::ThreadPool;

use crate::constants::*;
use crate::line_collector::LinesHighlighter;
use crate::refiner::to_highlighted_tokens;
use crate::string_future::StringFuture;
use crate::token_collector::{
    lowlight_timestamp, render, unhighlight_git_prefix, LINE_STYLE_NEW_FILENAME,
    LINE_STYLE_OLD_FILENAME,
};

pub(crate) struct PlusMinusHeaderHighlighter {
    done: bool,
    old_name: String,
    new_name: String,
}

impl LinesHighlighter for PlusMinusHeaderHighlighter {
    fn from_line(line: &str, thread_pool: &ThreadPool) -> Option<Self>
    where
        Self: Sized,
    {
        if !line.starts_with("--- ") && !line.starts_with("+++ ") {
            return None;
        }

        let mut highlighter = PlusMinusHeaderHighlighter {
            old_name: String::new(),
            new_name: String::new(),
            done: false,
        };

        // FIXME: Check for errors and handle them
        highlighter.consume_line(line);

        return Some(highlighter);
    }

    fn consume_line(&mut self, line: &str) -> Result<(), &str> {
        assert!(!self.done);

        if let Some(old_name) = line.strip_prefix("--- ") {
            if !self.new_name.is_empty() {
                self.done = true;
                return Err("Got --- after +++");
            }
            if !self.old_name.is_empty() {
                self.done = true;
                return Err("Got --- twice");
            }

            self.old_name.push_str(old_name);
            return Ok(());
        }

        if let Some(new_name) = line.strip_prefix("+++ ") {
            if self.old_name.is_empty() {
                self.done = true;
                return Err("Got only +++ without ---");
            }
            if !self.new_name.is_empty() {
                self.done = true;
                return Err("Got +++ twice");
            }
            self.new_name.push_str(new_name);
            return Ok(());
        }

        self.done = true;
        return Err("Got neither --- nor +++");
    }

    fn get_highlighted_if_done(&mut self) -> Option<StringFuture> {
        if self.new_name.is_empty() || self.old_name.is_empty() {
            return None;
        }

        if self.old_name == "/dev/null" {
            let mut highlighted = String::new();

            highlighted.push_str(FAINT);
            highlighted.push_str("--- /dev/null");
            highlighted.push_str(NORMAL);
            highlighted.push_str("\n");

            highlighted.push_str(BOLD);
            highlighted.push_str("+++ ");
            highlighted.push_str(&self.new_name);
            highlighted.push_str(NORMAL);
            highlighted.push_str("\n");

            return Some(StringFuture::from_string(highlighted));
        }

        if self.new_name == "/dev/null" {
            let mut highlighted = String::new();

            highlighted.push_str(BOLD);
            highlighted.push_str("--- ");
            highlighted.push_str(&self.old_name);
            highlighted.push_str(NORMAL);
            highlighted.push_str("\n");

            highlighted.push_str(FAINT);
            highlighted.push_str("+++ /dev/null");
            highlighted.push_str(NORMAL);
            highlighted.push_str("\n");

            return Some(StringFuture::from_string(highlighted));
        }

        let (mut old_tokens, mut new_tokens, _, _) =
            to_highlighted_tokens(&self.old_name, &self.new_name);

        lowlight_timestamp(&mut old_tokens);
        unhighlight_git_prefix(&mut old_tokens);
        lowlight_timestamp(&mut new_tokens);
        unhighlight_git_prefix(&mut new_tokens);

        let old_filename = render(&LINE_STYLE_OLD_FILENAME, old_tokens);
        let new_filename = render(&LINE_STYLE_NEW_FILENAME, new_tokens);

        let mut highlighted = String::new();
        highlighted.push_str(&old_filename);
        highlighted.push_str("\n");
        highlighted.push_str(&new_filename);
        highlighted.push_str("\n");

        return Some(StringFuture::from_string(highlighted));
    }
}
