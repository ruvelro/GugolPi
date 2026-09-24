//! Errores del núcleo. La biblioteca nunca hace `panic`: todo fallo llega por aquí.

use crate::config::Module;

/// Error del núcleo de GugolPi.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// La configuración recibida es contradictoria o está fuera de rango.
    #[error("configuración no válida: {0}")]
    InvalidConfig(String),

    /// El texto de tamaño no corresponde a ningún tamaño del módulo.
    #[error("tamaño desconocido «{text}» para el módulo {module}")]
    UnknownSize {
        /// Texto recibido.
        text: String,
        /// Módulo al que se aplicaba.
        module: Module,
    },

    /// No hay memoria suficiente para el cálculo pedido.
    #[error("memoria insuficiente: se necesitan {required} bytes y hay {available} disponibles")]
    InsufficientMemory {
        /// Bytes necesarios.
        required: u64,
        /// Bytes disponibles.
        available: u64,
    },

    /// El cálculo terminó pero el resultado no verifica.
    #[error("verificación fallida: {0}")]
    Verification(String),

    /// El run se canceló desde fuera.
    #[error("ejecución cancelada")]
    Cancelled,

    /// El módulo existe en la spec pero no en esta build.
    #[error("el módulo {0} no está disponible en esta build")]
    Unavailable(Module),

    /// Error de entrada/salida al leer o escribir un fichero.
    #[error("no se pudo acceder a {path}: {source}")]
    Io {
        /// Ruta afectada.
        path: String,
        /// Causa.
        #[source]
        source: std::io::Error,
    },

    /// Fallo interno que no debería ocurrir (hilo caído, invariante rota).
    #[error("error interno: {0}")]
    Internal(String),
}

/// Alias de resultado del núcleo.
pub type Result<T> = std::result::Result<T, Error>;
