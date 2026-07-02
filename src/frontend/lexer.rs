use crate::{
    error::{ScriptError, ScriptResult},
    frontend::token::{Token, TokenType},
};

pub struct Lexer {
    source: Vec<char>,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    column: usize,
    start_column: usize,
}

impl Lexer {
    pub fn new(source: &str) -> Self {
        Lexer {
            source: source.chars().collect(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            column: 1,
            start_column: 1,
        }
    }

    pub fn tokenize(mut self) -> ScriptResult<Vec<Token>> {
        while !self.is_at_end() {
            self.start = self.current;
            self.start_column = self.column;
            self.scan_token()?;
        }

        self.tokens.push(Token::new(
            TokenType::Eof,
            String::new(),
            self.line,
            self.column,
        ));

        Ok(self.tokens)
    }

    fn scan_token(&mut self) -> ScriptResult<()> {
        let char = self.advance();

        match char {
            ' ' | '\r' | '\t' => {}
            '\n' => {
                self.line += 1;
                self.column = 1;
            }

            '(' => self.add_token(TokenType::LParen),
            ')' => self.add_token(TokenType::RParen),
            '{' => self.add_token(TokenType::LBrace),
            '}' => self.add_token(TokenType::RBrace),
            '[' => self.add_token(TokenType::LBracket),
            ']' => self.add_token(TokenType::RBracket),
            ',' => self.add_token(TokenType::Comma),
            ';' => self.add_token(TokenType::Semicolon),
            ':' => self.add_token(TokenType::Colon),
            '.' => self.add_token(TokenType::Dot),

            '-' => self.add_token(TokenType::Minus),
            '+' => self.add_token(TokenType::Plus),
            '*' => self.add_token(TokenType::Star),
            '%' => self.add_token(TokenType::Percent),
            '/' => {
                if self.match_char('/') {
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                } else {
                    self.add_token(TokenType::Slash);
                }
            }
            '=' => self.add_conditional_token('=', TokenType::Equal, TokenType::Assign),
            '<' => self.add_conditional_token('=', TokenType::LessEqual, TokenType::LessThan),
            '>' => self.add_conditional_token('=', TokenType::GreaterEqual, TokenType::GreaterThan),
            '!' => self.add_conditional_token('=', TokenType::BangEqual, TokenType::Bang),

            '&' => {
                if self.match_char('&') {
                    self.add_token(TokenType::And);
                } else {
                    return Err(self.error("Diperyogikeun '&' saatos '&'."));
                }
            }
            '|' => {
                if self.match_char('|') {
                    self.add_token(TokenType::Or);
                } else {
                    return Err(self.error("Diperyogikeun '|' saatos '|'."));
                }
            }

            '"' => self.string()?,

            _ => {
                if char.is_ascii_digit() {
                    self.number()?;
                } else if char.is_alphabetic() || char == '_' {
                    self.identifier()
                } else {
                    return Err(self.error(&format!("Karakter teu dipikawanoh: '{}'.", char)));
                }
            }
        }

        Ok(())
    }

    fn string(&mut self) -> ScriptResult<()> {
        let mut value = String::new();
        let start_line = self.line;

        while self.peek() != '"' && !self.is_at_end() {
            let char = self.advance();

            if char == '\n' {
                self.line += 1;
                self.column = 1;
                value.push(char);
            } else if char == '\\' {
                if self.is_at_end() {
                    return Err(self.error("Teks teu ditutup saatos karakter escape."));
                }

                let escape_char = self.advance();
                match escape_char {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    _ => {
                        return Err(self.error(&format!(
                            "Karakter escape '\\{}' teu dipikawanoh.",
                            escape_char
                        )));
                    }
                }
            } else {
                value.push(char);
            }
        }

        if self.is_at_end() {
            return Err(ScriptError::LexerError {
                line: start_line,
                column: self.start_column,
                message: "Kutipan teks teu ditutup ku '\"'.".to_string(),
            });
        }

        self.advance();

        let lexeme: String = self.source[self.start..self.current].iter().collect();
        self.tokens.push(Token::new(
            TokenType::String(value),
            lexeme,
            start_line,
            self.start_column,
        ));

        Ok(())
    }

    fn number(&mut self) -> ScriptResult<()> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance();

            while self.peek().is_ascii_digit() {
                self.advance();
            }
        }

        let number_string: String = self.source[self.start..self.current].iter().collect();

        if number_string.contains('.') {
            let value = number_string.parse::<f64>().map_err(|_| {
                self.error(&format!("Gagal maca angka desimal: {}.", number_string))
            })?;

            self.tokens.push(Token::new(
                TokenType::Float(value),
                number_string,
                self.line,
                self.start_column,
            ));
        } else {
            let value = number_string
                .parse::<i64>()
                .map_err(|_| self.error(&format!("Gagal maca angka buleud: {}.", number_string)))?;

            self.tokens.push(Token::new(
                TokenType::Integer(value),
                number_string,
                self.line,
                self.start_column,
            ));
        }

        Ok(())
    }

    fn identifier(&mut self) {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let text: String = self.source[self.start..self.current].iter().collect();
        let kind = match text.as_str() {
            "citak" => TokenType::Print,
            "kosong" => TokenType::Null,
            "lajeungkeun" => TokenType::Continue,
            "lepat" => TokenType::False,
            "leres" => TokenType::True,
            "liren" => TokenType::Break,
            "ngulang" => TokenType::While,
            "pancen" => TokenType::Function,
            "pikeun" => TokenType::For,
            "sanes" => TokenType::Else,
            "simpen" => TokenType::Variable,
            "upami" => TokenType::If,
            "wangsul" => TokenType::Return,
            _ => TokenType::Identifier,
        };

        self.tokens
            .push(Token::new(kind, text, self.line, self.start_column));
    }

    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    fn advance(&mut self) -> char {
        let char = self.source[self.current];
        self.current += 1;
        self.column += 1;
        char
    }

    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.current]
        }
    }

    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.current + 1]
        }
    }

    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source[self.current] != expected {
            return false;
        }
        self.current += 1;
        self.column += 1;
        true
    }

    fn add_token(&mut self, kind: TokenType) {
        let text: String = self.source[self.start..self.current].iter().collect();
        self.tokens
            .push(Token::new(kind, text, self.line, self.start_column));
    }

    fn add_conditional_token(&mut self, expected: char, if_match: TokenType, if_not: TokenType) {
        let kind = if self.match_char(expected) {
            if_match
        } else {
            if_not
        };
        self.add_token(kind);
    }

    fn error(&self, message: &str) -> ScriptError {
        ScriptError::LexerError {
            line: self.line,
            column: self.start_column,
            message: message.to_string(),
        }
    }
}
