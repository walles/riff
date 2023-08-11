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
    pub fn from(&self, before: &AnsiStyle) -> String {
        if self == before {
            return String::from("");
        }

        if self
            == (&AnsiStyle {
                inverse: false,
                color: Color::Default,
            })
        {
            // Special case for resetting to default style
            return String::from("\x1b[0m");
        }

        let mut return_me = String::new();
        if self.inverse && !before.inverse {
            return_me.push_str("\x1b[7m");
        }
        if !self.inverse && before.inverse {
            return_me.push_str("\x1b[27m");
        }

        if self.color == before.color {
            return return_me;
        }

        match self.color {
            Color::Red => return_me.push_str("\x1b[31m"),
            Color::Green => return_me.push_str("\x1b[32m"),
            Color::Default => return_me.push_str("\x1b[39m"),
        }
        return return_me;
    }
}
