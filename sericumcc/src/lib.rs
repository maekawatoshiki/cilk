extern crate id_arena;
extern crate rustc_hash;
extern crate sericum;

use std::path::PathBuf;
use std::{
    fs,
    io::{BufWriter, Write},
    process,
};
use {rand, rand::Rng};

pub mod ast;
pub mod codegen;
pub mod lexer;
pub mod parser;
pub mod token;
pub mod types;

// TODO: Refine code

pub fn compile(path: PathBuf) {
    let mut lexer = lexer::Lexer::new(path);
    let mut parser = parser::Parser::new(&mut lexer);
    let nodes = match parser.parse() {
        Ok(ok) => ok,
        Err(lexer::Error::EOF) => panic!("unexpected EOF"),
        Err(lexer::Error::Message(loc, msg)) => {
            println!(
                "{}:{}: {}",
                lexer.path_arena().borrow()[loc.file]
                    .as_path()
                    .display()
                    .to_string(),
                loc.line,
                msg
            );
            println!("{}", lexer.get_surrounding_line(loc));
            panic!();
        }
    };

    println!("{:#?}", nodes);

    let mut codegen = codegen::Codegenerator::new(&mut parser.compound_types);
    for node in nodes {
        if let Err(codegen::Error::Message(loc, msg)) = codegen.generate(&node) {
            println!(
                "{}:{}: {}",
                parser.lexer.path_arena().borrow()[loc.file]
                    .as_path()
                    .display()
                    .to_string(),
                loc.line,
                msg
            );
            println!("{}", parser.lexer.get_surrounding_line(loc));
            panic!();
        }
    }
    println!("{:?}", codegen.module);

    // sericum::ir::mem2reg::Mem2Reg::new().run_on_module(&mut codegen.module);
    // sericum::ir::cse::CommonSubexprElimination::new().run_on_module(&mut codegen.module);
    // sericum::ir::licm::LoopInvariantCodeMotion::new().run_on_module(&mut codegen.module);

    let machine_module =
        sericum::codegen::x64::standard_conversion_into_machine_module(codegen.module);
    let mut printer = sericum::codegen::x64::asm::print::MachineAsmPrinter::new();
    printer.run_on_module(&machine_module);
    println!("{}", printer.output);

    assemble_and_run(&printer.output);
}

fn unique_file_name(extension: &str) -> String {
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    const LEN: usize = 16;
    let mut rng = rand::thread_rng();
    let name: String = (0..LEN)
        .map(|_| {
            let idx = rng.gen_range(0, CHARSET.len());
            CHARSET[idx] as char
        })
        .collect();
    format!("/tmp/{}.{}", name, extension)
}

fn assemble_and_run(s_target: &str) {
    let target_name = unique_file_name("s");
    {
        let mut target = BufWriter::new(fs::File::create(target_name.as_str()).unwrap());
        target.write_all(s_target.as_bytes()).unwrap();
    }

    let output_name = unique_file_name("out");
    let compilation = process::Command::new("clang")
        .args(&[target_name.as_str(), "-o", output_name.as_str()])
        .status()
        .unwrap();
    assert!(compilation.success());

    let execution = process::Command::new(output_name.as_str())
        .status()
        .unwrap();
    if let Some(code) = execution.code() {
        println!("Exit code: {:?}", code);
        assert!(code == 0);
    } else {
        assert!(execution.success());
    }

    fs::remove_file(output_name).unwrap();
    fs::remove_file(target_name).unwrap();
}
