#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    Yellow,
    Cyan,
    Default,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AnsiStyle {
    pub inverse: bool,
    pub color: Color,
}

impl AnsiStyle {
    pub fn from(&self, &before: &AnsiStyle) -> &str {
        todo!("Return a string switching from the before style to this style");
    }
}
