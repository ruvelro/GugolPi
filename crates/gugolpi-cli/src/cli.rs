//! Definición de argumentos con clap.

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use gugolpi_core::{Mode, Threads};

/// GugolPi: benchmark de CPU con los algoritmos de SuperPi y wPrime, y primos propios.
#[derive(Debug, Parser)]
#[command(name = "gugolpi", version, about, long_about = None, propagate_version = true)]
pub struct Cli {
    /// Opciones comunes a todos los comandos.
    #[command(flatten)]
    pub global: GlobalArgs,

    /// Comando a ejecutar.
    #[command(subcommand)]
    pub command: Command,
}

/// Opciones comunes.
#[derive(Debug, Args)]
pub struct GlobalArgs {
    /// Imprime el resultado en JSON por stdout en vez del resumen legible.
    #[arg(long, global = true)]
    pub json: bool,

    /// No muestra progreso ni resumen (útil con --json o --out).
    #[arg(long, short, global = true)]
    pub quiet: bool,

    /// Directorio donde guardar un fichero JSON por run.
    #[arg(long, global = true, value_name = "DIR")]
    pub out: Option<PathBuf>,
}

/// Subcomandos.
#[derive(Debug, Subcommand)]
pub enum Command {
    /// Raíces cuadradas de 1..N por Newton (compatible con wPrime 32M / 1024M).
    Radical(RadicalArgs),
    /// Ficha del sistema.
    Sysinfo(SysinfoArgs),
}

/// Opciones compartidas por los módulos de benchmark.
#[derive(Debug, Args)]
pub struct RunArgs {
    /// Modo de ejecución.
    #[arg(long, default_value = "single", value_parser = parse_mode)]
    pub mode: Mode,

    /// Hilos en modo multi: `auto` (CPU lógicas), `physical` (núcleos físicos) o un número.
    #[arg(long, default_value = "auto", value_parser = parse_threads)]
    pub threads: Threads,

    /// Fija cada hilo a un núcleo.
    #[arg(long)]
    pub affinity: bool,

    /// Repite el run N veces e informa mejor, media y desviación.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    pub repeat: u32,
}

/// Argumentos de `gugolpi radical`.
#[derive(Debug, Args)]
pub struct RadicalArgs {
    /// Tamaño: `32M`, `1024M` (oficiales), `128M`, `4096M` o un N libre.
    #[arg(long, default_value = "32M")]
    pub size: String,

    /// Reparto estático (rango contiguo por hilo) en vez de la cola dinámica. No puntúa.
    #[arg(long = "static")]
    pub static_split: bool,

    /// Opciones comunes de ejecución.
    #[command(flatten)]
    pub run: RunArgs,
}

/// Argumentos de `gugolpi sysinfo`.
#[derive(Debug, Args)]
pub struct SysinfoArgs {}

fn parse_mode(text: &str) -> Result<Mode, String> {
    text.parse().map_err(|e: gugolpi_core::Error| e.to_string())
}

fn parse_threads(text: &str) -> Result<Threads, String> {
    text.parse().map_err(|e: gugolpi_core::Error| e.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cli_definition_is_consistent() {
        use clap::CommandFactory;
        Cli::command().debug_assert();
    }
}
