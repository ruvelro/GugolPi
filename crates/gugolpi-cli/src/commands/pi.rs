//! `gugolpi pi`.

use anyhow::Context;
use gugolpi_core::config::PiOptions;
use gugolpi_core::{Module, RunConfig};

use crate::cli::{GlobalArgs, PiArgs};
use crate::commands::{apply_run_args, execute};
use crate::exit::ExitCode;

/// Ejecuta el módulo Pi con los argumentos dados.
pub fn run(global: &GlobalArgs, args: &PiArgs) -> anyhow::Result<ExitCode> {
    let mut config = RunConfig::new(Module::Pi, &args.size)
        .with_context(|| format!("tamaño «{}» no válido para pi", args.size))?;
    apply_run_args(&mut config, &args.run);
    config.pi = PiOptions {
        save_digits: args.save_digits.clone(),
    };
    execute(global, config, &args.run)
}
