use crate::constants::*;

/// Result of parsing a hunk header: <https://en.wikipedia.org/wiki/Diff#Unified_format>
///
/// Example hunk header: `@@ -1,2 +1,2 @@ Initial commit`
///
/// This would mean "old line numbers are 1-2, and new line numbers are 1-2",
/// making the line counts 2 for both.
#[derive(Debug, PartialEq)]
pub(crate) struct HunkHeader<'a> {
    pub old_start: usize,
    pub old_linecount: usize,

    pub new_start: usize,
    pub new_linecount: usize,

    pub title: Option<&'a str>,
}

const HUNK_HEADER: &str = "\x1b[36m"; // Cyan

impl<'a> HunkHeader<'a> {
    /// Parse a hunk header from a line of text.
    ///
    /// Returns `None` if the line is not a valid hunk header.
    pub fn parse(line: &'a str) -> Option<Self> {
        let mut parts = line.splitn(5, ' ');

        if parts.next()? != "@@" {
            return None;
        }

        // Example: "-1,2", or just "-55"
        let old_line_counts_part = parts.next()?;

        // Example: "+1,2", or just "+55"
        let new_line_counts_part = parts.next()?;

        // Skip the "@@" part
        let _at_at_part = parts.next()?;

        // Example: "Initial commit"
        let title = parts.next();

        // Parse the old line count
        let old_line_numbers = old_line_counts_part
            .trim_start_matches('-')
            .split(',')
            .collect::<Vec<_>>();
        if old_line_numbers.is_empty() || old_line_numbers.len() > 2 {
            return None;
        }
        let old_start = old_line_numbers[0].parse::<usize>().ok()?;
        let old_linecount = if old_line_numbers.len() == 2 {
            old_line_numbers[1].parse::<usize>().ok()?
        } else {
            1
        };

        // Parse the new line count
        let new_line_numbers = new_line_counts_part
            .trim_start_matches('+')
            .split(',')
            .collect::<Vec<_>>();
        if new_line_numbers.is_empty() || new_line_numbers.len() > 2 {
            return None;
        }
        let new_start = new_line_numbers[0].parse::<usize>().ok()?;
        let new_linecount = if new_line_numbers.len() == 2 {
            new_line_numbers[1].parse::<usize>().ok()?
        } else {
            1
        };

        Some(HunkHeader {
            old_start,
            old_linecount,
            new_start,
            new_linecount,
            title,
        })
    }

    /// Render into an ANSI highlighted string
    pub fn render(&self) -> String {
        let old_linecount = if self.old_linecount == 1 {
            self.old_start.to_string()
        } else {
            format!("{},{}", self.old_start, self.old_linecount)
        };
        let new_linecount = if self.new_linecount == 1 {
            self.new_start.to_string()
        } else {
            format!("{},{}", self.new_start, self.new_linecount)
        };

        if let Some(title) = self.title {
            // Highlight the title if we have one
            return format!(
                "{HUNK_HEADER}{FAINT}@@ -{} +{} @@ {BOLD}{}{NORMAL}",
                old_linecount, new_linecount, title
            );
        }

        return format!(
            "{HUNK_HEADER}@@ -{} +{} @@{NORMAL}",
            old_linecount, new_linecount
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_non_hunk_header() {
        assert_eq!(None, HunkHeader::parse("This is not a hunk header"));
        assert_eq!(None, HunkHeader::parse(""));
    }

    #[test]
    fn test_simple_hunk_header() {
        assert_eq!(
            Some(HunkHeader {
                old_start: 1,
                old_linecount: 2,
                new_start: 1,
                new_linecount: 2,
                title: None,
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@")
        );
    }

    #[test]
    fn test_hunk_header_with_title() {
        assert_eq!(
            Some(HunkHeader {
                old_start: 1,
                old_linecount: 2,
                new_start: 1,
                new_linecount: 2,
                title: Some("Hello there"),
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@ Hello there")
        );
    }

    #[test]
    fn test_hunk_header_with_spaced_title() {
        assert_eq!(
            Some(HunkHeader {
                old_start: 1,
                old_linecount: 2,
                new_start: 1,
                new_linecount: 2,
                title: Some("Hello  there"),
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@ Hello  there")
        );
    }

    #[test]
    fn test_hunk_header_with_default_linecounts() {
        assert_eq!(
            Some(HunkHeader {
                old_start: 5,
                old_linecount: 1,
                new_start: 6,
                new_linecount: 1,
                title: None,
            }),
            HunkHeader::parse("@@ -5 +6 @@")
        );
    }
}
