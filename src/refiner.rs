use crate::constants::*;
use crate::tokenizer;
use diffus::{
    edit::{self, collection},
    Diffable,
};

/// If more than this part of either adds or moves is highlighted,
/// we consider it to be a replacement rather than a move, and skip
/// highlighting it.
const MAX_HIGHLIGHT_PERCENTAGE: usize = 30;

/// If it's only this few highlights, we'll just highligh anyway without
/// checking the `MAX_HIGHLIGHT_PERCENTAGE`.
const OK_HIGHLIGHT_COUNT: usize = 5;

pub struct Refiner<'a> {
    adds: &'a str,
    removes: &'a str,
}

impl<'a> Refiner<'a> {
    pub fn create(adds: &'a str, removes: &'a str) -> Self {
        return Refiner { adds, removes };
    }

    /// Format add and remove lines in ADD and REMOVE colors.
    ///
    /// No intra-line refinement.
    #[must_use]
    fn simple_format(&self) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        for remove_line in self.removes.lines() {
            lines.push(format!("{}-{}{}", REMOVE, remove_line, NORMAL));
        }
        if (!self.removes.is_empty()) && !self.removes.ends_with('\n') {
            lines.push(format!(
                "{}{}{}",
                NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
            ));
        }

        for add_line in self.adds.lines() {
            lines.push(format!("{}+{}{}", ADD, add_line, NORMAL))
        }
        if (!self.adds.is_empty()) && !self.adds.ends_with('\n') {
            lines.push(format!(
                "{}{}{}",
                NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
            ));
        }

        return lines;
    }

    /// Returns a vector of ANSI highlighted lines
    #[must_use]
    pub fn format(self) -> Vec<String> {
        if self.adds.is_empty() {
            return self.simple_format();
        }

        if self.removes.is_empty() {
            return self.simple_format();
        }

        // Find diffs between adds and removals
        let mut highlighted_adds = String::new();
        let mut highlighted_removes = String::new();
        let mut adds_is_inverse = false;
        let mut removes_is_inverse = false;
        let mut adds_highlight_count = 0;
        let mut removes_highlight_count = 0;

        // Tokenize adds and removes before diffing them
        let tokenized_adds = tokenizer::tokenize(self.adds);
        let tokenized_removes = tokenizer::tokenize(self.removes);

        let diff = tokenized_removes.diff(&tokenized_adds);
        match diff {
            edit::Edit::Copy(unchanged) => {
                for token in unchanged {
                    highlighted_adds.push_str(token);
                    highlighted_removes.push_str(token);
                }
            }
            edit::Edit::Change(diff) => {
                diff.into_iter()
                    .map(|edit| {
                        match edit {
                            collection::Edit::Copy(elem) => {
                                if adds_is_inverse {
                                    highlighted_adds.push_str(NOT_INVERSE_VIDEO);
                                }
                                adds_is_inverse = false;

                                if removes_is_inverse {
                                    highlighted_removes.push_str(NOT_INVERSE_VIDEO);
                                }
                                removes_is_inverse = false;

                                highlighted_adds.push_str(elem);
                                highlighted_removes.push_str(elem);
                            }
                            collection::Edit::Insert(elem) => {
                                adds_highlight_count += 1;
                                if !adds_is_inverse {
                                    highlighted_adds.push_str(INVERSE_VIDEO);
                                }
                                adds_is_inverse = true;

                                if elem == "\n" {
                                    // Make sure the highlighted linefeed is visible
                                    highlighted_adds.push('⏎');

                                    // This will be reset by the linefeed, so we need to re-inverse on the next line
                                    adds_is_inverse = false;
                                }
                                highlighted_adds.push_str(elem);
                            }
                            collection::Edit::Remove(elem) => {
                                removes_highlight_count += 1;
                                if !removes_is_inverse {
                                    highlighted_removes.push_str(INVERSE_VIDEO);
                                }
                                removes_is_inverse = true;

                                if elem == "\n" {
                                    // Make sure the highlighted linefeed is visible
                                    highlighted_removes.push('⏎');

                                    // This will be reset by the linefeed, so we need to re-inverse on the next line
                                    removes_is_inverse = false;
                                }
                                highlighted_removes.push_str(elem);
                            }
                            collection::Edit::Change(_) => panic!("Not implemented, help!"),
                        };
                    })
                    .for_each(drop);
            }
        }

        let highlight_count = adds_highlight_count + removes_highlight_count;
        let token_count = tokenized_adds.len() + tokenized_removes.len();

        // FIXME: Maybe for this check count how many runs of characters were
        // highlighted rather than how many tokens? Heuristics are difficult...
        if highlight_count <= OK_HIGHLIGHT_COUNT {
            // Few enough highlights, Just do it (tm)
        } else if (100 * highlight_count) / token_count > MAX_HIGHLIGHT_PERCENTAGE {
            return self.simple_format();
        }

        let mut lines: Vec<String> = Vec::new();
        for highlighted_remove in highlighted_removes.lines() {
            lines.push(format!("{}-{}{}", REMOVE, highlighted_remove, NORMAL));
        }
        if (!self.removes.is_empty()) && !self.removes.ends_with('\n') {
            lines.push(format!(
                "{}{}{}",
                NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
            ));
        }

        for highlighted_add in highlighted_adds.lines() {
            lines.push(format!("{}+{}{}", ADD, highlighted_add, NORMAL));
        }
        if (!self.adds.is_empty()) && !self.adds.ends_with('\n') {
            lines.push(format!(
                "{}{}{}",
                NO_EOF_NEWLINE_COLOR, NO_EOF_NEWLINE_MARKER, NORMAL
            ));
        }

        return lines;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_simple_format_adds_and_removes() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(
            Refiner::create(&"".to_string(), &"".to_string()).simple_format(),
            empty
        );

        // Test adds-only
        assert_eq!(
            Refiner::create(&"a\n".to_string(), &"".to_string()).simple_format(),
            ["".to_string() + ADD + "+a" + NORMAL]
        );
        assert_eq!(
            Refiner::create(&"a\nb\n".to_string(), &"".to_string()).simple_format(),
            [
                "".to_string() + ADD + "+a" + NORMAL,
                "".to_string() + ADD + "+b" + NORMAL,
            ]
        );

        // Test removes-only
        assert_eq!(
            Refiner::create(&"".to_string(), &"a\n".to_string()).simple_format(),
            ["".to_string() + REMOVE + "-a" + NORMAL]
        );
        assert_eq!(
            Refiner::create(&"".to_string(), &"a\nb\n".to_string()).simple_format(),
            [
                "".to_string() + REMOVE + "-a" + NORMAL,
                "".to_string() + REMOVE + "-b" + NORMAL,
            ]
        );
    }

    #[test]
    fn test_quote_change() {
        assert_eq!(
            Refiner::create(&"[quotes]\n".to_string(), &"<quotes>\n".to_string()).format(),
            [
                format!(
                    "{}-{}<{}quotes{}>{}{}",
                    REMOVE,
                    INVERSE_VIDEO,
                    NOT_INVERSE_VIDEO,
                    INVERSE_VIDEO,
                    NOT_INVERSE_VIDEO,
                    NORMAL
                ),
                format!(
                    "{}+{}[{}quotes{}]{}{}",
                    ADD, INVERSE_VIDEO, NOT_INVERSE_VIDEO, INVERSE_VIDEO, NOT_INVERSE_VIDEO, NORMAL
                ),
            ]
        )
    }
}
