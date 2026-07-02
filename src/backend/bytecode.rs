use std::{cell::RefCell, collections::HashMap, rc::Rc};

use crate::backend::vm::VM;

#[derive(Debug, Clone)]
pub enum Value {
    Integer(i64),
    Float(f64),
    String(Rc<String>),
    Boolean(bool),
    Null,

    Array(Rc<RefCell<Vec<Value>>>),
    Object(Rc<RefCell<HashMap<String, Value>>>),

    Function(Rc<Function>),
    NativeMethod(Box<Value>, fn(&Value, &[Value]) -> Result<Value, String>),
    NativeHOF(
        Box<Value>,
        fn(&mut VM, &Chunk, &Value, &[Value]) -> Result<Value, String>,
    ),
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Integer(l), Value::Integer(r)) => l == r,
            (Value::Float(l), Value::Float(r)) => l == r,
            (Value::String(l), Value::String(r)) => l == r,
            (Value::Boolean(l), Value::Boolean(r)) => l == r,
            (Value::Null, Value::Null) => true,

            (Value::Array(l), Value::Array(r)) => l == r,
            (Value::Object(l), Value::Object(r)) => l == r,

            (Value::Function(l), Value::Function(r)) => l == r,
            (Value::NativeMethod(l_rec, _), Value::NativeMethod(r_rec, _)) => l_rec == r_rec,
            (Value::NativeHOF(l_rec, _), Value::NativeHOF(r_rec, _)) => l_rec == r_rec,

            _ => false,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct Function {
    pub name: String,
    pub address: usize,
    pub arity: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum OperationCode {
    Constant(usize),
    Pop,

    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Negate,
    Not,
    Equal,
    GreaterThan,
    LessThan,

    SetGlobal(usize),
    GetGlobal(usize),
    SetLocal(usize),
    GetLocal(usize),

    BuildArray(usize),
    GetIndex,
    SetIndex,
    BuildObject(usize),
    GetProperty(usize),
    SetProperty(usize),

    JumpIfFalse(usize),
    Jump(usize),

    Call(usize),
    Return,

    Print,
}

#[derive(Debug, Clone)]
pub struct Chunk {
    pub code: Vec<OperationCode>,
    pub constants: Vec<Value>,
    pub lines: Vec<usize>,
}

impl Chunk {
    pub fn new() -> Self {
        Chunk {
            code: Vec::new(),
            constants: Vec::new(),
            lines: Vec::new(),
        }
    }
}
