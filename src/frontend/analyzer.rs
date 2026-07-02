use std::collections::HashMap;

use crate::{
    error::{ScriptError, ScriptResult},
    frontend::{
        ast::{Expr, Stmt},
        token::Token,
    },
};

#[derive(Clone, Copy, PartialEq)]
enum FunctionType {
    None,
    Function,
}

pub struct Analyzer {
    scopes: Vec<HashMap<String, bool>>,
    current_function: FunctionType,
    loop_depth: usize,
}

impl Analyzer {
    pub fn new() -> Self {
        Analyzer {
            scopes: Vec::new(),
            current_function: FunctionType::None,
            loop_depth: 0,
        }
    }

    pub fn analyze(&mut self, statements: &[Stmt]) -> ScriptResult<()> {
        for stmt in statements {
            self.analyze_stmt(stmt)?;
        }

        Ok(())
    }

    fn analyze_stmt(&mut self, stmt: &Stmt) -> ScriptResult<()> {
        match stmt {
            Stmt::Expression(expr) => {
                self.analyze_expr(expr)?;
            }

            Stmt::Print(expr) => {
                self.analyze_expr(expr)?;
            }

            Stmt::Block(statements) => {
                self.begin_scope();
                self.analyze(statements)?;
                self.end_scope();
            }

            Stmt::Variable { name, initializer } => {
                self.declare(name)?;
                self.analyze_expr(initializer)?;
                self.define(name);
            }

            Stmt::Function { name, params, body } => {
                self.declare(name)?;
                self.define(name);
                self.analyze_function(params, body, FunctionType::Function)?;
            }

            Stmt::If {
                condition,
                then_branch,
                else_branch,
            } => {
                self.analyze_expr(condition)?;
                self.analyze_stmt(then_branch)?;

                if let Some(else_stmt) = else_branch {
                    self.analyze_stmt(else_stmt)?;
                }
            }

            Stmt::While {
                condition,
                body,
                increment,
            } => {
                self.analyze_expr(condition)?;

                self.loop_depth += 1;
                self.analyze_stmt(body)?;

                if let Some(increment) = increment {
                    self.analyze_expr(increment)?;
                }
                self.loop_depth -= 1;
            }

            Stmt::Break { keyword } => {
                if self.loop_depth == 0 {
                    return Err(self.error(keyword, "Teu tiasa nganggo 'liren' di luar ngulang."));
                }
            }

            Stmt::Continue { keyword } => {
                if self.loop_depth == 0 {
                    return Err(
                        self.error(keyword, "Teu tiasa nganggo 'lajeungkeun' di luar ngulang.")
                    );
                }
            }

            Stmt::Return { keyword, value } => {
                if self.current_function == FunctionType::None {
                    return Err(self.error(keyword, "Teu tiasa nganggo 'wangsul' di luar pancen."));
                }

                if let Some(expr) = value {
                    self.analyze_expr(expr)?;
                }
            }
        }

        Ok(())
    }

    fn analyze_expr(&mut self, expr: &Expr) -> ScriptResult<()> {
        match expr {
            Expr::Integer(_) | Expr::Float(_) | Expr::String(_) | Expr::Boolean(_) | Expr::Null => {
            }

            Expr::Function { params, body } => {
                self.analyze_function(params, body, FunctionType::Function)?;
            }

            Expr::Variable { name } => {
                self.resolve_local(name)?;
            }

            Expr::Assign { name, value } => {
                self.analyze_expr(value)?;
                self.resolve_local(name)?;
            }

            Expr::Unary { right, .. } => {
                self.analyze_expr(right)?;
            }

            Expr::Binary { left, right, .. } => {
                self.analyze_expr(left)?;
                self.analyze_expr(right)?;
            }

            Expr::Logical { left, right, .. } => {
                self.analyze_expr(left)?;
                self.analyze_expr(right)?;
            }

            Expr::Call {
                callee, arguments, ..
            } => {
                self.analyze_expr(callee)?;

                for arg in arguments {
                    self.analyze_expr(arg)?;
                }
            }

            Expr::Array { elements } => {
                for element in elements {
                    self.analyze_expr(element)?;
                }
            }

            Expr::Index { array, index, .. } => {
                self.analyze_expr(array)?;
                self.analyze_expr(index)?;
            }

            Expr::AssignIndex {
                array,
                index,
                value,
            } => {
                self.analyze_expr(array)?;
                self.analyze_expr(index)?;
                self.analyze_expr(value)?;
            }

            Expr::Object { properties } => {
                for (_, value) in properties {
                    self.analyze_expr(value)?;
                }
            }

            Expr::GetProperty { object, .. } => {
                self.analyze_expr(object)?;
            }

            Expr::SetProperty { object, value, .. } => {
                self.analyze_expr(object)?;
                self.analyze_expr(value)?;
            }
        }

        Ok(())
    }

    fn analyze_function(
        &mut self,
        params: &[Token],
        body: &[Stmt],
        func_type: FunctionType,
    ) -> ScriptResult<()> {
        let enclosing_function = self.current_function;
        let enclosing_loop_depth = self.loop_depth;

        self.current_function = func_type;
        self.loop_depth = 0;
        self.begin_scope();

        for param in params {
            self.declare(param)?;
            self.define(param);
        }

        self.analyze(body)?;

        self.end_scope();
        self.current_function = enclosing_function;
        self.loop_depth = enclosing_loop_depth;

        Ok(())
    }

    fn resolve_local(&self, name: &Token) -> ScriptResult<()> {
        for scope in self.scopes.iter().rev() {
            if let Some(&is_ready) = scope.get(&name.lexeme) {
                if !is_ready {
                    return Err(self.error(
                        name,
                        &format!(
                            "Teu tiasa maca variabel '{}' dina inisialisasina nyalira.",
                            name.lexeme
                        ),
                    ));
                }
                return Ok(());
            }
        }

        Ok(())
    }

    fn begin_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn end_scope(&mut self) {
        self.scopes.pop();
    }

    fn declare(&mut self, name: &Token) -> ScriptResult<()> {
        if let Some(scope) = self.scopes.last_mut() {
            if scope.contains_key(&name.lexeme) {
                return Err(self.error(
                    name,
                    &format!("Variabel '{}' parantos didamel dina blok ieu.", name.lexeme),
                ));
            }
            scope.insert(name.lexeme.clone(), false);
        }

        Ok(())
    }

    fn define(&mut self, name: &Token) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name.lexeme.clone(), true);
        }
    }

    fn error(&self, token: &Token, message: &str) -> ScriptError {
        ScriptError::SemanticError {
            line: token.line,
            column: token.column,
            message: message.to_string(),
        }
    }
}
