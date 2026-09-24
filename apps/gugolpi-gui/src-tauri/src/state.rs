//! Estado compartido de la aplicación.

use std::path::PathBuf;
use std::sync::Mutex;

use gugolpi_core::{CancelToken, SystemInfo};

/// Estado global gestionado por Tauri.
pub struct AppState {
    /// Ficha del sistema, recogida al arrancar.
    pub system: SystemInfo,
    /// Directorio donde se guardan los resultados.
    pub results_dir: PathBuf,
    /// Token de la sesión en curso, si la hay.
    cancel: Mutex<Option<CancelToken>>,
}

impl AppState {
    /// Estado inicial.
    pub fn new(results_dir: PathBuf) -> Self {
        Self {
            system: SystemInfo::collect(),
            results_dir,
            cancel: Mutex::new(None),
        }
    }

    /// Reserva la sesión: falla si ya hay una en curso.
    pub fn begin_session(&self) -> Result<CancelToken, String> {
        let mut slot = self
            .cancel
            .lock()
            .map_err(|_| "estado corrupto".to_owned())?;
        if slot.is_some() {
            return Err("ya hay una ejecución en curso".to_owned());
        }
        let token = CancelToken::new();
        *slot = Some(token.clone());
        Ok(token)
    }

    /// Pide cancelar la sesión en curso, si la hay.
    pub fn cancel_session(&self) {
        if let Ok(slot) = self.cancel.lock()
            && let Some(token) = slot.as_ref()
        {
            token.cancel();
        }
    }

    /// Libera la sesión.
    pub fn end_session(&self) {
        if let Ok(mut slot) = self.cancel.lock() {
            *slot = None;
        }
    }
}
