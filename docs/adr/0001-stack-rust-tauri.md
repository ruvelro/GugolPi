# ADR-0001: Núcleo en Rust, GUI en Tauri 2

- Estado: aceptada
- Fecha: 2026-09-24

## Contexto

GugolPi debe correr en Windows, macOS y Linux (x86-64 y ARM64), medir con precisión (sin
recolector de basura ni pausas), usar SIMD y ofrecer una interfaz moderna además de la CLI.

## Decisión

Un workspace de Cargo con `gugolpi-core` (biblioteca con toda la lógica), `gugolpi-cli` (clap) y,
en la 1.1, `gugolpi-gui` (Tauri 2 con frontend web). La GUI y la CLI son capas finas sobre el
mismo núcleo.

## Consecuencias

- Un solo lenguaje para toda la lógica y despacho SIMD en tiempo de ejecución con `std::arch`.
- Tauri añade una toolchain web al repositorio, pero da instaladores nativos pequeños.
- Alternativas descartadas: C++ con OpenMP (tres toolchains que mantener) y egui (menos pulido).
