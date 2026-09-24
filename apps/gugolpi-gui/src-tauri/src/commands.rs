//! Comandos invocables desde el frontend.
//!
//! Tauri inyecta `State` y los argumentos por valor; la firma la fija el macro `command`.

#![allow(clippy::needless_pass_by_value)]

use gugolpi_core::config::{PiOptions, RadicalOptions};
use gugolpi_core::result::ToolInfo;
use gugolpi_core::suite::{Job, PRESETS, SuiteFile, scaling_jobs};
use gugolpi_core::{Mode, Module, RunConfig, SystemInfo, Threads};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, State};
use tauri_plugin_opener::OpenerExt;

use crate::history::{self, HistoryEntry};
use crate::session;
use crate::state::AppState;

/// Ficha del sistema y de la build.
#[derive(Clone, Serialize)]
pub struct SystemSummary {
    /// Máquina.
    pub system: SystemInfo,
    /// Build.
    pub tool: ToolInfo,
    /// Directorio de resultados.
    pub results_dir: String,
}

/// Descripción de un módulo para la interfaz.
#[derive(Clone, Serialize)]
pub struct ModuleInfo {
    /// `pi`, `radical`, `zeta`.
    pub name: String,
    /// Título para mostrar.
    pub title: String,
    /// Frase de una línea.
    pub description: String,
    /// Tamaños oficiales.
    pub official_sizes: Vec<String>,
    /// Tamaños extendidos (no puntúan).
    pub extended_sizes: Vec<String>,
    /// Tamaño por defecto.
    pub default_size: String,
    /// Compatibilidad con el original.
    pub compatible_with: String,
}

/// Descripción de una suite.
#[derive(Clone, Serialize)]
pub struct SuiteInfo {
    /// Nombre del preset.
    pub name: String,
    /// Repeticiones por defecto.
    pub repeat: u32,
    /// Etiquetas de los jobs expandidos para esta máquina.
    pub jobs: Vec<String>,
}

/// Catálogo de módulos y suites.
#[derive(Clone, Serialize)]
pub struct Catalog {
    /// Módulos.
    pub modules: Vec<ModuleInfo>,
    /// Suites.
    pub suites: Vec<SuiteInfo>,
}

/// Petición de run desde la interfaz.
#[derive(Clone, Debug, Deserialize)]
pub struct RunRequest {
    /// Módulo.
    pub module: String,
    /// Tamaño.
    pub size: String,
    /// `single` o `multi`.
    #[serde(default)]
    pub mode: Option<String>,
    /// `auto`, `physical` o un número.
    #[serde(default)]
    pub threads: Option<String>,
    /// Afinidad.
    #[serde(default)]
    pub affinity: bool,
    /// Repeticiones.
    #[serde(default = "one")]
    pub repeat: u32,
    /// Radical: reparto estático.
    #[serde(default)]
    pub static_split: bool,
    /// Barrido de escalado.
    #[serde(default)]
    pub scaling: bool,
}

fn one() -> u32 {
    1
}

impl RunRequest {
    fn config(&self) -> Result<RunConfig, String> {
        let module: Module = self
            .module
            .parse()
            .map_err(|e: gugolpi_core::Error| e.to_string())?;
        let mut config = RunConfig::new(module, &self.size).map_err(|e| e.to_string())?;
        config.mode = match &self.mode {
            Some(m) => m.parse::<Mode>().map_err(|e| e.to_string())?,
            None => Mode::Single,
        };
        config.threads = match &self.threads {
            Some(t) => t.parse::<Threads>().map_err(|e| e.to_string())?,
            None => Threads::Auto,
        };
        config.affinity = self.affinity;
        config.radical = RadicalOptions {
            static_split: self.static_split,
        };
        config.pi = PiOptions::default();
        Ok(config)
    }
}

/// Ficha del sistema.
#[tauri::command]
pub fn system_summary(state: State<'_, AppState>) -> SystemSummary {
    SystemSummary {
        system: state.system.clone(),
        tool: ToolInfo::current(),
        results_dir: state.results_dir.display().to_string(),
    }
}

