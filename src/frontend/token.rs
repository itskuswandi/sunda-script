#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Break,
    Continue,
    Else,
    False,
    For,
    Function,
    If,
    Null,
    Print,
    Return,
    True,
    Variable,
    While,

    Identifier,
    Integer(i64),
    Float(f64),
    String(String),

    Assign,
    Equal,
    Bang,
    BangEqual,
    And,
    Or,
    LessThan,
    LessEqual,
    GreaterThan,
    GreaterEqual,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,

    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semicolon,
    Colon,
    Dot,

    Eof,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub kind: TokenType,
    pub lexeme: String,
    pub line: usize,
    pub column: usize,
}

impl Token {
    pub fn new(kind: TokenType, lexeme: String, line: usize, column: usize) -> Self {
        Token {
            kind,
            lexeme,
            line,
            column,
        }
    }
}
