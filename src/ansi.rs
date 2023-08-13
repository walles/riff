#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Default,
    Red,
    Green,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Weight {
    Normal,
    Bold,
    Faint,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnsiStyle {
    pub inverse: bool,
    pub weight: Weight,
    pub color: Color,
}

pub const ANSI_STYLE_NORMAL: AnsiStyle = AnsiStyle {
    inverse: false,
    weight: Weight::Normal,
    color: Color::Default,
};

impl AnsiStyle {
    /// Renders a (possibly empty) ANSI escape sequence to switch to this style
    /// from the before style.
    pub fn from(&self, before: &AnsiStyle) -> String {
        if self == before {
            return String::from("");
        }

        if self == &ANSI_STYLE_NORMAL {
            // Special case for resetting to default style
            return String::from("\x1b[0m");
        }

        let mut return_me = String::new();

        if self.inverse && !before.inverse {
            // Inverse on
            return_me.push_str("\x1b[7m");
        }
        if !self.inverse && before.inverse {
            // Inverse off
            return_me.push_str("\x1b[27m");
        }

        if self.weight != before.weight {
            match self.weight {
                Weight::Normal => return_me.push_str("\x1b[22m"),
                Weight::Bold => return_me.push_str("\x1b[1m"),
                Weight::Faint => return_me.push_str("\x1b[2m"),
            }
        }

        if self.color != before.color {
            match self.color {
                Color::Default => return_me.push_str("\x1b[39m"),
                Color::Red => return_me.push_str("\x1b[31m"),
                Color::Green => return_me.push_str("\x1b[32m"),
            }
        }

        return return_me;
    }
}
