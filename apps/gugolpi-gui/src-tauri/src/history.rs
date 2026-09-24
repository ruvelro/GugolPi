//! Historial: los JSON de resultado guardados en el directorio de datos de la aplicación.

use std::path::{Path, PathBuf};

use gugolpi_core::RunResult;
use serde::Serialize;

/// Una entrada del historial.
#[derive(Clone, Debug, Serialize)]
pub struct HistoryEntry {
    /// Ruta completa del fichero.
    pub path: String,
    /// Nombre del fichero.
    pub file_name: String,
    /// Resultado.
    pub result: RunResult,
    /// `true` si el hash de integridad coincide.
    pub intact: bool,
}

/// Lee todos los resultados del directorio, del más reciente al más antiguo.
pub fn load(dir: &Path) -> Result<Vec<HistoryEntry>, String> {
    let mut entries = Vec::new();
    let read = match std::fs::read_dir(dir) {
        Ok(read) => read,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return Ok(entries),
        Err(err) => return Err(format!("no se pudo leer {}: {err}", dir.display())),
    };
    for entry in read.flatten() {
        let path = entry.path();
        if path.extension().is_none_or(|ext| ext != "json") {
            continue;
        }
        let Ok(text) = std::fs::read_to_string(&path) else {
            continue;
        };
        let Ok(result) = serde_json::from_str::<RunResult>(&text) else {
            continue;
        };
        entries.push(HistoryEntry {
            file_name: path
                .file_name()
                .map(|f| f.to_string_lossy().into_owned())
                .unwrap_or_default(),
            path: path.display().to_string(),
            intact: result.integrity_ok(),
            result,
        });
    }
    entries.sort_by(|a, b| {
        b.result
            .timing
            .started_utc
            .cmp(&a.result.timing.started_utc)
    });
    Ok(entries)
}

/// Borra un fichero del historial. Sólo admite rutas dentro del directorio de resultados.
pub fn delete(dir: &Path, path: &str) -> Result<(), String> {
    let target = PathBuf::from(path);
    let canonical_dir = dir.canonicalize().map_err(|e| e.to_string())?;
    let canonical = target.canonicalize().map_err(|e| e.to_string())?;
    if !canonical.starts_with(&canonical_dir) || canonical.extension().is_none_or(|e| e != "json") {
        return Err("la ruta no pertenece al historial".to_owned());
    }
    std::fs::remove_file(&canonical).map_err(|e| format!("no se pudo borrar: {e}"))
}
