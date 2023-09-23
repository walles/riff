/// Result of parsing a hunk header.
///
/// Example hunk header: `@@ -1,2 +1,2 @@ Initial commit`
#[derive(Debug, PartialEq)]
struct HunkHeader<'a> {
    pub old_line_count: usize,
    pub new_line_count: usize,
    pub title: Option<&'a str>,
}

impl<'a> HunkHeader<'a> {
    /// Parse a hunk header from a line of text.
    ///
    /// Returns `None` if the line is not a valid hunk header.
    pub fn parse(_line: &'a str) -> Option<Self> {
        return None;
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
