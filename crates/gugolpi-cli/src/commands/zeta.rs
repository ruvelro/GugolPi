//! `gugolpi zeta`.

use anyhow::Context;
use gugolpi_core::{Module, RunConfig};

use crate::cli::{GlobalArgs, ZetaArgs};
use crate::commands::{apply_run_args, execute};
use crate::exit::ExitCode;

/// Ejecuta el módulo Zeta con los argumentos dados.
pub fn run(global: &GlobalArgs, args: &ZetaArgs) -> anyhow::Result<ExitCode> {
    let mut config = RunConfig::new(Module::Zeta, &args.size)
        .with_context(|| format!("tamaño «{}» no válido para zeta", args.size))?;
    apply_run_args(&mut config, &args.run);
    execute(global, config, &args.run)
}
