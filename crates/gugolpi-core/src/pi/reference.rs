//! Tabla de referencia embebida: SHA-256 de los D primeros decimales de Pi por tamaño.
//!
//! Generada con `tools/gen_pi_reference.py` (Gauss–Legendre sobre `decimal`, una implementación
//! independiente de la FFT de GugolPi) y guardada en `data/pi_reference.json`.

use std::sync::OnceLock;

use serde::Deserialize;

const JSON: &str = include_str!("../../data/pi_reference.json");

#[derive(Debug, Deserialize)]
struct Table {
    sizes: Vec<Entry>,
}

/// Una entrada de la tabla.
#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    /// Etiqueta (`1M`).
    pub label: String,
    /// Número de decimales.
    pub digits: u64,
    /// SHA-256 hexadecimal de los `digits` primeros decimales en ASCII.
    pub sha256: String,
    /// Últimos 20 decimales, para cotejos a ojo.
    pub last_digits: String,
}

fn table() -> &'static [Entry] {
    static TABLE: OnceLock<Vec<Entry>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str::<Table>(JSON)
            .map(|t| t.sizes)
            .unwrap_or_default()
    })
}

/// Todas las entradas, en orden creciente de tamaño.
pub fn entries() -> &'static [Entry] {
    table()
}

/// SHA-256 esperado para `digits` decimales, si hay referencia.
pub fn sha256_for(digits: u64) -> Option<&'static str> {
    table()
        .iter()
        .find(|e| e.digits == digits)
        .map(|e| e.sha256.as_str())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_official_sizes() {
        for label in crate::config::Module::Pi.official_sizes() {
            let size = crate::config::Module::Pi.parse_size(label).expect("size");
            assert!(
                sha256_for(size.value).is_some(),
                "falta referencia para {label}"
            );
        }
    }
}
