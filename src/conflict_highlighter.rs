use threadpool::ThreadPool;

use crate::line_collector::LinesHighlighter;
use crate::string_future::StringFuture;

struct ConflictHighlighter<'a> {
    thread_pool: &'a ThreadPool,
}

impl<'a> LinesHighlighter for ConflictHighlighter<'a> {
    fn from_line(line: &str, thread_pool: &ThreadPool) -> Option<Self>
    where
        Self: Sized,
    {
        todo!()
    }

    fn consume_line(&mut self, line: &str) -> Result<(), &str> {
        todo!()
    }

    fn get_highlighted_if_done(&mut self) -> Option<StringFuture> {
        todo!()
    }
}
