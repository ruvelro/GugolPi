# Changelog

Formato: [Keep a Changelog](https://keepachangelog.com/es/1.1.0/). Versionado semántico.

## [Unreleased]

### Añadido

- Workspace con `gugolpi-core` y `gugolpi-cli`, CI en Linux, macOS y Windows, `cargo deny`.
- API del núcleo: `Benchmark`, `RunConfig`, `RunResult` (esquema 1, sellado SHA-256),
  `Progress`, `CancelToken`, `SystemInfo`.
- Módulo Pi (compatible con SuperPi): aritmética propia en base 10^4 con FFT en `f64` y dígitos
  balanceados, Gauss–Legendre con 19 loops en 1M y 24 en 32M, referencias SHA-256 de 16K a 32M
  generadas con una implementación independiente, modo multi por instancias.
- Módulo Radical (compatible con wPrime): Newton–Raphson en `f64`, tamaños 32M y 1024M,
  single/multi, reparto dinámico o estático, afinidad, checksum determinista.
- Módulo Zeta (primos): criba de Eratóstenes segmentada con rueda 30, tamaños 1G, 10G y 100G,
  verificación de π(N) y suma de primos contra referencias independientes.
- CLI: `pi`, `radical`, `zeta` (con `--scaling`), `suite` (presets `classic`, `trio`, `full` o
  TOML propio, `summary.csv`), `compare`, `sysinfo`; `--json`, `--out`, `--repeat`, `--quiet`.

### Puntuación

- `score_version` = 1.
