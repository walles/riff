use threadpool::ThreadPool;

use crate::lines_highlighter::{LinesHighlighter, Response};
use crate::string_future::StringFuture;

pub(crate) struct ConflictsHighlighter {}

impl LinesHighlighter for ConflictsHighlighter {
    fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        todo!()
    }

    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        todo!()
    }

    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        todo!()
    }
}
