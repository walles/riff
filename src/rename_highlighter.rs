use crate::{
    lines_highlighter::{LineAcceptance, LinesHighlighter, Response},
    string_future::StringFuture,
};

// Parses sections looking like this:
//   rename from old_name
//   rename to new_name
pub(crate) struct RenameHighlighter {
    old_name: String,
}

impl LinesHighlighter for RenameHighlighter {
    fn consume_line(
        &mut self,
        line: &str,
        _thread_pool: &threadpool::ThreadPool,
    ) -> Result<crate::lines_highlighter::Response, String> {
        assert!(!self.old_name.is_empty());

        if let Some(rest) = line.strip_prefix("rename to ") {
            let new_name = rest.trim().to_string();
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedDone,
                highlighted: vec![StringFuture::from_string(self.highlighted(new_name))],
            });
        }

        return Err("Expected 'rename to <new_name>' after 'rename from <old_name>'".to_string());
    }

    fn consume_eof(
        &mut self,
        _thread_pool: &threadpool::ThreadPool,
    ) -> Result<Vec<crate::string_future::StringFuture>, String> {
        return Err(
            "Input ended early, rename from should have been followed by rename to".to_string(),
        );
    }
}

impl RenameHighlighter {
    /// Returns None if this line doesn't start a new renames section
    pub(crate) fn from_line(line: &str) -> Option<Self> {
        if let Some(rest) = line.strip_prefix("rename from ") {
            let old_name = rest.trim().to_string();
            Some(Self { old_name })
        } else {
            None
        }
    }

    fn highlighted(&self, new_name: String) -> String {
        // FIXME: Placeholder
        format!(
            "rename from \x1b[1;31m{}\x1b[0m\nrename to \x1b[1;32m{}\x1b[0m",
            self.old_name, new_name
        )
    }
}
