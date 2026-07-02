use crate::frontend::token::Token;

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Null,

    Function {
        params: Vec<Token>,
        body: Vec<Stmt>,
    },

    Variable {
        name: Token,
    },
    Assign {
        name: Token,
        value: Box<Expr>,
    },

    Unary {
        operator: Token,
        right: Box<Expr>,
    },
    Binary {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },
    Logical {
        left: Box<Expr>,
        operator: Token,
        right: Box<Expr>,
    },

    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
        paren_line: usize,
    },

    Array {
        elements: Vec<Expr>,
    },
    Index {
        array: Box<Expr>,
        index: Box<Expr>,
        bracket_line: usize,
    },
    AssignIndex {
        array: Box<Expr>,
        index: Box<Expr>,
        value: Box<Expr>,
    },

    Object {
        properties: Vec<(Token, Expr)>,
    },
    GetProperty {
        object: Box<Expr>,
        name: Token,
    },
    SetProperty {
        object: Box<Expr>,
        name: Token,
        value: Box<Expr>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    Expression(Expr),
    Print(Expr),
    Block(Vec<Stmt>),

    Variable {
        name: Token,
        initializer: Expr,
    },
    Function {
        name: Token,
        params: Vec<Token>,
        body: Vec<Stmt>,
    },

    If {
        condition: Expr,
        then_branch: Box<Stmt>,
        else_branch: Option<Box<Stmt>>,
    },
    While {
        condition: Expr,
        body: Box<Stmt>,
        increment: Option<Expr>,
    },

    Break {
        keyword: Token,
    },
    Continue {
        keyword: Token,
    },
    Return {
        keyword: Token,
        value: Option<Expr>,
    },
}
