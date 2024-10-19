pub const OLD: &str = "\x1b[31m"; // Red
pub const NEW: &str = "\x1b[32m"; // Green
pub const PARSE_ERROR: &str = "\x1b[33m\x1b[7m"; // Inverse yellow

pub const INVERSE_VIDEO: &str = "\x1b[7m";
pub const NO_INVERSE_VIDEO: &str = "\x1b[27m";

pub const NO_EOF_NEWLINE_COLOR: &str = "\x1b[2m"; // Faint

pub const BOLD: &str = "\x1b[1m";
pub const FAINT: &str = "\x1b[2m";
pub const NORMAL_INTENSITY: &str = "\x1b[22m"; // Neither bold nor faint

pub const YELLOW: &str = "\x1b[33m";
pub const GREEN: &str = "\x1b[32m";
pub const CYAN: &str = "\x1b[36m";
pub const RED: &str = "\x1b[31m";

// Dark blue: https://en.wikipedia.org/wiki/ANSI_escape_code#8-bit
pub const BLUE_TO_END_OF_LINE: &str = "\x1b[48;5;17m\x1b[0K";

pub const NORMAL: &str = "\x1b[0m";
