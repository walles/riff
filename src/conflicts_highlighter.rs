use threadpool::ThreadPool;

use crate::lines_highlighter::{LinesHighlighter, Response};
use crate::string_future::StringFuture;

pub(crate) struct ConflictsHighlighter {
    /// `<<<<<<< HEAD`, start of the whole conflict block. Followed by `c1`.
    c1Header: String,

    /// `||||||| 07ffb9b`, followed by `base` if found
    baseHeader: String,

    /// `=======`, followed by `c2`
    c2Header: String,

    /// `>>>>>>> branch`, marks the end of `c2` and the whole conflict
    footer: String,

    /// One of the conflicting variants. Always ends with a newline.
    c1: String,

    /// The base variant which both `c1` and `c2` are based on. Will be set only
    /// for `diff3` style conflict markers.
    base: Option<String>,

    /// The other conflicting variant. Always ends with a newline.
    c2: String,
}

impl LinesHighlighter for ConflictsHighlighter {
    fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if !line.starts_with("<<<<<<<") {
            return None;
        }

        return Some(ConflictsHighlighter {
            c1Header: line.to_string(),
            baseHeader: String::new(),
            c2Header: String::new(),
            footer: String::new(),
            c1: String::new(),
            base: None,
            c2: String::new(),
        });
    }

    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        todo!()
    }

    fn consume_eof(&mut self, _thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        return Err("Unexpected EOF inside a conflicts section".to_string());
    }
}
