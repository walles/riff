#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Color {
    Red,
    Green,
    Default,
}

#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub struct AnsiStyle {
    pub inverse: bool,
    pub faint: bool,
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
                faint: false,
                color: Color::Default,
            })
        {
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

        if self.faint && !before.faint {
            // Faint on
            return_me.push_str("\x1b[2m");
        }
        if !self.faint && before.faint {
            // Faint off
            return_me.push_str("\x1b[22m");
        }

        if self.color != before.color {
            match self.color {
                Color::Red => return_me.push_str("\x1b[31m"),
                Color::Green => return_me.push_str("\x1b[32m"),
                Color::Default => return_me.push_str("\x1b[39m"),
            }
        }

        return return_me;
    }
}
