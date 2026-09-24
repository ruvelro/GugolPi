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
    /// Dígitos de Pi por Gauss–Legendre (compatible con SuperPi 16K–32M).
    Pi(PiArgs),
    /// Raíces cuadradas de 1..N por Newton (compatible con wPrime 32M / 1024M).
    Radical(RadicalArgs),
    /// Primos hasta N por criba segmentada (propio de GugolPi: 100M, 1G, 10G).
    Zeta(ZetaArgs),
    /// Ejecuta una suite: un preset (classic, trio, full) o un fichero TOML.
    Suite(SuiteArgs),
    /// Ficha del sistema.
    Sysinfo(SysinfoArgs),
    /// Compara ficheros de resultado.
    Compare(CompareArgs),
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

    /// Repite cada run N veces e informa mejor, media y desviación.
    #[arg(long, default_value_t = 1, value_parser = clap::value_parser!(u32).range(1..))]
    pub repeat: u32,

    /// Barrido de escalado: ejecuta con 1, 2, 4 … hilos hasta las CPU lógicas y muestra el
    /// speedup. Ignora --mode y --threads.
    #[arg(long)]
    pub scaling: bool,
}

/// Argumentos de `gugolpi pi`.
#[derive(Debug, Args)]
pub struct PiArgs {
    /// Tamaño: `16K`, `32K`, … `1M`, … `32M` (oficiales, como SuperPi).
    #[arg(long, default_value = "1M")]
    pub size: String,

    /// Guarda los dígitos calculados en este fichero (fuera del tiempo medido).
    #[arg(long, value_name = "FICHERO")]
    pub save_digits: Option<PathBuf>,

    /// Opciones comunes de ejecución. En modo multi cada hilo es una instancia independiente.
    #[command(flatten)]
    pub run: RunArgs,
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

/// Argumentos de `gugolpi zeta`.
#[derive(Debug, Args)]
pub struct ZetaArgs {
    /// Tamaño: `1G`, `10G`, `100G` (oficiales), `100M` o un N libre.
    #[arg(long, default_value = "10G")]
    pub size: String,

    /// Opciones comunes de ejecución.
    #[command(flatten)]
    pub run: RunArgs,
}

/// Argumentos de `gugolpi suite`.
#[derive(Debug, Args)]
pub struct SuiteArgs {
    /// Nombre de un preset (`classic`, `trio`, `full`) o ruta a un fichero TOML.
    pub suite: String,

    /// Repeticiones de cada test (sobrescribe el valor del fichero).
    #[arg(long, value_parser = clap::value_parser!(u32).range(1..))]
    pub repeat: Option<u32>,

    /// Sólo muestra el plan de la suite, sin ejecutar nada.
    #[arg(long)]
    pub dry_run: bool,
}

/// Argumentos de `gugolpi sysinfo`.
#[derive(Debug, Args)]
pub struct SysinfoArgs {}

/// Argumentos de `gugolpi compare`.
#[derive(Debug, Args)]
pub struct CompareArgs {
    /// Ficheros JSON de resultado o directorios que los contengan.
    #[arg(required = true, value_name = "FICHERO")]
    pub inputs: Vec<PathBuf>,

    /// Salida en CSV.
    #[arg(long)]
    pub csv: bool,

    /// Incluye runs no oficiales (tamaños libres, opciones que alteran la carga).
    #[arg(long)]
    pub include_unofficial: bool,
}

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
