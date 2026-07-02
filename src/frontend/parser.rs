use std::mem::discriminant;

use crate::{
    error::{ScriptError, ScriptResult},
    frontend::{
        ast::{Expr, Stmt},
        token::{
            Token,
            TokenType::{self},
        },
    },
};

#[derive(Debug, Clone, PartialEq, PartialOrd)]
enum Precedence {
    None = 0,
    Assignment,
    Or,
    And,
    Equality,
    Comparison,
    Term,
    Factor,
    Unary,
    Call,
    Primary,
}

impl Precedence {
    fn next(&self) -> Precedence {
        match self {
            Precedence::None => Precedence::Assignment,
            Precedence::Assignment => Precedence::Or,
            Precedence::Or => Precedence::And,
            Precedence::And => Precedence::Equality,
            Precedence::Equality => Precedence::Comparison,
            Precedence::Comparison => Precedence::Term,
            Precedence::Term => Precedence::Factor,
            Precedence::Factor => Precedence::Unary,
            Precedence::Unary => Precedence::Call,
            Precedence::Call => Precedence::Primary,
            Precedence::Primary => Precedence::Primary,
        }
    }
}

pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Parser { tokens, current: 0 }
    }

    pub fn parse(&mut self) -> ScriptResult<Vec<Stmt>> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        Ok(statements)
    }

    fn declaration(&mut self) -> ScriptResult<Stmt> {
        if self.match_token(&[TokenType::Variable]) {
            self.variable_declaration()
        } else if self.match_token(&[TokenType::Function]) {
            self.function_declaration()
        } else {
            self.statement()
        }
    }

    fn statement(&mut self) -> ScriptResult<Stmt> {
        if self.match_token(&[TokenType::For]) {
            self.for_statement()
        } else if self.match_token(&[TokenType::Print]) {
            self.print_statement()
        } else if self.match_token(&[TokenType::If]) {
            self.if_statement()
        } else if self.match_token(&[TokenType::While]) {
            self.while_statement()
        } else if self.match_token(&[TokenType::Break]) {
            self.break_statement()
        } else if self.match_token(&[TokenType::Continue]) {
            self.continue_statement()
        } else if self.match_token(&[TokenType::Return]) {
            self.return_statement()
        } else if self.match_token(&[TokenType::LBrace]) {
            Ok(Stmt::Block(self.block()?))
        } else {
            self.expression_statement()
        }
    }

    fn expression_statement(&mut self) -> ScriptResult<Stmt> {
        let expression = self.expression()?;
        self.consume(TokenType::Semicolon, "Diperyogikeun ';' saatos ekspresi.")?;
        Ok(Stmt::Expression(expression))
    }

    fn print_statement(&mut self) -> ScriptResult<Stmt> {
        self.consume(TokenType::LParen, "Diperyogikeun '(' saatos 'citak'.")?;
        let value = self.expression()?;
        self.consume(TokenType::RParen, "Diperyogikeun ')' saatos nilai 'citak'.")?;
        self.consume(
            TokenType::Semicolon,
            "Diperyogikeun ';' saatos parentah 'citak'.",
        )?;

        Ok(Stmt::Print(value))
    }

    fn block(&mut self) -> ScriptResult<Vec<Stmt>> {
        let mut statements = Vec::new();

        while !self.check(TokenType::RBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.consume(TokenType::RBrace, "Diperyogikeun '}' saatos blok kode.")?;

        Ok(statements)
    }

    fn variable_declaration(&mut self) -> ScriptResult<Stmt> {
        let name = self.consume_identifier("Diperyogikeun nami variabel saatos 'simpen'.")?;
        self.consume(TokenType::Assign, "Diperyogikeun '=' saatos nami variabel.")?;

        let initializer = self.expression()?;
        self.consume(
            TokenType::Semicolon,
            "Diperyogikeun ';' saatos deklarasi variabel.",
        )?;

        Ok(Stmt::Variable { name, initializer })
    }

    fn function_declaration(&mut self) -> ScriptResult<Stmt> {
        let name = self.consume_identifier("Diperyogikeun nami pancen saatos 'pancen'.")?;
        self.consume(TokenType::LParen, "Diperyogikeun '(' saatos nami pancen.")?;

        let mut params = Vec::new();
        if !self.check(TokenType::RParen) {
            loop {
                params.push(self.consume_identifier("Diperyogikeun parameter pancen.")?);
                if !self.match_token(&[TokenType::Comma]) {
                    break;
                }
            }
        }

        self.consume(TokenType::RParen, "Diperyogikeun ')' saatos parameter.")?;
        self.consume(
            TokenType::LBrace,
            "Diperyogikeun '{' sateuacan eusi pancen.",
        )?;

        let body = self.block()?;
        Ok(Stmt::Function { name, params, body })
    }

    fn if_statement(&mut self) -> ScriptResult<Stmt> {
        self.consume(TokenType::LParen, "Diperyogikeun '(' saatos 'upami'.")?;
        let condition = self.expression()?;
        self.consume(
            TokenType::RParen,
            "Diperyogikeun ')' saatos kondisi 'upami'.",
        )?;
        self.consume(
            TokenType::LBrace,
            "Diperyogikeun '{' sateuacan blok 'upami'.",
        )?;

        let then_branch = Box::new(Stmt::Block(self.block()?));
        let mut else_branch = None;

        if self.match_token(&[TokenType::Else]) {
            if self.match_token(&[TokenType::If]) {
                else_branch = Some(Box::new(self.if_statement()?));
            } else {
                self.consume(TokenType::LBrace, "Diperyogikeun '{' saatos 'sanes'.")?;

                else_branch = Some(Box::new(Stmt::Block(self.block()?)));
            }
        }

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
        })
    }

    fn while_statement(&mut self) -> ScriptResult<Stmt> {
        self.consume(TokenType::LParen, "Diperyogikeun '(' saatos 'ngulang'.")?;
        let condition = self.expression()?;
        self.consume(
            TokenType::RParen,
            "Diperyogikeun ')' saatos kondisi 'ngulang'.",
        )?;
        self.consume(
            TokenType::LBrace,
            "Diperyogikeun '{' sateuacan blok 'ngulang'.",
        )?;

        let body = Box::new(Stmt::Block(self.block()?));

        Ok(Stmt::While {
            condition,
            body,
            increment: None,
        })
    }

    fn for_statement(&mut self) -> ScriptResult<Stmt> {
        self.consume(TokenType::LParen, "Diperyogikeun '(' saatos 'pikeun'.")?;

        let initializer;
        if self.match_token(&[TokenType::Semicolon]) {
            initializer = None;
        } else if self.match_token(&[TokenType::Variable]) {
            initializer = Some(self.variable_declaration()?);
        } else {
            initializer = Some(self.expression_statement()?);
        }

        let condition = if !self.check(TokenType::Semicolon) {
            self.expression()?
        } else {
            Expr::Boolean(true)
        };
        self.consume(
            TokenType::Semicolon,
            "Diperyogikeun ';' saatos kondisi 'pikeun'.",
        )?;

        let increment = if !self.check(TokenType::RParen) {
            Some(self.expression()?)
        } else {
            None
        };
        self.consume(
            TokenType::RParen,
            "Diperyogikeun ')' saatos klausul 'pikeun'.",
        )?;

        let body = self.statement()?;

        let mut loop_stmt = Stmt::While {
            condition,
            body: Box::new(body),
            increment,
        };

        if let Some(init) = initializer {
            loop_stmt = Stmt::Block(vec![init, loop_stmt])
        }

        Ok(loop_stmt)
    }

    fn break_statement(&mut self) -> ScriptResult<Stmt> {
        let keyword = self.previous().clone();
        self.consume(TokenType::Semicolon, "Diperyogikeun ';' saatos 'liren'.")?;
        Ok(Stmt::Break { keyword })
    }

    fn continue_statement(&mut self) -> ScriptResult<Stmt> {
        let keyword = self.previous().clone();
        self.consume(
            TokenType::Semicolon,
            "Diperyogikeun ';' saatos 'lajeungkeun'.",
        )?;
        Ok(Stmt::Continue { keyword })
    }

    fn return_statement(&mut self) -> ScriptResult<Stmt> {
        let keyword = self.previous().clone();
        let value = if !self.check(TokenType::Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(
            TokenType::Semicolon,
            "Diperyogikeun ';' saatos parentah 'wangsul'.",
        )?;

        Ok(Stmt::Return { keyword, value })
    }

    fn expression(&mut self) -> ScriptResult<Expr> {
        self.parse_precedence(Precedence::Assignment)
    }

    fn parse_precedence(&mut self, precedence: Precedence) -> ScriptResult<Expr> {
        let mut left = self.parse_prefix()?;

        while precedence <= self.get_precedence(&self.peek().kind) {
            let operator = self.advance().clone();
            left = self.parse_infix(left, operator)?;
        }

        Ok(left)
    }

    fn get_precedence(&self, kind: &TokenType) -> Precedence {
        match kind {
            TokenType::Assign => Precedence::Assignment,
            TokenType::Or => Precedence::Or,
            TokenType::And => Precedence::And,
            TokenType::Equal | TokenType::BangEqual => Precedence::Equality,
            TokenType::LessThan
            | TokenType::LessEqual
            | TokenType::GreaterThan
            | TokenType::GreaterEqual => Precedence::Comparison,
            TokenType::Plus | TokenType::Minus => Precedence::Term,
            TokenType::Star | TokenType::Slash | TokenType::Percent => Precedence::Factor,
            TokenType::LParen | TokenType::LBracket | TokenType::Dot => Precedence::Call,
            _ => Precedence::None,
        }
    }

    fn parse_prefix(&mut self) -> ScriptResult<Expr> {
        let token = self.advance().clone();

        match token.kind {
            TokenType::Integer(value) => Ok(Expr::Integer(value)),
            TokenType::Float(value) => Ok(Expr::Float(value)),
            TokenType::String(value) => Ok(Expr::String(value)),
            TokenType::True => Ok(Expr::Boolean(true)),
            TokenType::False => Ok(Expr::Boolean(false)),
            TokenType::Null => Ok(Expr::Null),

            TokenType::Identifier => Ok(Expr::Variable { name: token }),

            TokenType::Function => {
                self.consume(TokenType::LParen, "Diperyogikeun '(' saatos 'pancen'.")?;

                let mut params = Vec::new();
                if !self.check(TokenType::RParen) {
                    loop {
                        params.push(self.consume_identifier("Diperyogikeun parameter pancen.")?);
                        if !self.match_token(&[TokenType::Comma]) {
                            break;
                        }
                    }
                }

                self.consume(TokenType::RParen, "Diperyogikeun ')' saatos parameter.")?;
                self.consume(
                    TokenType::LBrace,
                    "Diperyogikeun '{' sateuacan eusi pancen.",
                )?;
                let body = self.block()?;

                Ok(Expr::Function { params, body })
            }

            TokenType::LParen => {
                let expression = self.expression()?;
                self.consume(TokenType::RParen, "Diperyogikeun ')' saatos ekspresi.")?;
                Ok(expression)
            }
            TokenType::LBracket => {
                let mut elements = Vec::new();
                if !self.check(TokenType::RBracket) {
                    loop {
                        elements.push(self.expression()?);

                        if !self.match_token(&[TokenType::Comma]) {
                            break;
                        }
                    }
                }

                self.consume(
                    TokenType::RBracket,
                    "Diperyogikeun ']' saatos elemen daptar.",
                )?;

                Ok(Expr::Array { elements })
            }
            TokenType::LBrace => {
                let mut properties = Vec::new();
                if !self.check(TokenType::RBrace) {
                    loop {
                        let name = self.consume_identifier("Diperyogikeun nami properti objek.")?;
                        self.consume(TokenType::Colon, "Diperyogikeun ':' saatos nami properti.")?;
                        let value = self.expression()?;

                        properties.push((name, value));

                        if !self.match_token(&[TokenType::Comma]) {
                            break;
                        }
                    }
                }

                self.consume(
                    TokenType::RBrace,
                    "Diperyogikeun '}' saatos properti objek.",
                )?;

                Ok(Expr::Object { properties })
            }

            TokenType::Minus | TokenType::Bang => {
                let right = self.parse_precedence(Precedence::Unary)?;
                Ok(Expr::Unary {
                    operator: token,
                    right: Box::new(right),
                })
            }

            _ => Err(self.error(&token, "Ekspresi teu sah atanapi teu dipikawanoh.")),
        }
    }

    fn parse_infix(&mut self, left: Expr, operator: Token) -> ScriptResult<Expr> {
        match operator.kind {
            TokenType::Plus
            | TokenType::Minus
            | TokenType::Star
            | TokenType::Slash
            | TokenType::Percent
            | TokenType::Equal
            | TokenType::BangEqual
            | TokenType::LessThan
            | TokenType::LessEqual
            | TokenType::GreaterThan
            | TokenType::GreaterEqual => {
                let rule_precedence = self.get_precedence(&operator.kind);
                let right = self.parse_precedence(rule_precedence.next())?;

                Ok(Expr::Binary {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                })
            }

            TokenType::Or | TokenType::And => {
                let rule_precedence = self.get_precedence(&operator.kind);
                let right = self.parse_precedence(rule_precedence.next())?;

                Ok(Expr::Logical {
                    left: Box::new(left),
                    operator,
                    right: Box::new(right),
                })
            }

            TokenType::Assign => {
                let value = self.parse_precedence(Precedence::Assignment)?;
                match left {
                    Expr::Variable { name } => Ok(Expr::Assign {
                        name,
                        value: Box::new(value),
                    }),
                    Expr::Index { array, index, .. } => Ok(Expr::AssignIndex {
                        array,
                        index,
                        value: Box::new(value),
                    }),
                    Expr::GetProperty { object, name } => Ok(Expr::SetProperty {
                        object,
                        name,
                        value: Box::new(value),
                    }),
                    _ => Err(self.error(&operator, "Target panugasan teu sah.")),
                }
            }

            TokenType::LParen => {
                let mut arguments = Vec::new();
                if !self.check(TokenType::RParen) {
                    loop {
                        arguments.push(self.expression()?);

                        if !self.match_token(&[TokenType::Comma]) {
                            break;
                        }
                    }
                }

                let parent = self
                    .consume(
                        TokenType::RParen,
                        "Diperyogikeun ')' saatos argumen pancen.",
                    )?
                    .clone();

                Ok(Expr::Call {
                    callee: Box::new(left),
                    arguments,
                    paren_line: parent.line,
                })
            }

            TokenType::LBracket => {
                let index = self.expression()?;
                let bracket = self
                    .consume(
                        TokenType::RBracket,
                        "Diperyogikeun ']' saatos indeks daptar.",
                    )?
                    .clone();

                Ok(Expr::Index {
                    array: Box::new(left),
                    index: Box::new(index),
                    bracket_line: bracket.line,
                })
            }

            TokenType::Dot => {
                let name = self.consume_identifier("Diperyogikeun nami properti saatos '.'.")?;
                Ok(Expr::GetProperty {
                    object: Box::new(left),
                    name,
                })
            }

            _ => unreachable!(),
        }
    }

    fn match_token(&mut self, types: &[TokenType]) -> bool {
        for t in types {
            if discriminant(&self.peek().kind) == discriminant(t) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn consume(&mut self, expected: TokenType, message: &str) -> ScriptResult<&Token> {
        if discriminant(&self.peek().kind) == discriminant(&expected) {
            Ok(self.advance())
        } else {
            let token = self.peek().clone();
            Err(self.error(&token, message))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> ScriptResult<Token> {
        if let TokenType::Identifier = self.peek().kind {
            Ok(self.advance().clone())
        } else {
            let token = self.peek().clone();
            Err(self.error(&token, message))
        }
    }

    fn check(&self, expected: TokenType) -> bool {
        if self.is_at_end() {
            return false;
        }
        discriminant(&self.peek().kind) == discriminant(&expected)
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        self.peek().kind == TokenType::Eof
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn error(&self, token: &Token, message: &str) -> ScriptError {
        ScriptError::ParseError {
            line: token.line,
            column: token.column,
            message: message.to_string(),
        }
    }
}
