//! Códigos de salida de la CLI, como fija la spec.

use gugolpi_core::Error;

/// Código de salida del proceso.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExitCode {
    /// Todo correcto.
    Ok,
    /// Error de uso o de configuración.
    Usage,
    /// La verificación matemática falló (CPU inestable o bug).
    VerificationFailed,
    /// Recursos insuficientes (memoria).
    InsufficientResources,
}

impl ExitCode {
    /// Valor numérico para `std::process::exit`.
    pub fn code(self) -> i32 {
        match self {
            ExitCode::Ok => 0,
            ExitCode::Usage => 1,
            ExitCode::VerificationFailed => 2,
            ExitCode::InsufficientResources => 3,
        }
    }

    /// Código que corresponde a un error, mirando si en la cadena hay uno del núcleo.
    pub fn from_error(err: &anyhow::Error) -> Self {
        match err.downcast_ref::<Error>() {
            Some(Error::InsufficientMemory { .. }) => ExitCode::InsufficientResources,
            Some(Error::Verification(_)) => ExitCode::VerificationFailed,
            _ => ExitCode::Usage,
        }
    }
}
