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

pub struct TokenCollector<'a> {
    line_prefix: StyledToken,
    rendered: String,
    color: &'a str,
    is_inverse: bool,
    is_start_of_line: bool,
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

impl TokenCollector<'_> {
    #[must_use]
    pub fn create(line_prefix: StyledToken) -> Self {
        return TokenCollector {
            line_prefix,
            rendered: String::new(),
            color: NORMAL,
            is_inverse: false,
            is_start_of_line: true,
        };
    }

    pub fn push(&mut self, token: StyledToken) {
        if token.token == "\n" {
            self.rendered.push_str(NORMAL);
            self.rendered.push('\n');
            self.is_inverse = false;
            self.is_start_of_line = true;
            self.color = NORMAL;
            return;
        }

        if self.is_start_of_line {
            self.is_start_of_line = false;
            self.push(self.line_prefix.clone());
        }

        if token.style.is_inverse() && !self.is_inverse {
            self.rendered.push_str(INVERSE_VIDEO);
        }
        if self.is_inverse && !token.style.is_inverse() {
            self.rendered.push_str(NOT_INVERSE_VIDEO);
        }
        self.is_inverse = token.style.is_inverse();

        if token.style.color() != self.color {
            self.rendered.push_str(token.style.color());
            self.color = token.style.color();
        }

        self.rendered.push_str(&token.token);
    }

    #[must_use]
    pub fn render(&mut self) -> String {
        if !self.rendered.ends_with(&"\n") {
            // Don't forget to reset even if we don't end in a newline
            self.rendered.push_str(NORMAL);
        }

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
