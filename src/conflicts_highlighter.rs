use std::vec;

use threadpool::ThreadPool;

use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::string_future::StringFuture;
use crate::token_collector::LINE_STYLE_NEW;
use crate::token_collector::LINE_STYLE_OLD;
use crate::{refiner, token_collector};

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

        let c1_header = self.c1_header.clone();
        let c1 = self.c1.clone();
        let c2_header = self.c2_header.clone();
        let c2 = self.c2.clone();
        let footer = self.footer.clone();
        return StringFuture::from_function(
            move || {
                let (c1_tokens, c2_tokens, _, _) = refiner::to_highlighted_tokens(&c1, &c2);
                let highlighted_c1 = token_collector::render(&LINE_STYLE_OLD, "", &c1_tokens);
                let highlighted_c2 = token_collector::render(&LINE_STYLE_NEW, "", &c2_tokens);

                let mut rendered = String::new();
                rendered.push_str(&c1_header);
                rendered.push('\n');
                rendered.push_str(&highlighted_c1);

                rendered.push_str(&c2_header);
                rendered.push('\n');
                rendered.push_str(&highlighted_c2);

                rendered.push_str(&footer);
                rendered.push('\n');

                rendered
            },
            thread_pool,
        );
    }

    /// In this case we'll render:
    /// * In section C1, we highlight additions compared to base
    /// * In section base, we highlight things that have been removed either
    ///   vs C1 or vs C2.
    /// * In section C2, we highlight additions compared to base
    fn render_diff3(&self, thread_pool: &ThreadPool) -> StringFuture {
        assert!(self.base.is_some());
        let c1_header = self.c1_header.clone();
        let c1 = self.c1.clone();
        let base_header = self.base_header.clone();
        let base = self.base.clone().unwrap();
        let c2_header = self.c2_header.clone();
        let c2 = self.c2.clone();
        let footer = self.footer.clone();

        return StringFuture::from_function(
            move || {
                let base_or_newline = if base.is_empty() { "\n" } else { &base };

                let c1_or_newline = if c1.is_empty() { "\n" } else { &c1 };
                let (base_vs_c1_tokens, c1_tokens, _, _) =
                    refiner::to_highlighted_tokens(base_or_newline, c1_or_newline);
                let highlighted_c1 = token_collector::render(&LINE_STYLE_NEW, "", &c1_tokens);

                let c2_or_newline = if c2.is_empty() { "\n" } else { &c2 };
                let (base_vs_c2_tokens, c2_tokens, _, _) =
                    refiner::to_highlighted_tokens(base_or_newline, c2_or_newline);
                let highlighted_c2 = token_collector::render(&LINE_STYLE_NEW, "", &c2_tokens);

                assert_eq!(base_vs_c1_tokens.len(), base_vs_c2_tokens.len());

                // Now, highlight everything in base that was removed either vs c1 or vs c2
                let mut base_tokens = vec![];
                for (base_vs_c1, base_vs_c2) in base_vs_c1_tokens.iter().zip(base_vs_c2_tokens) {
                    if base_vs_c1.style as u8 >= base_vs_c2.style as u8 {
                        base_tokens.push(base_vs_c1.clone());
                    } else {
                        base_tokens.push(base_vs_c2.clone());
                    }
                }

                let highlighted_base = token_collector::render(&LINE_STYLE_OLD, "", &base_tokens);

                let mut rendered = String::new();
                rendered.push_str(&c1_header);
                rendered.push('\n');
                if !c1.is_empty() {
                    rendered.push_str(&highlighted_c1);
                }

                rendered.push_str(&base_header);
                rendered.push('\n');
                if !base.is_empty() {
                    rendered.push_str(&highlighted_base);
                }

                rendered.push_str(&c2_header);
                rendered.push('\n');
                if !c2.is_empty() {
                    rendered.push_str(&highlighted_c2);
                }

                rendered.push_str(&footer);
                rendered.push('\n');

                rendered
            },
            thread_pool,
        );
    }
}
