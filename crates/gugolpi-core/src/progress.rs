//! Canal tipado de progreso. Sirve igual para una barra en la CLI y para la GUI.

use std::sync::mpsc::{self, Receiver, Sender};

use serde::Serialize;

use crate::config::Module;

/// Evento de progreso emitido por un benchmark mientras corre.
///
/// Se serializa con la etiqueta `kind` (`started`, `advanced`, `loop`, `verifying`, `finished`)
/// para que la GUI lo reciba tal cual.
#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
#[non_exhaustive]
pub enum ProgressEvent {
    /// El cálculo ha empezado (el cronómetro ya corre).
    Started {
        /// Módulo en ejecución.
        module: Module,
        /// Unidades de trabajo totales (números, dígitos, loops…).
        total: u64,
        /// Nombre de la unidad, para mostrar.
        unit: &'static str,
    },
    /// Trabajo completado hasta ahora.
    Advanced {
        /// Unidades completadas.
        done: u64,
        /// Unidades totales.
        total: u64,
    },
    /// Un loop de Pi ha terminado (equivalente a la línea "Loop n" de SuperPi).
    Loop {
        /// Índice del loop, empezando en 1.
        index: u32,
        /// Duración del loop en segundos.
        seconds: f64,
        /// Tiempo acumulado en segundos.
        cumulative_seconds: f64,
    },
    /// El cálculo ha terminado y se está verificando.
    Verifying,
    /// Todo ha terminado.
    Finished,
}

/// Emisor de progreso. Un emisor sin receptor descarta los eventos sin coste.
#[derive(Clone, Debug, Default)]
pub struct Progress {
    sender: Option<Sender<ProgressEvent>>,
}

impl Progress {
    /// Emisor que descarta todo (para tests y runs silenciosos).
    pub fn none() -> Self {
        Self { sender: None }
    }

    /// Crea un par emisor/receptor.
    pub fn channel() -> (Self, Receiver<ProgressEvent>) {
        let (tx, rx) = mpsc::channel();
        (Self { sender: Some(tx) }, rx)
    }

    /// Emite un evento. Si el receptor ha desaparecido, el evento se descarta.
    pub fn send(&self, event: ProgressEvent) {
        if let Some(tx) = &self.sender {
            let _ = tx.send(event);
        }
    }
}
