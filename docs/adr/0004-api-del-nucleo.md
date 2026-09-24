# ADR-0004: API del núcleo estable desde el primer commit

- Estado: aceptada
- Fecha: 2026-09-24

## Contexto

GUI (1.1), Gúgol (1.2) y firma (1.3) se construyen sobre el núcleo de la 1.0. No queremos
refactorizar la API cuando lleguen.

## Decisión

- `Benchmark` es un trait con `run(config, system, progress, cancel) -> Result<RunResult>`;
  `benchmark_for(module)` devuelve `Error::Unavailable` para módulos aún no implementados en vez
  de cambiar de tipo de retorno más adelante.
- `RunConfig` y `RunResult` son structs serde con `schema_version`; los campos nuevos son
  opcionales (`#[serde(default)]`).
- El progreso es un canal tipado (`ProgressEvent`), no callbacks.
- Los enums públicos son `#[non_exhaustive]` cuando crecerán (`Error`, `ProgressEvent`).
- Las constantes de puntuación viven en `score` con `SCORE_VERSION`.

## Consecuencias

Añadir Pi, Zeta o Gúgol es añadir implementaciones y variantes, no cambiar firmas. Los ficheros
JSON de la 1.0 seguirán leyéndose en versiones posteriores.
