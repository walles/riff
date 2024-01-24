use threadpool::ThreadPool;

use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::string_future::StringFuture;

const CONFLICTS_HEADER: &str = "<<<<<<<";
const BASE_HEADER: &str = "|||||||";
const C2_HEADER: &str = "=======";
const CONFLICTS_FOOTER: &str = ">>>>>>>";

pub(crate) struct ConflictsHighlighter {
    /// `<<<<<<< HEAD`, start of the whole conflict block. Followed by `c1`.
    c1_header: String,

    /// `||||||| 07ffb9b`, followed by `base` if found
    base_header: String,

    /// `=======`, followed by `c2`
    c2_header: String,

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
        if !line.starts_with(CONFLICTS_HEADER) {
            return None;
        }

        return Some(ConflictsHighlighter {
            c1_header: line.to_string(),
            base_header: String::new(),
            c2_header: String::new(),
            footer: String::new(),
            c1: String::new(),
            base: None,
            c2: String::new(),
        });
    }

    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        if line.starts_with(BASE_HEADER) {
            if !self.c2.is_empty() {
                return Err(format!(
                    "Unexpected `{BASE_HEADER}` line after `{C2_HEADER}`"
                ));
            }
            if self.base.is_some() {
                return Err(format!(
                    "Multiple `{BASE_HEADER}` lines before `{C2_HEADER}`"
                ));
            }

            self.base_header = line.to_string();
            self.base = Some(String::new());
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: vec![],
            });
        }

        if line.starts_with(C2_HEADER) {
            if !self.c2.is_empty() {
                return Err(format!(
                    "Multiple `{C2_HEADER}` lines before `{CONFLICTS_FOOTER}`"
                ));
            }

            self.c2_header = line.to_string();
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: vec![],
            });
        }

        if line.starts_with(CONFLICTS_FOOTER) {
            self.footer = line.to_string();
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedDone,
                highlighted: vec![self.render(thread_pool)],
            });
        }

        if !self.c2_header.is_empty() {
            // We're in the last section
            self.c2.push_str(line);
            self.c2.push('\n');
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: vec![],
            });
        } else if !self.base_header.is_empty() {
            // We're in the base section
            self.base.as_mut().unwrap().push_str(line);
            self.base.as_mut().unwrap().push('\n');
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: vec![],
            });
        } else {
            // We're in the first section
            self.c1.push_str(line);
            self.c1.push('\n');
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: vec![],
            });
        }
    }

    fn consume_eof(&mut self, _thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        return Err("Unexpected EOF inside a conflicts section".to_string());
    }
}

impl ConflictsHighlighter {
    fn render(&self, thread_pool: &ThreadPool) -> StringFuture {
        if self.base.is_some() {
            return self.render_diff3(thread_pool);
        }

        // FIXME: Highlight self.c1 and self.c2 before rendering

        let mut rendered = String::new();
        rendered.push_str(&self.c1_header);
        rendered.push('\n');
        rendered.push_str(&self.c1);

        rendered.push_str(&self.c2_header);
        rendered.push('\n');
        rendered.push_str(&self.c2);

        rendered.push_str(&self.footer);
        rendered.push('\n');

        return StringFuture::from_string(rendered);
    }

    fn render_diff3(&self, _thread_pool: &ThreadPool) -> StringFuture {
        // FIXME: Highlight self.c1, self.base and self.c2 before rendering

        let mut rendered = String::new();
        rendered.push_str(&self.c1_header);
        rendered.push('\n');
        rendered.push_str(&self.c1);

        rendered.push_str(&self.base_header);
        rendered.push('\n');
        rendered.push_str(self.base.as_ref().unwrap());

        rendered.push_str(&self.c2_header);
        rendered.push('\n');
        rendered.push_str(&self.c2);

        rendered.push_str(&self.footer);
        rendered.push('\n');

        return StringFuture::from_string(rendered);
    }
}
