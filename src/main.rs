mod backend;
mod error;
mod frontend;

use std::{env::args, fs::read_to_string, process::exit, time::Instant};

use crate::{
    backend::{compiler::Compiler, vm::VM},
    error::ScriptResult,
    frontend::{analyzer::Analyzer, lexer::Lexer, parser::Parser},
};

fn main() {
    let args: Vec<String> = args().collect();

    if args.len() < 2 {
        eprintln!("Cara ngagunakeun: cargo run <nami_file>.sd");
        exit(1)
    }

    let file_path = &args[1];

    let source_code = match read_to_string(file_path) {
        Ok(code) => code,
        Err(err) => {
            eprintln!(
                "[\x1b[31mLepat Sistem\x1b[0m] Hapunten: Teu tiasa maca file '{}' ({}).",
                file_path, err
            );
            exit(1)
        }
    };

    let start_time = Instant::now();
    let result = run(&source_code);
    let duration = start_time.elapsed();

    match result {
        Ok(_) => {
            println!(
                "\n\x1b[32m✓ Ngajalankeun\x1b[0m '{}' \x1b[32mrengse\x1b[0m dina \x1b[36m{:.2} detik\x1b[0m",
                file_path,
                duration.as_secs_f64()
            );
        }
        Err(err) => {
            eprintln!("{}", err);
            eprintln!(
                "\n\x1b[31m✗ Ngajalankeun\x1b[0m '{}' \x1b[31mgagal\x1b[0m dina \x1b[36m{:.2} detik\x1b[0m",
                file_path,
                duration.as_secs_f64()
            );
            exit(1)
        }
    }
}

fn run(source_code: &str) -> ScriptResult<()> {
    let lexer = Lexer::new(source_code);
    let tokens = lexer.tokenize()?;

    let mut parser = Parser::new(tokens);
    let ast = parser.parse()?;

    let mut analyzer = Analyzer::new();
    analyzer.analyze(&ast)?;

    let compiler = Compiler::new();
    let chunk = compiler.compile(ast);

    let mut vm = VM::new();
    vm.run(&chunk)?;

    Ok(())
}
