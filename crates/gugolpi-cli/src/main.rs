//! `gugolpi`: la CLI de GugolPi.
//!
//! Capa fina sobre `gugolpi-core`: interpreta argumentos, muestra progreso y escribe
//! resultados. No contiene lógica de cálculo ni de medición.

#![allow(clippy::print_stdout, clippy::print_stderr)]

mod cli;
mod commands;
mod exit;
mod output;

use clap::Parser;

use crate::cli::{Cli, Command};
use crate::exit::ExitCode;

fn main() {
    let cli = match Cli::try_parse() {
        Ok(cli) => cli,
        Err(err) => {
            // clap imprime ayuda y versión por stdout con código 0; los errores de uso van a
            // stderr con nuestro código 1.
            let is_error = err.use_stderr();
            let _ = err.print();
            std::process::exit(if is_error {
                ExitCode::Usage.code()
            } else {
                ExitCode::Ok.code()
            });
        }
    };

    let outcome = match &cli.command {
        Command::Radical(args) => commands::radical::run(&cli.global, args),
        Command::Sysinfo(args) => commands::sysinfo::run(&cli.global, args),
    };

    match outcome {
        Ok(code) => std::process::exit(code.code()),
        Err(err) => {
            eprintln!("error: {err:#}");
            std::process::exit(ExitCode::from_error(&err).code());
        }
    }
}
