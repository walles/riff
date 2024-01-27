use crate::constants::*;
use crate::line_collector::NO_EOF_NEWLINE_MARKER_HOLDER;
use crate::token_collector::*;
use crate::tokenizer;
use diffus::{
    edit::{self, collection},
    Diffable,
};

/// Like format!(), but faster for our special case
fn format_simple_line(old_new: &str, plus_minus: &str, contents: &str) -> String {
    let mut line = String::with_capacity(old_new.len() + 1 + contents.len() + NORMAL.len());
    line.push_str(old_new);
    line.push_str(plus_minus);
    line.push_str(contents);
    line.push_str(NORMAL);
    return line;
}

/// Format old and new lines in OLD and NEW colors.
///
/// No intra-line refinement.
#[must_use]
fn format_simple(prefixes: &Vec<String>, prefix_texts: &Vec<String>) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for (prefix, prefix_text) in prefixes.iter().zip(prefix_texts.iter()) {
        let old_new = if prefix.contains('+') { NEW } else { OLD };

        // If the user adds a section with a missing trailing newline, we want
        // to draw a highlighted-in-red newline symbol at the end of the last
        // line.
        let draw_missing_trailing_newline = prefix.contains('+') && !prefix_text.ends_with('\n');

        for (pos, line) in prefix_text.lines().enumerate() {
            let last_line = pos == prefix_text.lines().count() - 1;

            if last_line && draw_missing_trailing_newline {
                lines.push(format!("{NEW}+{line}{OLD}{INVERSE_VIDEO}⏎{NORMAL}"));
            } else {
                lines.push(format_simple_line(old_new, prefix, line));
            }
        }

        if !prefix_text.ends_with('\n') {
            let no_eof_newline_marker_guard = NO_EOF_NEWLINE_MARKER_HOLDER.lock().unwrap();
            let no_eof_newline_marker = no_eof_newline_marker_guard.as_ref().unwrap();
            lines.push(format!(
                "{NO_EOF_NEWLINE_COLOR}{no_eof_newline_marker}{NORMAL}"
            ));
        }
    }

    return lines;
}

/// LCS is O(m * n) complexity. If it gets too complex, refining will take too
/// much time and memory, so we shouldn't.
///
/// Ref: https://github.com/walles/riff/issues/35
fn too_large_to_refine(texts: &Vec<String>) -> bool {
    let size = texts.iter().map(|text| text.len()).sum::<usize>();

    // Around this point refining starts taking near one second on Johan's
    // laptop. Numbers have been invented through experimentation.
    return size > 13_000usize * 13_000usize;
}

