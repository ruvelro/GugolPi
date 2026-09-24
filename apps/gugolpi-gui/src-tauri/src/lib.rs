//! GUI de GugolPi sobre Tauri 2.
//!
//! Capa fina sobre `gugolpi-core`: los comandos exponen la ficha del sistema, el catálogo de
//! módulos y suites, lanzan sesiones de ejecución que emiten eventos de progreso y gestionan el
//! historial de resultados (los mismos JSON que escribe la CLI).

#![allow(clippy::print_stderr)]

mod commands;
mod history;
mod session;
mod state;

use tauri::Manager;

use crate::state::AppState;

/// Arranca la aplicación.
pub fn run() {
    let outcome = tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .setup(|app| {
            let results_dir = app.path().app_data_dir()?.join("results");
            std::fs::create_dir_all(&results_dir)?;
            app.manage(AppState::new(results_dir));
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::system_summary,
            commands::catalog,
            commands::start_run,
            commands::start_suite,
            commands::cancel_session,
            commands::history,
            commands::delete_history_entry,
            commands::open_results_dir,
        ])
        .run(tauri::generate_context!());
    if let Err(err) = outcome {
        eprintln!("error al arrancar GugolPi: {err}");
        std::process::exit(1);
    }
}
