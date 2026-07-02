use std::{mem::take, rc::Rc};

use crate::{
    backend::bytecode::{Chunk, Function, OperationCode, Value},
    frontend::{
        ast::{Expr, Stmt},
        token::TokenType,
    },
};

struct Local {
    name: String,
    depth: usize,
}

pub struct Compiler {
    chunk: Chunk,
    locals: Vec<Local>,
    scope_depth: usize,
    continue_jumps: Vec<Vec<usize>>,
    break_jumps: Vec<Vec<usize>>,
    loop_local_counts: Vec<usize>,
}

impl Compiler {
    pub fn new() -> Self {
        Compiler {
            chunk: Chunk::new(),
            locals: Vec::new(),
            scope_depth: 0,
            continue_jumps: Vec::new(),
            break_jumps: Vec::new(),
            loop_local_counts: Vec::new(),
        }
    }

    pub fn compile(mut self, statements: Vec<Stmt>) -> Chunk {
        for stmt in statements {
            self.compile_stmt(stmt)
        }
        self.chunk
    }

    fn compile_stmt(&mut self, stmt: Stmt) {
        match stmt {
            Stmt::Expression(expr) => {
                self.compile_expr(expr);
                self.emit_op(OperationCode::Pop, 0);
            }

            Stmt::Print(expr) => {
                self.compile_expr(expr);
                self.emit_op(OperationCode::Print, 0);
            }

            Stmt::Block(statements) => {
                self.scope_depth += 1;

                for stmt in statements {
                    self.compile_stmt(stmt);
                }

                while let Some(local) = self.locals.last() {
                    if local.depth == self.scope_depth {
                        self.emit_op(OperationCode::Pop, 0);
                        self.locals.pop();
                    } else {
                        break;
                    }
                }

                self.scope_depth -= 1;
            }

            Stmt::Variable { name, initializer } => {
                self.compile_expr(initializer);

                let variable_name = name.lexeme;

                if self.scope_depth > 0 {
                    self.locals.push(Local {
                        name: variable_name,
                        depth: self.scope_depth,
                    });
                } else {
                    let global_index = self.identifier_constant(variable_name);
                    self.emit_op(OperationCode::SetGlobal(global_index), name.line);
                    self.emit_op(OperationCode::Pop, name.line);
                }
            }

            Stmt::Function { name, params, body } => {
                let jump_over = self.emit_jump(OperationCode::Jump(0));

                let address = self.chunk.code.len();
                let arity = params.len();
                let function_name = name.lexeme;
                let function_line = name.line;

                let previous_locals = take(&mut self.locals);
                let previous_scope_depth = self.scope_depth;

                self.scope_depth = 1;

                for param in params {
                    self.locals.push(Local {
                        name: param.lexeme,
                        depth: self.scope_depth,
                    });
                }

                for stmt in body {
                    self.compile_stmt(stmt);
                }

                self.chunk.constants.push(Value::Boolean(false));
                self.emit_op(
                    OperationCode::Constant(self.chunk.constants.len() - 1),
                    name.line,
                );
                self.emit_op(OperationCode::Return, name.line);

                self.locals = previous_locals;
                self.scope_depth = previous_scope_depth;

                self.patch_jump(jump_over);

                let func = Value::Function(Rc::new(Function {
                    name: function_name.clone(),
                    address,
                    arity,
                }));

                self.chunk.constants.push(func);
                self.emit_op(
                    OperationCode::Constant(self.chunk.constants.len() - 1),
                    name.line,
                );

                if self.scope_depth > 0 {
                    self.locals.push(Local {
                        name: function_name,
                        depth: self.scope_depth,
                    });
                } else {
                    let global_index = self.identifier_constant(function_name);
                    self.emit_op(OperationCode::SetGlobal(global_index), function_line);
                    self.emit_op(OperationCode::Pop, name.line);
                }
            }

            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.compile_expr(condition);

                let if_jump = self.emit_jump(OperationCode::JumpIfFalse(0));
                self.emit_op(OperationCode::Pop, 0);

                self.compile_stmt(*then_branch);

                let jump_over_else = self.emit_jump(OperationCode::Jump(0));

                self.patch_jump(if_jump);
                self.emit_op(OperationCode::Pop, 0);

                if let Some(else_stmt) = else_branch {
                    self.compile_stmt(*else_stmt);
                }

                self.patch_jump(jump_over_else);
            }

            Stmt::While {
                condition,
                body,
                increment,
            } => {
                let loop_start = self.chunk.code.len();
                let local_count = self.locals.len();

                self.continue_jumps.push(Vec::new());
                self.break_jumps.push(Vec::new());
                self.loop_local_counts.push(local_count);

                self.compile_expr(condition);
                let exit_jump = self.emit_jump(OperationCode::JumpIfFalse(0));
                self.emit_op(OperationCode::Pop, 0);

                self.compile_stmt(*body);

                if let Some(continues) = self.continue_jumps.pop() {
                    for jump in continues {
                        self.patch_jump(jump);
                    }
                }

                if let Some(inc) = increment {
                    self.compile_expr(inc);
                    self.emit_op(OperationCode::Pop, 0);
                }

                self.emit_op(OperationCode::Jump(loop_start), 0);

                self.patch_jump(exit_jump);
                self.emit_op(OperationCode::Pop, 0);

                if let Some(breaks) = self.break_jumps.pop() {
                    for jump in breaks {
                        self.patch_jump(jump);
                    }
                }

                self.loop_local_counts.pop();
            }

            Stmt::Break { keyword } => {
                if let Some(&local_count) = self.loop_local_counts.last() {
                    let locals_to_pop = self.locals.len() - local_count;
                    for _ in 0..locals_to_pop {
                        self.emit_op(OperationCode::Pop, keyword.line);
                    }
                }

                let jump = self.emit_jump(OperationCode::Jump(0));
                if let Some(breaks) = self.break_jumps.last_mut() {
                    breaks.push(jump);
                }
            }

            Stmt::Continue { keyword } => {
                if let Some(&local_count) = self.loop_local_counts.last() {
                    let locals_to_pop = self.locals.len() - local_count;
                    for _ in 0..locals_to_pop {
                        self.emit_op(OperationCode::Pop, keyword.line);
                    }
                }

                let jump = self.emit_jump(OperationCode::Jump(0));
                if let Some(continues) = self.continue_jumps.last_mut() {
                    continues.push(jump);
                }
            }

            Stmt::Return { keyword, value } => {
                if let Some(expr) = value {
                    self.compile_expr(expr);
                } else {
                    self.chunk.constants.push(Value::Boolean(false));
                    self.emit_op(
                        OperationCode::Constant(self.chunk.constants.len() - 1),
                        keyword.line,
                    );
                }

                self.emit_op(OperationCode::Return, keyword.line);
            }
        }
    }

    fn compile_expr(&mut self, expr: Expr) {
        match expr {
            Expr::Integer(value) => {
                self.chunk.constants.push(Value::Integer(value));
                self.emit_op(OperationCode::Constant(self.chunk.constants.len() - 1), 0);
            }
            Expr::Float(value) => {
                self.chunk.constants.push(Value::Float(value));
                self.emit_op(OperationCode::Constant(self.chunk.constants.len() - 1), 0);
            }
            Expr::String(value) => {
                self.chunk.constants.push(Value::String(Rc::new(value)));
                self.emit_op(OperationCode::Constant(self.chunk.constants.len() - 1), 0);
            }
            Expr::Boolean(value) => {
                self.chunk.constants.push(Value::Boolean(value));
                self.emit_op(OperationCode::Constant(self.chunk.constants.len() - 1), 0);
            }
            Expr::Null => {
                self.chunk.constants.push(Value::Null);
                self.emit_op(OperationCode::Constant(self.chunk.constants.len() - 1), 0);
            }

            Expr::Function { params, body } => {
                let jump_over = self.emit_jump(OperationCode::Jump(0));
                let address = self.chunk.code.len();
                let arity = params.len();
                let function_name = "<pancen_teu_aya_nami>".to_string();

                let previous_locals = take(&mut self.locals);
                let previous_scope_depth = self.scope_depth;
                self.scope_depth = 1;

                for param in params {
                    self.locals.push(Local {
                        name: param.lexeme,
                        depth: self.scope_depth,
                    });
                }

                for stmt in body {
                    self.compile_stmt(stmt);
                }

                self.chunk.constants.push(Value::Boolean(false));
                self.emit_op(OperationCode::Constant(self.chunk.constants.len() - 1), 0);
                self.emit_op(OperationCode::Return, 0);

                self.locals = previous_locals;
                self.scope_depth = previous_scope_depth;
                self.patch_jump(jump_over);

                let function = Value::Function(Rc::new(Function {
                    name: function_name,
                    address,
                    arity,
                }));

                self.chunk.constants.push(function);
                self.emit_op(OperationCode::Constant(self.chunk.constants.len() - 1), 0);
            }

            Expr::Variable { name } => {
                let variable_name = name.lexeme;

                if let Some(index) = self.resolve_local(&variable_name) {
                    self.emit_op(OperationCode::GetLocal(index), name.line);
                } else {
                    let global_index = self.identifier_constant(variable_name);
                    self.emit_op(OperationCode::GetGlobal(global_index), name.line);
                }
            }

            Expr::Assign { name, value } => {
                self.compile_expr(*value);
                let variable_name = name.lexeme;

                if let Some(index) = self.resolve_local(&variable_name) {
                    self.emit_op(OperationCode::SetLocal(index), name.line);
                } else {
                    let global_index = self.identifier_constant(variable_name);
                    self.emit_op(OperationCode::SetGlobal(global_index), name.line);
                }
            }

            Expr::Unary { operator, right } => {
                self.compile_expr(*right);

                if operator.kind == TokenType::Minus {
                    self.emit_op(OperationCode::Negate, operator.line);
                } else if operator.kind == TokenType::Bang {
                    self.emit_op(OperationCode::Not, operator.line);
                }
            }

            Expr::Binary {
                left,
                operator,
                right,
            } => {
                self.compile_expr(*left);
                self.compile_expr(*right);

                match operator.kind {
                    TokenType::Plus => self.emit_op(OperationCode::Add, operator.line),
                    TokenType::Minus => self.emit_op(OperationCode::Subtract, operator.line),
                    TokenType::Star => self.emit_op(OperationCode::Multiply, operator.line),
                    TokenType::Slash => self.emit_op(OperationCode::Divide, operator.line),
                    TokenType::Percent => self.emit_op(OperationCode::Modulo, operator.line),
                    TokenType::Equal => self.emit_op(OperationCode::Equal, operator.line),
                    TokenType::BangEqual => {
                        self.emit_op(OperationCode::Equal, operator.line);
                        self.emit_op(OperationCode::Not, operator.line);
                    }
                    TokenType::GreaterThan => {
                        self.emit_op(OperationCode::GreaterThan, operator.line)
                    }
                    TokenType::GreaterEqual => {
                        self.emit_op(OperationCode::LessThan, operator.line);
                        self.emit_op(OperationCode::Not, operator.line);
                    }
                    TokenType::LessThan => self.emit_op(OperationCode::LessThan, operator.line),
                    TokenType::LessEqual => {
                        self.emit_op(OperationCode::GreaterThan, operator.line);
                        self.emit_op(OperationCode::Not, operator.line);
                    }
                    _ => {}
                }
            }

            Expr::Logical {
                left,
                operator,
                right,
            } => {
                self.compile_expr(*left);

                if operator.kind == TokenType::And {
                    let end_jump = self.emit_jump(OperationCode::JumpIfFalse(0));
                    self.emit_op(OperationCode::Pop, operator.line);
                    self.compile_expr(*right);
                    self.patch_jump(end_jump);
                } else if operator.kind == TokenType::Or {
                    let else_jump = self.emit_jump(OperationCode::JumpIfFalse(0));
                    let end_jump = self.emit_jump(OperationCode::Jump(0));

                    self.patch_jump(else_jump);
                    self.emit_op(OperationCode::Pop, operator.line);
                    self.compile_expr(*right);
                    self.patch_jump(end_jump);
                }
            }

            Expr::Call {
                callee,
                arguments,
                paren_line,
            } => {
                self.compile_expr(*callee);

                let arity = arguments.len();
                for arg in arguments {
                    self.compile_expr(arg);
                }

                self.emit_op(OperationCode::Call(arity), paren_line);
            }

            Expr::Array { elements } => {
                let element_count = elements.len();

                for element in elements {
                    self.compile_expr(element);
                }
                self.emit_op(OperationCode::BuildArray(element_count), 0);
            }

            Expr::Index {
                array,
                index,
                bracket_line,
            } => {
                self.compile_expr(*array);
                self.compile_expr(*index);
                self.emit_op(OperationCode::GetIndex, bracket_line);
            }

            Expr::AssignIndex {
                array,
                index,
                value,
            } => {
                self.compile_expr(*array);
                self.compile_expr(*index);
                self.compile_expr(*value);
                self.emit_op(OperationCode::SetIndex, 0);
            }

            Expr::Object { properties } => {
                let properties_count = properties.len();

                for (name_token, value_expr) in properties {
                    let name_index = self.identifier_constant(name_token.lexeme);
                    self.emit_op(OperationCode::Constant(name_index), name_token.line);
                    self.compile_expr(value_expr);
                }

                self.emit_op(OperationCode::BuildObject(properties_count), 0);
            }

            Expr::GetProperty { object, name } => {
                self.compile_expr(*object);
                let name_index = self.identifier_constant(name.lexeme);
                self.emit_op(OperationCode::GetProperty(name_index), name.line);
            }

            Expr::SetProperty {
                object,
                name,
                value,
            } => {
                self.compile_expr(*object);
                self.compile_expr(*value);
                let name_index = self.identifier_constant(name.lexeme);
                self.emit_op(OperationCode::SetProperty(name_index), name.line);
            }
        }
    }

    fn resolve_local(&self, name: &str) -> Option<usize> {
        for (i, local) in self.locals.iter().enumerate().rev() {
            if local.name == name {
                return Some(i);
            }
        }
        None
    }

    fn emit_jump(&mut self, instruction: OperationCode) -> usize {
        let line = if !self.chunk.lines.is_empty() {
            *self.chunk.lines.last().unwrap()
        } else {
            0
        };
        self.emit_op(instruction, line);
        self.chunk.code.len() - 1
    }

    fn patch_jump(&mut self, offset: usize) {
        let target = self.chunk.code.len();

        match self.chunk.code[offset] {
            OperationCode::JumpIfFalse(_) => {
                self.chunk.code[offset] = OperationCode::JumpIfFalse(target)
            }
            OperationCode::Jump(_) => self.chunk.code[offset] = OperationCode::Jump(target),
            _ => unreachable!(),
        }
    }

    fn emit_op(&mut self, op: OperationCode, line: usize) {
        self.chunk.code.push(op);
        self.chunk.lines.push(line);
    }

    fn identifier_constant(&mut self, name: String) -> usize {
        for (i, constant) in self.chunk.constants.iter().enumerate() {
            if let Value::String(existing_name) = constant {
                if **existing_name == name {
                    return i;
                }
            }
        }

        self.chunk.constants.push(Value::String(Rc::new(name)));
        self.chunk.constants.len() - 1
    }
}