/// Returns a vector of ANSI highlighted lines.
///
/// `prefix_texts` are multi line strings. Having or not having trailing
/// newlines will affect tokenization. The lines are not expected to have any
/// prefixes like `+` or `-`.
///
/// `prefixes` are the prefixes to use for each `prefix_texts` text.
#[must_use]
pub fn format(prefixes: &Vec<String>, prefix_texts: &Vec<String>) -> Vec<String> {
    if prefixes.len() < 2 {
        // Nothing to compare, we can't highlight anything
        return format_simple(prefixes, prefix_texts);
    }
    if !prefixes.iter().any(|prefix| prefix.contains('+')) {
        // Nothing added, we can't highlight anything
        return format_simple(prefixes, prefix_texts);
    }

    if too_large_to_refine(prefix_texts) {
        return format_simple(prefixes, prefix_texts);
    }

    // This is what all old texts will be compared against
    let new_text = prefix_texts.last().unwrap();
    let new_prefix = prefixes.last().unwrap();

    // These are all except for the last element
    let old_prefixes = &prefixes[0..prefixes.len() - 1];
    let old_prefix_texts = &prefix_texts[0..prefix_texts.len() - 1];

    let mut old_tokens = vec![];
    let mut new_tokens = None;
    let mut old_highlights = false;
    let mut new_unhighlighted = false;
    for old_text in old_prefix_texts.iter() {
        let (
            old_tokens_internal,
            mut new_tokens_internal,
            old_highlights_internal,
            new_unhighlighted_internal,
        ) = to_highlighted_tokens(old_text, new_text);

        old_tokens.push(old_tokens_internal);
        old_highlights |= old_highlights_internal;
        new_unhighlighted |= new_unhighlighted_internal;

        if new_tokens.is_none() {
            // First iteration, just remember the new tokens
            new_tokens = Some(new_tokens_internal);
            continue;
        }

        // Subsequent iterations, merge the new token styles
        for (new_token, new_token_internal) in
            new_tokens.unwrap().iter_mut().zip(new_tokens_internal)
        {
            if new_token_internal.style as u8 > new_token.style as u8 {
                new_token.style = new_token_internal.style;
            }
        }
    }

    // We should now have one token vector per old text
    assert_eq!(old_tokens.len(), prefix_texts.len() - 1);

    // Now turn all our token vectors (all vectors in old_tokens plus
    // new_tokens) into lines of highlighted text
    let new_line_count = count_lines(&new_tokens.unwrap());
    let all_line_counts_match = old_tokens
        .iter()
        .all(|tokens| count_lines(tokens) == new_line_count);

    let (old_style, new_style) = if old_highlights || new_unhighlighted || !all_line_counts_match {
        // Classical highlighting
        (LINE_STYLE_OLD, LINE_STYLE_NEW)
    } else {
        // Special adds-only highlighting
        (LINE_STYLE_OLD_FAINT, LINE_STYLE_ADDS_ONLY)
    };

    // First render() into strings, then to_lines() into lines
    let mut highlighted_lines = Vec::new();
    for (prefix, tokens) in old_prefixes.iter().zip(old_tokens.iter()) {
        let text = render(&old_style, prefix, tokens);
        highlighted_lines.extend(to_lines(&text));
    }
    let new_text = render(&new_style, &new_prefix, &new_tokens.unwrap());
    highlighted_lines.extend(to_lines(&new_text));

    return highlighted_lines;
}

/// Returns two vectors for old and new sections. The first bool is true if
/// there were any highlights found in the old text. The second bool is true if
/// any highlights were removed for readability in the new text.
///
/// `old_text` and `new_text` are multi lines strings. Having or not having
/// trailing newlines will affect tokenization. The lines are not expected to
/// have any prefixes like `+` or `-`.
pub fn to_highlighted_tokens(
    old_text: &str,
    new_text: &str,
) -> (Vec<StyledToken>, Vec<StyledToken>, bool, bool) {
    // Find diffs between adds and removals
    let mut old_tokens = Vec::new();
    let mut new_tokens = Vec::new();

    // Tokenize adds and removes before diffing them
    let mut tokenized_old = tokenizer::tokenize(old_text);
    let mut tokenized_new = tokenizer::tokenize(new_text);

    // Help visualize what actually happens in "No newline at end of file" diffs
    if old_text.ends_with('\n') && !new_text.ends_with('\n') {
        tokenized_old.insert(tokenized_old.len() - 1, "⏎");
    } else if new_text.ends_with('\n') && !old_text.ends_with('\n') {
        tokenized_new.insert(tokenized_new.len() - 1, "⏎");
    }

    let diff = tokenized_old.diff(&tokenized_new);
    let mut old_highlights = false;
    match diff {
        edit::Edit::Copy(tokens) => {
            for &token in tokens {
                // FIXME: "Copy" means that old and new are the same, why was
                // format_split() called on this non-difference?
                //
                // Get here using "git show 686f3d7ae | cargo run" with git 2.35.1
                old_tokens.push(StyledToken::new(token.to_string(), Style::Plain));
                new_tokens.push(StyledToken::new(token.to_string(), Style::Plain));
            }
        }
        edit::Edit::Change(diff) => {
            diff.into_iter()
                .map(|edit| {
                    match edit {
                        collection::Edit::Copy(token) => {
                            old_tokens.push(StyledToken::new(token.to_string(), Style::Plain));
                            new_tokens.push(StyledToken::new(token.to_string(), Style::Plain));
                        }
                        collection::Edit::Insert(token) => {
                            new_tokens
                                .push(StyledToken::new(token.to_string(), Style::Highlighted));
                        }
                        collection::Edit::Remove(token) => {
                            old_tokens
                                .push(StyledToken::new(token.to_string(), Style::Highlighted));
                            old_highlights = true;
                        }
                        collection::Edit::Change(_) => {
                            unimplemented!("Edit/Change/Change not implemented, help!")
                        }
                    };
                })
                .for_each(drop);
        }
    }

    bridge_consecutive_highlighted_tokens(&mut old_tokens);
    unhighlight_noisy_rows(&mut old_tokens);

    bridge_consecutive_highlighted_tokens(&mut new_tokens);
    let new_unhighlighted = unhighlight_noisy_rows(&mut new_tokens);
    highlight_trailing_whitespace(&mut new_tokens);
    highlight_nonleading_tabs(&mut new_tokens);

    return (old_tokens, new_tokens, old_highlights, new_unhighlighted);
}

