use crate::env::*;
use crate::error::*;
use crate::layout::*;
use crate::lexer::*;
use crate::virtual_key::*;
use ratatui::style::Color;
use std::sync::Arc;
#[derive(Debug)]
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, current: 0 }
    }

    fn peek(&self) -> Result<&Token, ParserError> {
        if self.current < self.tokens.len() {
            Ok(&self.tokens[self.current])
        } else {
            Err(ParserError::Err("EOF".to_string()))
        }
    }

    fn advance(&mut self) -> Result<&Token, ParserError> {
        if self.current < self.tokens.len() {
            self.current += 1;
            Ok(&self.tokens[self.current - 1])
        } else {
            Err(ParserError::Err("EOF".to_string()))
        }
    }

    fn consume(&mut self, ty: TokenType) -> Result<&Token, ParserError> {
        let t = self.peek()?;
        if t.token_type == ty {
            self.advance()
        } else {
            Err(ParserError::Err(format!(
                "Expected {:?}, found {:?}",
                ty, t.token_type
            )))
        }
    }

    pub fn parse(&mut self, env: &mut Env) -> Result<Layout, ParserError> {
        let mut layer = Vec::new();

        while self.current < self.tokens.len() {
            match self.peek()?.token_type {
                TokenType::Ident => self.parse_assign(env)?,
                TokenType::LineHead => {
                    let row = self.parse_line(env)?;
                    layer.push(row);
                }
                _ => break,
            }
        }

        Ok(Layout { layer })
    }

    pub fn parse_assign(&mut self, env: &mut Env) -> Result<(), ParserError> {
        let ident = self.consume(TokenType::Ident)?;
        let name = ident.value.clone();
        self.consume(TokenType::Equal)?;
        let v = self.parse_value(env)?;
        env.insert(&name, v);
        Ok(())
    }

    fn parse_value(&mut self, env: &Env) -> Result<Value, ParserError> {
        match self.peek()?.token_type {
            TokenType::Number => {
                let num = self.consume(TokenType::Number)?;
                Ok(Value::Number(num.value.parse()?))
            }
            TokenType::At => {
                self.consume(TokenType::At)?;
                self.consume(TokenType::LParen)?;
                let r = self.consume(TokenType::Number)?.clone();
                self.consume(TokenType::Comma)?;
                let g = self.consume(TokenType::Number)?.clone();
                self.consume(TokenType::Comma)?;
                let b = self.consume(TokenType::Number)?.clone();
                self.consume(TokenType::RParen)?;
                Ok(Value::RGB(
                    r.value.parse()?,
                    g.value.parse()?,
                    b.value.parse()?,
                ))
            }
            TokenType::Ident => {
                let ident = self.consume(TokenType::Ident)?;
                let name = ident.value.clone();
                match env.get(name.as_str()) {
                    Some(v) => Ok(*v),
                    None => Err(ParserError::Err(format!("Unbound Variable {:?}.", name))),
                }
            }
            _ => Err(ParserError::Err(
                "Expected Number, Identifier or RGB.".to_string(),
            )),
        }
    }

    fn parse_line(&mut self, env: &Env) -> Result<Vec<Button>, ParserError> {
        let mut row = Vec::new();
        self.consume(TokenType::LineHead)?;
        self.consume(TokenType::Split)?;

        while self.current < self.tokens.len() && self.peek()?.token_type != TokenType::LineTail {
            let name_token = self.consume(TokenType::Name)?;
            let name_str = name_token.value.clone();
            let mut binds = vec![];
            binds.push((Arc::from(name_str.as_str()), virtual_key_from_name(&name_str)));
            let mut attr = Attr::default(&name_str);

            while self.peek()?.token_type == TokenType::Comma {
                self.consume(TokenType::Comma)?;
                let name_token = self.consume(TokenType::Name)?;
                let name_str = name_token.value.clone();
                binds.push((Arc::from(name_str.as_str()), virtual_key_from_name(&name_str)));
            }

            if self.peek()?.token_type == TokenType::LBracket {
                self.parse_attr(&mut attr, env)?;
            }

            row.push(Button { attr, binds });

            self.consume(TokenType::Split)?;
        }

        self.consume(TokenType::LineTail)?;
        Ok(row)
    }

    fn parse_attr(&mut self, attr: &mut Attr, env: &Env) -> Result<(), ParserError> {
        // [width, height, border_color, highlight]
        self.consume(TokenType::LBracket)?; // [

        let mut pos = 0;

        while self.peek()?.token_type != TokenType::RBracket {
            let t_type = self.peek()?.token_type;

            if t_type == TokenType::Comma {
                self.consume(TokenType::Comma)?;
                pos += 1;
                continue;
            }

            match pos {
                0 => {
                    // width
                    if let Value::Number(w) = self.parse_value(env)? {
                        attr.width = w;
                    } else {
                        return Err(ParserError::Err("Width must be a number".into()));
                    }
                }
                1 => {
                    // height
                    if let Value::Number(h) = self.parse_value(env)? {
                        attr.height = h;
                    } else {
                        return Err(ParserError::Err("Height must be a number".into()));
                    }
                }
                2 => {
                    // border_color
                    if let Value::RGB(r, g, b) = self.parse_value(env)? {
                        attr.border_color = Some(Color::Rgb(r, g, b));
                    } else {
                        return Err(ParserError::Err("Border color must be RGB".into()));
                    }
                }
                3 => {
                    // highlight
                    if let Value::RGB(r, g, b) = self.parse_value(env)? {
                        attr.highlight = Some(Color::Rgb(r, g, b));
                    } else {
                        return Err(ParserError::Err("Highlight must be RGB".into()));
                    }
                }
                _ => {
                    self.advance()?;
                }
            }

            if self.peek()?.token_type == TokenType::Comma {
                self.consume(TokenType::Comma)?;
                pos += 1;
            }
        }

        self.consume(TokenType::RBracket)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::virtual_key::VirtualKey;

    // Helper to create a Name token
    fn t_name(val: &str) -> Token {
        Token {
            token_type: TokenType::Name,
            value: val.to_string(),
        }
    }

    #[test]
    fn test_parser_success() {
        let tokens = vec![
            Token {
                token_type: TokenType::LineHead,
                value: ":".into(),
            },
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            t_name("Tab"),
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            t_name("P"),
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            Token {
                token_type: TokenType::LineTail,
                value: "-".into(),
            },
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse(&mut Env::new()).unwrap();

        assert_eq!(result.layer.len(), 1);
        assert_eq!(result.layer[0][0].binds[0].0.as_ref(), "Tab");
        assert_eq!(result.layer[0][0].binds[0].1, Some(VirtualKey::Tab));
        assert_eq!(result.layer[0][1].binds[0].0.as_ref(), "P");
        assert_eq!(result.layer[0][1].binds[0].1, Some(VirtualKey::KeyP));
    }

    #[test]
    fn test_parser_missing_split() {
        let tokens = vec![
            Token {
                token_type: TokenType::LineHead,
                value: ":".into(),
            },
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            t_name("A"),
            t_name("B"),
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            Token {
                token_type: TokenType::LineTail,
                value: "-".into(),
            },
        ];

        let mut parser = Parser::new(tokens);
        let result = parser.parse(&mut Env::new());
        assert!(result.is_err());
    }

    #[test]
    fn test_parser_with_attr() {
        let tokens = vec![
            Token {
                token_type: TokenType::LineHead,
                value: ":".into(),
            },
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            t_name("Tab"),
            Token {
                token_type: TokenType::LBracket,
                value: "[".into(),
            },
            Token {
                token_type: TokenType::Number,
                value: "10".into(),
            },
            Token {
                token_type: TokenType::RBracket,
                value: "]".into(),
            },
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            t_name("P"),
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            Token {
                token_type: TokenType::LineTail,
                value: "-".into(),
            },
        ];

        let mut parser = Parser::new(tokens);

        let result = parser.parse(&mut Env::new()).unwrap();
        assert_eq!(result.layer.len(), 1);
        assert_eq!(result.layer[0][0].binds[0].0.as_ref(), "Tab");
        assert_eq!(result.layer[0][0].binds[0].1, Some(VirtualKey::Tab));
        assert_eq!(result.layer[0][0].attr.width, 10);
        assert_eq!(result.layer[0][1].binds[0].0.as_ref(), "P");
        assert_eq!(result.layer[0][1].binds[0].1, Some(VirtualKey::KeyP));
        assert_eq!(result.layer[0][1].attr.width, 4);
    }

    #[test]
    fn test_multi_binds() {
        let tokens = vec![
            Token {
                token_type: TokenType::LineHead,
                value: ":".into(),
            },
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            t_name("A"),
            Token {
                token_type: TokenType::Comma,
                value: ",".into(),
            },
            t_name("C"),
            Token {
                token_type: TokenType::Comma,
                value: ",".into(),
            },
            t_name("D"),
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            t_name("B"),
            Token {
                token_type: TokenType::Split,
                value: "|".into(),
            },
            Token {
                token_type: TokenType::LineTail,
                value: "-".into(),
            },
        ];

        let mut parser = Parser::new(tokens);

        let result = parser.parse(&mut Env::new()).unwrap();
        assert_eq!(result.layer.len(), 1);
        assert_eq!(
            result.layer[0][0].binds,
            [
                (Arc::from("A"), Some(VirtualKey::KeyA)),
                (Arc::from("C"), Some(VirtualKey::KeyC)),
                (Arc::from("D"), Some(VirtualKey::KeyD)),
            ]
        );
        assert_eq!(result.layer[0][1].binds[0].0.as_ref(), "B");
        assert_eq!(result.layer[0][1].binds[0].1, Some(VirtualKey::KeyB));
        assert_eq!(result.layer[0][1].attr.width, 4);
    }
}
