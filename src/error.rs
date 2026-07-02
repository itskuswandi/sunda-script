use std::fmt;

#[derive(Debug, Clone, PartialEq)]
pub enum ScriptError {
    LexerError {
        line: usize,
        column: usize,
        message: String,
    },

    ParseError {
        line: usize,
        column: usize,
        message: String,
    },

    SemanticError {
        line: usize,
        column: usize,
        message: String,
    },

    RuntimeError {
        line: usize,
        message: String,
    },
}

impl fmt::Display for ScriptError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ScriptError::LexerError {
                line,
                column,
                message,
            } => {
                write!(
                    f,
                    "[\x1b[31mLepat Maca\x1b[0m | Baris {}, Kolom {}] Hapunten: {}",
                    line, column, message
                )
            }

            ScriptError::ParseError {
                line,
                column,
                message,
            } => {
                write!(
                    f,
                    "[\x1b[31mLepat Sintaks\x1b[0m | Baris {}, Kolom {}] Hapunten: {}",
                    line, column, message
                )
            }

            ScriptError::SemanticError {
                line,
                column,
                message,
            } => {
                write!(
                    f,
                    "[\x1b[31mLepat Semantik\x1b[0m | Baris {}, Kolom {}] Hapunten: {}",
                    line, column, message
                )
            }

            ScriptError::RuntimeError { line, message } => {
                write!(
                    f,
                    "[\x1b[31mLepat Eksekusi\x1b[0m | Baris {}] Hapunten: {}",
                    line, message
                )
            }
        }
    }
}

impl std::error::Error for ScriptError {}

pub type ScriptResult<T> = std::result::Result<T, ScriptError>;