/// Splits text into lines. If the text doesn't end in a newline, a no-newline
/// marker will be added at the end.
#[must_use]
fn to_lines(text: &str) -> Vec<String> {
    let mut lines: Vec<String> = Vec::new();

    for line in text.lines() {
        lines.push(line.to_string());
    }
    if (!text.is_empty()) && !text.ends_with('\n') {
        let no_eof_newline_marker_guard = NO_EOF_NEWLINE_MARKER_HOLDER.lock().unwrap();
        let no_eof_newline_marker = no_eof_newline_marker_guard.as_ref().unwrap();
        lines.push(format!(
            "{NO_EOF_NEWLINE_COLOR}{no_eof_newline_marker}{NORMAL}"
        ));
    }

    return lines;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    #[test]
    fn test_simple_format_adds_and_removes() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(format_simple("", ""), empty);

        // Test adds-only
        assert_eq!(
            format_simple("", "a\n"),
            ["".to_string() + NEW + "+a" + NORMAL]
        );
        assert_eq!(
            format_simple("", "a\nb\n"),
            [
                "".to_string() + NEW + "+a" + NORMAL,
                "".to_string() + NEW + "+b" + NORMAL,
            ]
        );

        // Test removes-only
        assert_eq!(
            format_simple("a\n", ""),
            ["".to_string() + OLD + "-a" + NORMAL]
        );
        assert_eq!(
            format_simple("a\nb\n", ""),
            [
                "".to_string() + OLD + "-a" + NORMAL,
                "".to_string() + OLD + "-b" + NORMAL,
            ]
        );
    }

    #[test]
    fn test_quote_change() {
        // FIXME: Get this from somewhere else?
        const NOT_INVERSE_VIDEO: &str = "\x1b[27m";

        let result = format(
            "<unchanged text between quotes>\n",
            "[unchanged text between quotes]\n",
        );
        assert_eq!(
            result,
            [
                format!(
                    "{OLD}-{INVERSE_VIDEO}<{NOT_INVERSE_VIDEO}unchanged text between quotes{INVERSE_VIDEO}>{NORMAL}"
                ),
                format!(
                    "{NEW}+{INVERSE_VIDEO}[{NOT_INVERSE_VIDEO}unchanged text between quotes{INVERSE_VIDEO}]{NORMAL}"
                ),
            ]
        )
    }

    #[test]
    fn test_almost_empty_changes() {
        let result = format("x\n", "");
        assert_eq!(result, [format!("{OLD}-x{NORMAL}"),]);

        let result = format("", "x\n");
        assert_eq!(result, [format!("{NEW}+x{NORMAL}"),]);
    }
}
