# Contribuir a GugolPi

## Principios

- La API del núcleo (`Benchmark`, `RunConfig`, `RunResult`, `Progress`) es estable: los campos
  nuevos son opcionales y compatibles con ficheros antiguos.
- El núcleo no hace `panic` ni usa `unwrap`/`expect`; los errores son `gugolpi_core::Error`.
- El núcleo no sabe de terminales ni de JSON bonito: la presentación vive en la CLI o la GUI.
- Todo lo que mueve los tiempos vive en `gugolpi_core::score` y exige subir `SCORE_VERSION`
  con una entrada en el changelog.
- `unsafe` sólo en el módulo `simd`, aislado tras funciones seguras y probado contra la versión
  escalar.

## Flujo

1. Rama desde `main`.
2. `cargo fmt --all`, `cargo clippy --workspace --all-targets --all-features -- -D warnings`,
   `cargo test --workspace`.
3. Commits en formato [Conventional Commits](https://www.conventionalcommits.org/) en español o
   inglés: `feat(radical): reparto estático`, `fix(cli): código de salida en --help`.
4. Cambios de comportamiento visibles: entrada en `CHANGELOG.md`.
5. Decisiones de diseño: un ADR nuevo en `docs/adr/` (plantilla en `0000-plantilla.md`).
6. Pull request; la CI debe estar en verde.

## Tests

- Unitarios junto al código (`#[cfg(test)]`).
- Integración de la CLI en `crates/gugolpi-cli/tests/`.
- Tests dorados (dígitos de Pi, π(N), checksums) cuando lleguen los módulos.