/// Catálogo de módulos y suites para esta máquina.
#[tauri::command]
pub fn catalog(state: State<'_, AppState>) -> Result<Catalog, String> {
    let modules = Module::ALL
        .iter()
        .map(|&m| {
            let (title, description, compatible_with, default_size) = match m {
                Module::Pi => (
                    "Pi",
                    "Dígitos de Pi por Gauss–Legendre con multiplicación FFT.",
                    "SuperPi",
                    "1M",
                ),
                Module::Radical => (
                    "Radical",
                    "Raíces cuadradas de 1..N por Newton–Raphson.",
                    "wPrime",
                    "32M",
                ),
                Module::Zeta => (
                    "Zeta",
                    "Primos hasta N con criba de Eratóstenes segmentada.",
                    "propio de GugolPi",
                    "10G",
                ),
            };
            ModuleInfo {
                name: m.name().to_owned(),
                title: title.to_owned(),
                description: description.to_owned(),
                official_sizes: m.official_sizes().iter().map(|s| (*s).to_owned()).collect(),
                extended_sizes: m.extended_sizes().iter().map(|s| (*s).to_owned()).collect(),
                default_size: default_size.to_owned(),
                compatible_with: compatible_with.to_owned(),
            }
        })
        .collect();
    let mut suites = Vec::new();
    for (name, _) in PRESETS {
        let suite = SuiteFile::preset(name)
            .ok_or_else(|| format!("preset {name} ausente"))?
            .map_err(|e| e.to_string())?;
        let jobs = suite.jobs(&state.system).map_err(|e| e.to_string())?;
        suites.push(SuiteInfo {
            name: (*name).to_owned(),
            repeat: suite.suite.repeat,
            jobs: jobs.into_iter().map(|j| j.label).collect(),
        });
    }
    Ok(Catalog { modules, suites })
}

/// Lanza un run (o un barrido de escalado) en segundo plano.
#[tauri::command]
pub fn start_run(
    app: AppHandle,
    state: State<'_, AppState>,
    request: RunRequest,
) -> Result<(), String> {
    let config = request.config()?;
    let jobs = if request.scaling {
        scaling_jobs(&config, &state.system)
    } else {
        vec![Job::from_config(config)]
    };
    session::spawn(app, jobs, request.repeat)
}

/// Lanza una suite (preset) en segundo plano.
#[tauri::command]
pub fn start_suite(
    app: AppHandle,
    state: State<'_, AppState>,
    name: String,
    repeat: Option<u32>,
) -> Result<(), String> {
    let suite = SuiteFile::preset(&name)
        .ok_or_else(|| format!("no existe el preset «{name}»"))?
        .map_err(|e| e.to_string())?;
    let jobs = suite.jobs(&state.system).map_err(|e| e.to_string())?;
    session::spawn(app, jobs, repeat.unwrap_or(suite.suite.repeat))
}

/// Cancela la sesión en curso.
#[tauri::command]
pub fn cancel_session(state: State<'_, AppState>) {
    state.cancel_session();
}

/// Historial de resultados, del más reciente al más antiguo.
#[tauri::command]
pub fn history(state: State<'_, AppState>) -> Result<Vec<HistoryEntry>, String> {
    history::load(&state.results_dir)
}

/// Borra una entrada del historial.
#[tauri::command]
pub fn delete_history_entry(state: State<'_, AppState>, path: String) -> Result<(), String> {
    history::delete(&state.results_dir, &path)
}

/// Abre el directorio de resultados en el explorador de ficheros.
#[tauri::command]
pub fn open_results_dir(app: AppHandle, state: State<'_, AppState>) -> Result<(), String> {
    std::fs::create_dir_all(&state.results_dir).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(state.results_dir.display().to_string(), None::<&str>)
        .map_err(|e| e.to_string())
}
