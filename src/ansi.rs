#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnsiStyle {
    pub inverse: bool,
    pub color: Color,
}

impl AnsiStyle {
    /// Renders a (possibly empty) ANSI escape sequence to switch to this style
    /// from the before style.
    pub fn from(&self, _before: &AnsiStyle) -> &str {
        todo!("Return a string switching from the before style to this style");
    }
}
