use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::process::ExitCode;

mod table;

fn main() -> ExitCode {
    let mut lenient = false;
    let mut path: Option<String> = None;

    for arg in env::args().skip(1) {
        match arg.as_str() {
            "--lenient" => lenient = true,
            "-h" | "--help" => {
                print_usage();
                return ExitCode::SUCCESS;
            }
            _ if path.is_none() => path = Some(arg),
            other => {
                eprintln!("mdtfmt: unexpected argument: {other}");
                print_usage();
                return ExitCode::FAILURE;
            }
        }
    }

    let input = match read_input(path.as_deref()) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("mdtfmt: {e}");
            return ExitCode::FAILURE;
        }
    };

    match table::format_document(&input, lenient) {
        Ok(output) => {
            print!("{output}");
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("mdtfmt: {e}");
            ExitCode::FAILURE
        }
    }
}

fn read_input(path: Option<&str>) -> io::Result<String> {
    match path {
        Some("-") | None => {
            let mut buf = String::new();
            io::stdin().read_to_string(&mut buf)?;
            Ok(buf)
        }
        Some(p) => fs::read_to_string(p),
    }
}

fn print_usage() {
    let _ = writeln!(
        io::stderr(),
        "usage: mdtfmt [--lenient] [FILE]\n\n\
         Normalizes markdown tables: aligned columns, consistent pipes.\n\
         Reads FILE, or stdin if FILE is omitted or '-'.\n\n\
         By default, a table with a ragged row (wrong column count) is a\n\
         hard error. Pass --lenient to pad short rows and truncate long\n\
         ones instead of failing."
    );
}
