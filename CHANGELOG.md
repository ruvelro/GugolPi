# Changelog

Formato: [Keep a Changelog](https://keepachangelog.com/es/1.1.0/). Versionado semántico.

## [Unreleased]

### Añadido

- Workspace con `gugolpi-core` y `gugolpi-cli`, CI en Linux, macOS y Windows, `cargo deny`.
- API del núcleo: `Benchmark`, `RunConfig`, `RunResult` (esquema 1, sellado SHA-256),
  `Progress`, `CancelToken`, `SystemInfo`.
- Módulo Radical (compatible con wPrime): Newton–Raphson en `f64`, tamaños 32M y 1024M,
  single/multi, reparto dinámico o estático, afinidad, checksum determinista.
- CLI `gugolpi radical` y `gugolpi sysinfo` con `--json`, `--out`, `--repeat`, `--quiet`.

### Puntuación

- `score_version` = 1.
