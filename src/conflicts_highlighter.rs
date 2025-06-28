use std::vec;

use threadpool::ThreadPool;

use crate::constants::{GREEN, INVERSE_VIDEO, NORMAL};
use crate::lines_highlighter::{LineAcceptance, LinesHighlighter, Response};
use crate::string_future::StringFuture;
use crate::token_collector::{Style, LINE_STYLE_CONFLICT_OLD};
use crate::token_collector::{LINE_STYLE_CONFLICT_BASE, LINE_STYLE_CONFLICT_NEW};
use crate::{refiner, token_collector};

const CONFLICTS_HEADER1: &str = "<<<<<<<";
const CONFLICTS_HEADER2: &str = "++<<<<<<<";
const BASE_HEADER: &str = "|||||||";
const C2_HEADER: &str = "=======";
const CONFLICTS_FOOTER: &str = ">>>>>>>";

pub(crate) struct ConflictsHighlighter {
    /// `<<<<<<< HEAD`, start of the whole conflict block. Followed by `c1`.
    c1_header: String,

    /// One of the conflicting variants. Multi line string. Always ends with a
    /// newline.
    c1: String,

    /// `||||||| 07ffb9b`, followed by `base` if found. Empty if not found.
    base_header: String,

    /// The base variant which both `c1` and `c2` are based on. Will be
    /// non-empty only for `diff3` style conflict markers. Multi line string.
    /// Always ends with a newline.
    base: String,

    /// Prefixes of the base section, one per line.
    base_line_prefixes: vec::Vec<String>,

    /// `=======`, followed by `c2`
    c2_header: String,

    /// The other conflicting variant. Multi line string. Always ends with a
    /// newline.
    c2: String,

    /// `>>>>>>> branch`, marks the end of `c2` and the whole conflict
    footer: String,
}

impl LinesHighlighter for ConflictsHighlighter {
    fn consume_line(&mut self, line: &str, thread_pool: &ThreadPool) -> Result<Response, String> {
        if self.starts_with(line, BASE_HEADER) {
            if !self.c2.is_empty() {
                return Err(format!(
                    "Unexpected `{BASE_HEADER}` line after `{C2_HEADER}`"
                ));
            }
            if !self.base.is_empty() {
                return Err(format!(
                    "Multiple `{BASE_HEADER}` lines before `{C2_HEADER}`"
                ));
            }

            self.base_header = line.to_string();
            self.base = String::new();
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedWantMore,
                highlighted: vec![],
            });
        }

        if self.starts_with(line, C2_HEADER) {
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

        if self.starts_with(line, CONFLICTS_FOOTER) {
            self.footer = line.to_string();
            return Ok(Response {
                line_accepted: LineAcceptance::AcceptedDone,
                highlighted: vec![self.render(thread_pool)],
            });
        }

        //
        // All header lines handled, this is a content line
        //

        let (prefix_destination, destination) = if !self.c2_header.is_empty() {
            (None, &mut self.c2)
        } else if !self.base_header.is_empty() {
            (Some(&mut self.base_line_prefixes), &mut self.base)
        } else {
            (None, &mut self.c1)
        };

        let prefixes = if self.c1_header.starts_with("++") {
            // Possible content line prefixes when doing "git diff"
            vec!["+ ", "++", " +", " -", "- ", "  "]
        } else {
            vec![""]
        };

        for prefix in prefixes {
            if let Some(line) = line.strip_prefix(prefix) {
                // Handle the context line
                destination.push_str(line);
                destination.push('\n');

                if let Some(prefix_destination) = prefix_destination {
                    prefix_destination.push(prefix.to_string());
                }

                return Ok(Response {
                    line_accepted: LineAcceptance::AcceptedWantMore,
                    highlighted: vec![],
                });
            }
        }

        // Not a context line, just give up and do a simple render
        return Ok(Response {
            line_accepted: LineAcceptance::RejectedDone,
            highlighted: vec![self.render_plain()],
        });
    }

    fn consume_eof(&mut self, _thread_pool: &ThreadPool) -> Result<Vec<StringFuture>, String> {
        // FIXME: Or maybe just `self.render_plain()`?
        return Err("Unexpected EOF inside a conflicts section".to_string());
    }
}

