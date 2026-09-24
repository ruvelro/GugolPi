//! Tabla de referencia embebida: π(N) y suma de primos ≤ N (mod 2^64) por tamaño.
//!
//! Generada con `tools/gen_zeta_reference.py` (criba de impares en Python, independiente de la
//! rueda 30 de GugolPi) y guardada en `data/zeta_reference.json`. Los valores coinciden con la
//! secuencia A046731 de la OEIS.

use std::sync::OnceLock;

use serde::Deserialize;

const JSON: &str = include_str!("../../data/zeta_reference.json");

#[derive(Debug, Deserialize)]
struct Table {
    sizes: Vec<Entry>,
}

/// Una entrada de la tabla.
#[derive(Clone, Debug, Deserialize)]
pub struct Entry {
    /// Límite N.
    pub n: u64,
    /// π(N).
    pub prime_count: u64,
    /// Suma exacta de los primos ≤ N, en decimal.
    pub prime_sum: String,
    /// Esa suma módulo 2^64, en hexadecimal de 16 dígitos.
    pub prime_sum_mod_2_64: String,
}

fn table() -> &'static [Entry] {
    static TABLE: OnceLock<Vec<Entry>> = OnceLock::new();
    TABLE.get_or_init(|| {
        serde_json::from_str::<Table>(JSON)
            .map(|t| t.sizes)
            .unwrap_or_default()
    })
}

/// Todas las entradas.
pub fn entries() -> &'static [Entry] {
    table()
}

/// Entrada de referencia para `n`, si existe.
pub fn entry_for(n: u64) -> Option<&'static Entry> {
    table().iter().find(|e| e.n == n)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_covers_official_sizes() {
        for label in crate::config::Module::Zeta.official_sizes() {
            let size = crate::config::Module::Zeta.parse_size(label).expect("size");
            assert!(
                entry_for(size.value).is_some(),
                "falta referencia para {label}"
            );
        }
    }
}
