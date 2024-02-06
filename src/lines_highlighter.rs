use threadpool::ThreadPool;

use crate::string_future::StringFuture;

#[derive(Debug, PartialEq)]
pub(crate) enum LineAcceptance {
    AcceptedWantMore,

    /// Line accepted, but now I'm done
    AcceptedDone,

    /// Line not accepted, and I'm done, ask somebody else
    RejectedDone,
}

pub(crate) struct Response {
    // If true, I accepted this line and you should keep sending me lines. If
    // false, I did not accept this line, and you should handle it some other
    // way.
    pub(crate) line_accepted: LineAcceptance,

    pub(crate) highlighted: Vec<StringFuture>,
}

/// Consume some lines, return some highlighted text
pub(crate) trait LinesHighlighter {
    /// Consume one line of input.
    ///
    /// May or may not return a highlighted string.
    ///
    /// In case this call returns an error, this whole object will be invalid.
    /// afterwards.
    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String>;

    // No more lines available
    fn consume_eof(&mut self, thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String>;
}

impl std::fmt::Debug for dyn LinesHighlighter {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        return write!(f, "LinesHighlighter");
    }
}
