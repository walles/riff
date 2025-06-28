use similar::capture_diff_slices;

use crate::ansi::Color::Green;
use crate::ansi::Color::Red;
use crate::ansi::Color::Yellow;
use crate::ansi::Weight::Faint;
use crate::ansi::ANSI_STYLE_NORMAL;
use crate::constants::*;
use crate::line_collector::NO_EOF_NEWLINE_MARKER_HOLDER;
use crate::token_collector::*;
use crate::tokenizer;

#[derive(Debug, Clone, Copy, PartialEq)]
pub(crate) struct Formatter {
    line_style_old: LineStyle,
    line_style_new: LineStyle,
}

impl Formatter {
    // The other LineStyles live in token_collector.rs

    pub(crate) fn default() -> Self {
        Formatter {
            line_style_old: LineStyle {
                prefix_style: ANSI_STYLE_NORMAL.with_color(Red),
                unchanged_style: ANSI_STYLE_NORMAL.with_color(Red),
                midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
                highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
            },
            line_style_new: LineStyle {
                prefix_style: ANSI_STYLE_NORMAL.with_color(Green),
                unchanged_style: ANSI_STYLE_NORMAL.with_color(Green),
                midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
                highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
            },
        }
    }

    pub(crate) fn yellow() -> Self {
        Formatter {
            line_style_old: LineStyle {
                prefix_style: ANSI_STYLE_NORMAL.with_color(Red),
                unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow).with_weight(Faint),
                midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
                highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
            },
            line_style_new: LineStyle {
                prefix_style: ANSI_STYLE_NORMAL.with_color(Green),
                unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
                midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
                highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
            },
        }
    }

    /// Format old and new lines in OLD and NEW colors.
    ///
    /// No intra-line refinement.
    #[must_use]
    fn format_simple(&self, prefixes: &[&str], prefix_texts: &[&str]) -> Vec<String> {
        let mut lines: Vec<String> = Vec::new();

        for (prefix, prefix_text) in prefixes.iter().zip(prefix_texts.iter()) {
            let line_style = if prefix.contains('+') {
                &self.line_style_new
            } else {
                &self.line_style_old
            };

            // If the user adds a section with a missing trailing newline, we want
            // to draw a highlighted-in-red newline symbol at the end of the last
            // line.
            let draw_missing_trailing_newline =
                prefix.contains('+') && !prefix_text.ends_with('\n');

            let last_pos = prefix_text.lines().count() - 1;
            for (pos, line) in prefix_text.lines().enumerate() {
                let last_line = pos == last_pos;

                let to_push = render_row(
                    line_style,
                    prefix,
                    &[StyledToken::new(
                        line.to_string(),
                        Style::DiffPartMidlighted,
                    )],
                    false,
                );
                if last_line && draw_missing_trailing_newline {
                    lines.push(format!("{to_push}{OLD}{INVERSE_VIDEO}⏎{NORMAL}"));
                } else {
                    lines.push(to_push);
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
    fn too_large_to_refine(texts: &[&str]) -> bool {
        let size = texts.iter().map(|text| text.len()).sum::<usize>();

        // This limit used to be higher, but then refining took too long on
        // Johan's laptop. This number is based on very limited experimentation.
        return size > 300_000usize;
    }

    /// Returns a vector of ANSI highlighted lines.
    ///
    /// `prefix_texts` are multi line strings. Having or not having trailing
    /// newlines will affect tokenization. The lines are not expected to have any
    /// prefixes like `+` or `-`.
    ///
    /// `prefixes` are the prefixes to use for each `prefix_texts` text.
    #[must_use]
    pub fn format(&self, prefixes: &[&str], prefix_texts: &[&str]) -> Vec<String> {
        if prefixes.len() < 2 {
            // Nothing to compare, we can't highlight anything
            return self.format_simple(prefixes, prefix_texts);
        }
        if !prefixes.iter().any(|prefix| prefix.contains('+')) {
            // Nothing added, we can't highlight anything
            return self.format_simple(prefixes, prefix_texts);
        }

        if Formatter::too_large_to_refine(prefix_texts) {
            return self.format_simple(prefixes, prefix_texts);
        }

        // This is what all old texts will be compared against
        let new_text = prefix_texts.last().unwrap();
        let new_prefix = prefixes.last().unwrap();

        // These are all except for the last element
        let old_prefixes = &prefixes[0..prefixes.len() - 1];
        let old_prefix_texts = &prefix_texts[0..prefix_texts.len() - 1];

        let mut old_tokens = vec![];
        let mut new_tokens = vec![];
        for old_text in old_prefix_texts.iter() {
            let (old_tokens_internal, new_tokens_internal) = diff(old_text, new_text);

            old_tokens.push(old_tokens_internal);

            if new_tokens.is_empty() {
                // First iteration, just remember the new tokens
                new_tokens = new_tokens_internal;
                continue;
            }

            // Subsequent iterations, merge the new token styles
            for (new_token, new_token_internal) in new_tokens.iter_mut().zip(new_tokens_internal) {
                if new_token_internal.style as u8 > new_token.style as u8 {
                    new_token.style = new_token_internal.style;
                }
            }
        }

        // We should now have one token vector per old text
        assert_eq!(old_tokens.len(), prefix_texts.len() - 1);

        // Now turn all our token vectors (all vectors in old_tokens plus
        // new_tokens) into lines of highlighted text

        // First render() into strings, then to_lines() into lines
        let mut highlighted_lines = Vec::new();
        for (prefix, tokens) in old_prefixes.iter().zip(old_tokens.iter()) {
            let text = render(&self.line_style_old, prefix, tokens);
            highlighted_lines.extend(to_lines(&text));
        }
        let new_text = render(&self.line_style_new, new_prefix, &new_tokens);
        highlighted_lines.extend(to_lines(&new_text));

        return highlighted_lines;
    }
}

fn should_highlight_change(tokens: &[&str], whitespace_only_is_fine: bool) -> bool {
    let whitespace_only = tokens
        .iter()
        .all(|token| token.chars().all(|c| c.is_whitespace()));
    let contains_newline = tokens.iter().any(|token| token.contains('\n'));
    let is_newline_marker = tokens.len() == 2 && tokens[0] == "⏎" && tokens[1] == "\n";

    if is_newline_marker {
        return true;
    }

    if (whitespace_only && !whitespace_only_is_fine) || contains_newline {
        return false;
    }

    return true;
}

fn is_whitepace_replacement(old_run: &[&str], new_run: &[&str]) -> bool {
    let old_whitespace_only = old_run
        .iter()
        .all(|token| token.chars().all(|c| c.is_whitespace()));
    let new_whitespace_only = new_run
        .iter()
        .all(|token| token.chars().all(|c| c.is_whitespace()));

    return old_whitespace_only && new_whitespace_only;
}

fn push_styled_tokens(destination: &mut Vec<StyledToken>, run: Vec<&str>, style: Style) {
    // Except for just pushing the tokens, any leading or trailing
    // whitespace-only tokens in the run should always be midlighted.

    let first_non_leading_whitespace_index = run
        .iter()
        .position(|token| !token.chars().all(|c| c.is_whitespace()));

    let last_non_trailing_whitespace_index = run
        .iter()
        .rposition(|token| !token.chars().all(|c| c.is_whitespace()));

    for (index, token) in run.iter().enumerate() {
        let in_leading_whitespace = first_non_leading_whitespace_index.is_some()
            && index < first_non_leading_whitespace_index.unwrap();
        let in_trailing_whitespace = last_non_trailing_whitespace_index.is_some()
            && index > last_non_trailing_whitespace_index.unwrap();
        let style = if in_leading_whitespace || in_trailing_whitespace {
            Style::DiffPartMidlighted
        } else {
            style
        };

        destination.push(StyledToken::new(token.to_string(), style));
    }
}

/// Returns two vectors of highlighted tokens, one for the old text and one for
/// the new.
///
/// `old_text` and `new_text` are multi line strings. Having or not having
/// trailing newlines will affect tokenization. The lines are not expected to
/// have any prefixes like `+` or `-`.
///
/// Conflict diffs are highlighted somewhat differently from regular diffs.
pub fn diff(old_text: &str, new_text: &str) -> (Vec<StyledToken>, Vec<StyledToken>) {
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

    let diff = capture_diff_slices(similar::Algorithm::Patience, &tokenized_old, &tokenized_new);
    let mut old_start_of_line = true;
    let mut new_start_of_line = true;
    for change in diff.iter() {
        match change {
            similar::DiffOp::Equal {
                old_index,
                new_index,
                len,
            } => {
                for token in tokenized_old.iter().skip(*old_index).take(*len) {
                    old_tokens.push(StyledToken::new(
                        token.to_string(),
                        Style::DiffPartUnchanged,
                    ));
                }
                for token in tokenized_new.iter().skip(*new_index).take(*len) {
                    new_tokens.push(StyledToken::new(
                        token.to_string(),
                        Style::DiffPartUnchanged,
                    ));
                }
            }

            similar::DiffOp::Insert {
                old_index: _,
                new_index,
                new_len,
            } => {
                let run = tokenized_new[*new_index..*new_index + *new_len].to_vec();
                let style = if should_highlight_change(&run, !new_start_of_line) {
                    Style::DiffPartHighlighted
                } else {
                    Style::DiffPartMidlighted
                };
                push_styled_tokens(&mut new_tokens, run, style);
            }

            similar::DiffOp::Delete {
                old_index,
                old_len,
                new_index: _,
            } => {
                let run = tokenized_old[*old_index..*old_index + *old_len].to_vec();
                let style = if should_highlight_change(&run, !old_start_of_line) {
                    Style::DiffPartHighlighted
                } else {
                    Style::DiffPartMidlighted
                };
                push_styled_tokens(&mut old_tokens, run, style);
            }

            similar::DiffOp::Replace {
                old_index,
                old_len,
                new_index,
                new_len,
            } => {
                let old_run = tokenized_old[*old_index..*old_index + *old_len].to_vec();
                let new_run = tokenized_new[*new_index..*new_index + *new_len].to_vec();

                let style = if should_highlight_change(&old_run, false)
                    && should_highlight_change(&new_run, false)
                    && !is_whitepace_replacement(&old_run, &new_run)
                {
                    Style::DiffPartHighlighted
                } else {
                    Style::DiffPartMidlighted
                };

                push_styled_tokens(&mut old_tokens, old_run, style);
                push_styled_tokens(&mut new_tokens, new_run, style);
            }
        }

        old_start_of_line = old_tokens
            .last()
            .is_none_or(|token| token.token.ends_with('\n'));
        new_start_of_line = new_tokens
            .last()
            .is_none_or(|token| token.token.ends_with('\n'));
    }

    // Refine old tokens highlighting
    bridge_consecutive_highlighted_tokens(&mut old_tokens);

    // Refine new tokens highlighting
    bridge_consecutive_highlighted_tokens(&mut new_tokens);
    errorlight_trailing_whitespace(&mut new_tokens);
    errorlight_nonleading_tabs(&mut new_tokens);

    return (old_tokens, new_tokens);
}

/// Highlight single space between two highlighted tokens
fn bridge_consecutive_highlighted_tokens(tokens: &mut [StyledToken]) {
    fn bridgable(candidate: &StyledToken) -> bool {
        if candidate.style as u8 > Style::DiffPartUnchanged as u8 {
            return false;
        }
        if candidate.token.len() != 1 {
            return false;
        }

        let rune = candidate.token.chars().next().unwrap();
        return rune == ' ' || rune.is_ascii_punctuation();
    }

    for i in 1..(tokens.len() - 1) {
        if tokens[i - 1].style != Style::DiffPartHighlighted
            || tokens[i + 1].style != Style::DiffPartHighlighted
        {
            continue;
        }
        if bridgable(&tokens[i - 1]) || !bridgable(&tokens[i]) || bridgable(&tokens[i + 1]) {
            continue;
        }

        tokens[i].style = Style::DiffPartHighlighted;
    }
}

fn errorlight_trailing_whitespace(tokens: &mut [StyledToken]) {
    let mut in_trailer = true;
    for token in tokens.iter_mut().rev() {
        if token.token == "\n" {
            in_trailer = true;
            continue;
        }

        if in_trailer && token.is_whitespace() {
            token.style = Style::Error;
            continue;
        }

        in_trailer = false;
    }
}

fn errorlight_nonleading_tabs(tokens: &mut [StyledToken]) {
    let mut leading = true;
    for token in tokens.iter_mut() {
        if token.token == "\n" {
            leading = true;
            continue;
        }

        if token.token != "\t" {
            leading = false;
        }

        if token.token == "\t" && !leading {
            token.style = Style::Error;
        }
    }
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
pub(crate) mod tests {
    use super::*;

    #[cfg(test)]
    use pretty_assertions::assert_eq;

    const LINE_STYLE_OLD: LineStyle = {
        LineStyle {
            prefix_style: ANSI_STYLE_NORMAL.with_color(Red),
            unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
            midlighted_style: ANSI_STYLE_NORMAL.with_color(Red),
            highlighted_style: ANSI_STYLE_NORMAL.with_color(Red).with_inverse(true),
        }
    };

    const LINE_STYLE_NEW: LineStyle = {
        LineStyle {
            prefix_style: ANSI_STYLE_NORMAL.with_color(Green),
            unchanged_style: ANSI_STYLE_NORMAL.with_color(Yellow),
            midlighted_style: ANSI_STYLE_NORMAL.with_color(Green),
            highlighted_style: ANSI_STYLE_NORMAL.with_color(Green).with_inverse(true),
        }
    };

    pub(crate) const FORMATTER: Formatter = Formatter {
        line_style_old: LINE_STYLE_OLD,
        line_style_new: LINE_STYLE_NEW,
    };

    #[test]
    fn test_simple_format_adds_and_removes() {
        let empty: Vec<String> = Vec::new();
        assert_eq!(FORMATTER.format_simple(&[], &[]), empty);

        // Test adds-only
        assert_eq!(
            FORMATTER.format_simple(&["+"], &["a\n"]),
            ["".to_string() + GREEN + "+a" + NORMAL]
        );
        assert_eq!(
            FORMATTER.format_simple(&["+"], &["a\nb\n"]),
            [
                "".to_string() + GREEN + "+a" + NORMAL,
                "".to_string() + GREEN + "+b" + NORMAL,
            ]
        );

        // Test removes-only
        assert_eq!(
            FORMATTER.format_simple(&["-"], &["a\n"]),
            ["".to_string() + OLD + "-a" + NORMAL]
        );
        assert_eq!(
            FORMATTER.format_simple(&["-"], &["a\nb\n"]),
            [
                "".to_string() + OLD + "-a" + NORMAL,
                "".to_string() + OLD + "-b" + NORMAL,
            ]
        );
    }

    /// `format_simple()` turned out to have quadratic complexity. If this test
    /// hangs, that's probably what happened again.
    #[test]
    fn test_format_simple_complexity() {
        // Values from when this file was added in a single commit:
        // https://github.com/walles/moar/blob/59270d6f8cf454f7a79fcde36a7fcf794768ced9/sample-files/large-git-log-patch.txt
        let lines = 300_000;
        let line_length = 50;
        let mut text = String::new();
        for _ in 0..lines {
            text.push_str(&"a".repeat(line_length));
            text.push('\n');
        }

        let prefixes = vec!["+"];
        let texts = vec![text.as_str()];

        let result = FORMATTER.format_simple(&prefixes, &texts);
        assert_eq!(text.lines().count(), result.len());
    }

    #[test]
    fn test_quote_change() {
        let result = FORMATTER.format(
            &["-", "+"],
            &[
                "<unchanged text between quotes>\n",
                "[unchanged text between quotes]\n",
            ],
        );
        assert_eq!(
            result,
            [
                format!(
                    "{OLD}-{INVERSE_VIDEO}<{NO_INVERSE_VIDEO}{YELLOW}unchanged text between quotes{INVERSE_VIDEO}{OLD}>{NORMAL}"
                ),
                format!(
                    "{GREEN}+{INVERSE_VIDEO}[{NO_INVERSE_VIDEO}{YELLOW}unchanged text between quotes{INVERSE_VIDEO}{GREEN}]{NORMAL}"
                ),
            ]
        )
    }

    #[test]
    fn test_almost_empty_changes() {
        let result = FORMATTER.format(&["-"], &["x\n"]);
        assert_eq!(result, [format!("{OLD}-x{NORMAL}"),]);

        let result = FORMATTER.format(&["+"], &["x\n"]);
        assert_eq!(result, [format!("{GREEN}+x{NORMAL}"),]);
    }

    #[test]
    fn test_space_highlighting() {
        // Add new initial spacing (indentation). We don't want to highlight indentation.
        let (_, new_tokens) = diff("x", " x");
        assert_eq!(
            new_tokens,
            vec![
                StyledToken::new(" ".to_string(), Style::DiffPartMidlighted),
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged)
            ]
        );

        // Increase indentation. Do not highlight this.
        let (_, new_tokens) = diff(" x", "  x");
        assert_eq!(
            new_tokens,
            vec![
                StyledToken::new("  ".to_string(), Style::DiffPartMidlighted),
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged)
            ]
        );

        // Add a new internal space. We do want to highlight this.
        //
        // This particular example is from a Markdown heading where someone forgot
        // the space after the leading `#`.
        let (_, new_tokens) = diff("#x", "# x");
        assert_eq!(
            new_tokens,
            vec![
                StyledToken::new("#".to_string(), Style::DiffPartUnchanged),
                StyledToken::new(" ".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged)
            ]
        );

        // Increase internal space. We do not want to highlight this. Probably code reformatting.
        let (_, new_tokens) = diff("x y", "x  y");
        assert_eq!(
            new_tokens,
            vec![
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("  ".to_string(), Style::DiffPartMidlighted),
                StyledToken::new("y".to_string(), Style::DiffPartUnchanged)
            ]
        );

        // Remove trailing space. We do want to highlight this.
        let (old_tokens, _) = diff("x ", "x");
        assert_eq!(
            old_tokens,
            vec![
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new(" ".to_string(), Style::DiffPartHighlighted)
            ]
        );
    }

    fn is_char_bridged(before: char, victim: char, after: char) -> bool {
        let mut row = [
            StyledToken::new(before.to_string(), Style::DiffPartHighlighted),
            StyledToken::new(victim.to_string(), Style::DiffPartUnchanged),
            StyledToken::new(after.to_string(), Style::DiffPartHighlighted),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        return row[1].style == Style::DiffPartHighlighted;
    }

    #[test]
    fn test_bridge_consecutive_highlighted_tokens() {
        assert!(is_char_bridged('a', ' ', 'b'));
        assert!(is_char_bridged('>', ' ', '5'));
        assert!(is_char_bridged('a', ' ', ' ')); // Because the second space is highlighted
        assert!(!is_char_bridged('\'', '1', '\''));
        assert!(is_char_bridged('a', '.', 'b')); // Bridge separators
    }

    #[test]
    fn test_four_tokens_highlighting() {
        let mut row = [
            StyledToken::new("\n".to_string(), Style::DiffPartHighlighted),
            StyledToken::new("*".to_string(), Style::DiffPartHighlighted),
            StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("Hello".to_string(), Style::DiffPartHighlighted),
        ];

        bridge_consecutive_highlighted_tokens(&mut row);

        assert_eq!(
            row,
            [
                StyledToken::new("\n".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("*".to_string(), Style::DiffPartHighlighted),
                StyledToken::new(" ".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("Hello".to_string(), Style::DiffPartHighlighted),
            ]
        );
    }

    #[test]
    fn test_errorlight_nonleading_tabs() {
        let mut tokens = vec![
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("one".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\n".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("two".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut tokens);

        assert_eq!(
            tokens,
            vec![
                StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("one".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\n".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("two".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\t".to_string(), Style::Error),
            ]
        );
    }

    #[test]
    fn test_errorlight_trailing_whitespace() {
        // Just a whitespace
        let mut row = [StyledToken::new(" ".to_string(), Style::DiffPartUnchanged)];
        errorlight_trailing_whitespace(&mut row);
        assert_eq!(row, [StyledToken::new(" ".to_string(), Style::Error)]);

        // Trailing whitespace
        let mut row = [
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new(" ".to_string(), Style::Error),
            ]
        );

        // Leading whitespace
        let mut row = [
            StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_trailing_whitespace(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new(" ".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            ]
        );
    }

    #[test]
    fn test_add_nonleading_tab() {
        // Trailing TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\t".to_string(), Style::Error),
            ]
        );

        // Middle TAB
        let mut row = [
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("y".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("\t".to_string(), Style::Error),
                StyledToken::new("y".to_string(), Style::DiffPartUnchanged),
            ]
        );

        // Leading TAB (don't highlight)
        let mut row = [
            StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
            StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
        ];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [
                StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),
                StyledToken::new("x".to_string(), Style::DiffPartUnchanged),
            ]
        );

        // Single TAB (don't highlight because it is leading)
        let mut row = [StyledToken::new("\t".to_string(), Style::DiffPartUnchanged)];
        errorlight_nonleading_tabs(&mut row);
        assert_eq!(
            row,
            [StyledToken::new("\t".to_string(), Style::DiffPartUnchanged),]
        );
    }

    #[test]
    fn test_push_styled_tokens() {
        let mut tokens = Vec::new();
        push_styled_tokens(&mut tokens, vec!["a", "b", "c"], Style::DiffPartHighlighted);
        assert_eq!(
            tokens,
            vec![
                StyledToken::new("a".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("b".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("c".to_string(), Style::DiffPartHighlighted),
            ]
        );

        let mut tokens = Vec::new();
        push_styled_tokens(&mut tokens, vec![" ", "b", "c"], Style::DiffPartHighlighted);
        assert_eq!(
            tokens,
            vec![
                StyledToken::new(" ".to_string(), Style::DiffPartMidlighted),
                StyledToken::new("b".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("c".to_string(), Style::DiffPartHighlighted),
            ]
        );

        let mut tokens = Vec::new();
        push_styled_tokens(&mut tokens, vec!["a", "b", " "], Style::DiffPartHighlighted);
        assert_eq!(
            tokens,
            vec![
                StyledToken::new("a".to_string(), Style::DiffPartHighlighted),
                StyledToken::new("b".to_string(), Style::DiffPartHighlighted),
                StyledToken::new(" ".to_string(), Style::DiffPartMidlighted),
            ]
        );

        let mut tokens = Vec::new();
        push_styled_tokens(&mut tokens, vec![" ", "b", " "], Style::DiffPartHighlighted);
        assert_eq!(
            tokens,
            vec![
                StyledToken::new(" ".to_string(), Style::DiffPartMidlighted),
                StyledToken::new("b".to_string(), Style::DiffPartHighlighted),
                StyledToken::new(" ".to_string(), Style::DiffPartMidlighted),
            ]
        );
    }
}