impl ConflictsHighlighter {
    /// Create a new LinesHighlighter from a line of input.
    ///
    /// Returns None if this line doesn't start a new LinesHighlighter.
    pub(crate) fn from_line(line: &str) -> Option<Self>
    where
        Self: Sized,
    {
        if !line.starts_with(CONFLICTS_HEADER1) && !line.starts_with(CONFLICTS_HEADER2) {
            return None;
        }

        return Some(ConflictsHighlighter {
            c1_header: line.to_string(),
            base_header: String::new(),
            c2_header: String::new(),
            footer: String::new(),
            c1: String::new(),
            base: String::new(),
            base_line_prefixes: Vec::new(),
            c2: String::new(),
        });
    }

    // Check if `line` starts with `prefix` or `++prefix` depending on what the
    // `c1_header` looks like.
    fn starts_with(&self, line: &str, prefix: &str) -> bool {
        let prefix = if self.c1_header.starts_with("++") {
            "++".to_string() + prefix
        } else {
            prefix.to_string()
        };

        return line.starts_with(&prefix);
    }

    fn render(&self, thread_pool: &ThreadPool) -> StringFuture {
        if !self.base.is_empty() {
            // We have three sections
            return self.render_diff3(thread_pool);
        }

        let (header_prefix, c1_prefix, c2_prefix, reset) = if self.c1_header.starts_with("++") {
            (INVERSE_VIDEO, " +", "+ ", NORMAL)
        } else {
            (INVERSE_VIDEO, "", "", "")
        };

        let c1_header = self.c1_header.clone();
        let c1 = self.c1.clone();
        let base_header = self.base_header.clone();
        let c2_header = self.c2_header.clone();
        let c2 = self.c2.clone();
        let footer = self.footer.clone();
        return StringFuture::from_function(
            move || {
                let c1_or_newline = if c1.is_empty() { "\n" } else { &c1 };
                let c2_or_newline = if c2.is_empty() { "\n" } else { &c2 };
                let (c1_tokens, c2_tokens) = refiner::diff(c1_or_newline, c2_or_newline);

                let c1_style = if base_header.is_empty() {
                    LINE_STYLE_CONFLICT_OLD.clone()
                } else {
                    LINE_STYLE_CONFLICT_NEW.clone()
                };
                let highlighted_c1 = token_collector::render(&c1_style, c1_prefix, &c1_tokens);
                let highlighted_c2 =
                    token_collector::render(&LINE_STYLE_CONFLICT_NEW, c2_prefix, &c2_tokens);

                let mut rendered = String::new();
                rendered.push_str(header_prefix);
                rendered.push_str(&c1_header);
                rendered.push_str(reset);
                rendered.push('\n');
                if !c1.is_empty() {
                    rendered.push_str(&highlighted_c1);
                }

                if !base_header.is_empty() {
                    rendered.push_str(header_prefix);
                    rendered.push_str(&base_header);
                    rendered.push_str(reset);
                    rendered.push('\n');
                }

                rendered.push_str(header_prefix);
                rendered.push_str(&c2_header);
                rendered.push_str(reset);
                rendered.push('\n');
                if !c2.is_empty() {
                    rendered.push_str(&highlighted_c2);
                }

                rendered.push_str(header_prefix);
                rendered.push_str(&footer);
                rendered.push_str(reset);
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
        let (header_prefix, c1_prefix, c2_prefix, reset) = if self.c1_header.starts_with("++") {
            (INVERSE_VIDEO, " +", "+ ", NORMAL)
        } else {
            (INVERSE_VIDEO, "", "", "")
        };

        assert!(!self.base.is_empty());
        let c1_header = self.c1_header.clone();
        let c1 = self.c1.clone();
        let base_header = self.base_header.clone();
        let base = self.base.clone();
        let base_line_prefixes = self.base_line_prefixes.clone();
        let c2_header = self.c2_header.clone();
        let c2 = self.c2.clone();
        let footer = self.footer.clone();

        return StringFuture::from_function(
            move || {
                let base_or_newline = if base.is_empty() { "\n" } else { &base };

                let c1_or_newline = if c1.is_empty() { "\n" } else { &c1 };
                let (mut base_vs_c1_tokens, c1_tokens) =
                    refiner::diff(base_or_newline, c1_or_newline);
                if c1.is_empty() {
                    // In the base, show only diffs vs c2
                    base_vs_c1_tokens.iter_mut().for_each(|token| {
                        token.style = Style::Context;
                    });
                }
                let highlighted_c1 =
                    token_collector::render(&LINE_STYLE_CONFLICT_NEW, c1_prefix, &c1_tokens);

                let c2_or_newline = if c2.is_empty() { "\n" } else { &c2 };
                let (mut base_vs_c2_tokens, c2_tokens) =
                    refiner::diff(base_or_newline, c2_or_newline);
                if c2.is_empty() {
                    // In the base, show only diffs vs c1
                    base_vs_c2_tokens.iter_mut().for_each(|token| {
                        token.style = Style::Context;
                    });
                }
                let highlighted_c2 =
                    token_collector::render(&LINE_STYLE_CONFLICT_NEW, c2_prefix, &c2_tokens);

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

                let highlighted_base = token_collector::render_multiprefix(
                    &LINE_STYLE_CONFLICT_BASE,
                    &base_line_prefixes,
                    &base_tokens,
                );

                let mut rendered = String::new();
                rendered.push_str(header_prefix);
                rendered.push_str(&c1_header);
                rendered.push_str(reset);
                rendered.push('\n');
                if !c1.is_empty() {
                    rendered.push_str(&highlighted_c1);
                }

                rendered.push_str(header_prefix);
                rendered.push_str(&base_header);
                rendered.push_str(reset);
                rendered.push('\n');
                if !base.is_empty() {
                    rendered.push_str(&highlighted_base);
                }

                rendered.push_str(header_prefix);
                rendered.push_str(&c2_header);
                rendered.push_str(reset);
                rendered.push('\n');
                if !c2.is_empty() {
                    rendered.push_str(&highlighted_c2);
                }

                rendered.push_str(header_prefix);
                rendered.push_str(&footer);
                rendered.push_str(reset);
                rendered.push('\n');

                rendered
            },
            thread_pool,
        );
    }

    // Render everything we have so far without any highlighting, even if it is
    // incomplete
    fn render_plain(&self) -> StringFuture {
        let (color_prefix, reset) = if self.c1_header.starts_with("++") {
            (GREEN, NORMAL)
        } else {
            ("", "")
        };

        let mut rendered = String::new();
        rendered.push_str(color_prefix);
        rendered.push_str(&self.c1_header);
        rendered.push_str(reset);
        rendered.push('\n');

        if !self.c1.is_empty() {
            self.c1.lines().for_each(|line| {
                rendered.push_str(color_prefix);
                rendered.push_str(" +");
                rendered.push_str(line);
                rendered.push_str(reset);
                rendered.push('\n');
            });
        }

        if self.base_header.is_empty() {
            return StringFuture::from_string(rendered);
        }
        rendered.push_str(color_prefix);
        rendered.push_str(&self.base_header);
        rendered.push_str(reset);
        rendered.push('\n');

        if !self.base.is_empty() {
            self.base.lines().for_each(|line| {
                rendered.push_str(color_prefix);
                rendered.push_str("++");
                rendered.push_str(line);
                rendered.push_str(reset);
                rendered.push('\n');
            });
        }

        if self.c2_header.is_empty() {
            return StringFuture::from_string(rendered);
        }

        if !self.c2.is_empty() {
            self.base.lines().for_each(|line| {
                rendered.push_str(color_prefix);
                rendered.push_str("+ ");
                rendered.push_str(line);
                rendered.push_str(reset);
                rendered.push('\n');
            });
        }

        if self.footer.is_empty() {
            return StringFuture::from_string(rendered);
        }
        rendered.push_str(color_prefix);
        rendered.push_str(&self.footer);
        rendered.push_str(reset);
        rendered.push('\n');

        return StringFuture::from_string(rendered);
    }
}
