//! `gugolpi radical`.

use anyhow::Context;
use gugolpi_core::config::RadicalOptions;
use gugolpi_core::{Module, RunConfig, benchmark_for};

use crate::cli::{GlobalArgs, RadicalArgs};
use crate::commands::{apply_run_args, execute};
use crate::exit::ExitCode;

/// Ejecuta el módulo Radical con los argumentos dados.
pub fn run(global: &GlobalArgs, args: &RadicalArgs) -> anyhow::Result<ExitCode> {
    let mut config = RunConfig::new(Module::Radical, &args.size)
        .with_context(|| format!("tamaño «{}» no válido para radical", args.size))?;
    apply_run_args(&mut config, &args.run);
    config.radical = RadicalOptions {
        static_split: args.static_split,
    };
    let benchmark = benchmark_for(Module::Radical)?;
    execute(global, benchmark.as_ref(), &config, args.run.repeat)
}
