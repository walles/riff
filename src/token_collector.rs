use crate::constants::*;

#[derive(Clone)]
pub enum Style {
    Old,
    OldInverse,
    New,
    NewInverse,
}

#[derive(Clone)]
pub struct StyledToken {
    token: String,
    style: Style,
}

impl StyledToken {
    pub fn new(token: String, style: Style) -> StyledToken {
        return StyledToken { token, style };
    }
}

pub struct TokenCollector {
    line_prefix: StyledToken,
    rendered: String,
    current_row: Vec<StyledToken>,
}

impl Style {
    #[must_use]
    pub fn is_inverse(&self) -> bool {
        match self {
            Style::OldInverse | Style::NewInverse => {
                return true;
            }
            _ => {
                return false;
            }
        }
    }

    #[must_use]
    pub fn color<'a>(&self) -> &'a str {
        match self {
            Style::Old => {
                return OLD;
            }
            Style::OldInverse => {
                return OLD;
            }
            Style::New => {
                return NEW;
            }
            Style::NewInverse => {
                return NEW;
            }
        }
    }
}

impl TokenCollector {
    #[must_use]
    pub fn create(line_prefix: StyledToken) -> Self {
        return TokenCollector {
            line_prefix,
            rendered: String::new(),
            current_row: Vec::new(),
        };
    }

    pub fn push(&mut self, token: StyledToken) {
        if token.token == "\n" {
            self.commit();
            self.rendered.push('\n');
            return;
        }

        self.current_row.push(token);
    }

    fn commit(&mut self) {
        if self.current_row.is_empty() {
            return;
        }

        // Set inverse from prefix
        let mut is_inverse = self.line_prefix.style.is_inverse();
        if is_inverse {
            self.rendered.push_str(INVERSE_VIDEO);
        }

        // Set line color from prefix
        let mut color = self.line_prefix.style.color();
        self.rendered.push_str(self.line_prefix.style.color());

        // Render prefix
        self.rendered.push_str(&self.line_prefix.token);

        for token in &self.current_row {
            if token.style.is_inverse() && !is_inverse {
                self.rendered.push_str(INVERSE_VIDEO);
            }
            if is_inverse && !token.style.is_inverse() {
                self.rendered.push_str(NOT_INVERSE_VIDEO);
            }
            is_inverse = token.style.is_inverse();

            if token.style.color() != color {
                self.rendered.push_str(token.style.color());
                color = token.style.color();
            }

            self.rendered.push_str(&token.token);
        }

        self.rendered.push_str(NORMAL);
        self.current_row.clear();
    }

    #[must_use]
    pub fn render(&mut self) -> String {
        self.commit();

        return self.rendered.clone();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_basic() {
        let mut test_me = TokenCollector::create(StyledToken {
            token: "+".to_string(),
            style: Style::New,
        });

        test_me.push(StyledToken {
            token: "hej".to_string(),
            style: Style::New,
        });

        test_me.push(StyledToken {
            token: "\n".to_string(),
            style: Style::New,
        });

        assert_eq!(test_me.render(), format!("{}+hej{}\n", NEW, NORMAL));
    }
}
