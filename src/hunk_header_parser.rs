/// Result of parsing a hunk header.
///
/// Example hunk header: `@@ -1,2 +1,2 @@ Initial commit`
///
/// This would mean "old line numbers are 1-2, and new line numbers are 1-2",
/// making the line counts 2 for both.
#[derive(Debug, PartialEq)]
pub(crate) struct HunkHeader<'a> {
    pub old_line_count: usize,
    pub new_line_count: usize,
    pub title: Option<&'a str>,
}

impl<'a> HunkHeader<'a> {
    /// Parse a hunk header from a line of text.
    ///
    /// Returns `None` if the line is not a valid hunk header.
    pub fn parse(line: &'a str) -> Option<Self> {
        let mut parts = line.splitn(5, ' ');
        let mut header = Self {
            old_line_count: 0,
            new_line_count: 0,
            title: None,
        };

        if parts.next()? != "@@" {
            return None;
        }

        // Example: "-1,2"
        let old_line_counts_part = parts.next()?;

        // Example: "+1,2"
        let new_line_counts_part = parts.next()?;

        // Skip the "@@" part
        let _at_at_part = parts.next()?;

        // Example: "Initial commit"
        let title_part = parts.next();

        // Parse the old line count
        let old_line_counts = old_line_counts_part
            .trim_start_matches('-')
            .split(',')
            .collect::<Vec<_>>();
        if old_line_counts.len() != 2 {
            return None;
        }
        header.old_line_count = 1 + old_line_counts[1].parse::<usize>().ok()?
            - old_line_counts[0].parse::<usize>().ok()?;

        // Parse the new line count
        let new_line_counts = new_line_counts_part
            .trim_start_matches('+')
            .split(',')
            .collect::<Vec<_>>();
        if new_line_counts.len() != 2 {
            return None;
        }
        header.new_line_count = 1 + new_line_counts[1].parse::<usize>().ok()?
            - new_line_counts[0].parse::<usize>().ok()?;

        // Grab the title if there is one
        header.title = title_part;

        Some(header)
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
                old_line_count: 2,
                new_line_count: 2,
                title: None,
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@")
        );
    }

    #[test]
    fn test_hunk_header_with_title() {
        assert_eq!(
            Some(HunkHeader {
                old_line_count: 2,
                new_line_count: 2,
                title: Some("Hello there"),
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@ Hello there")
        );
    }

    #[test]
    fn test_hunk_header_with_spaced_title() {
        assert_eq!(
            Some(HunkHeader {
                old_line_count: 2,
                new_line_count: 2,
                title: Some("Hello  there"),
            }),
            HunkHeader::parse("@@ -1,2 +1,2 @@ Hello  there")
        );
    }
}
