# GugolPi

Benchmark de CPU en Rust. La spec completa está en `SPEC.md`; léela antes de tocar el núcleo.

## Reglas

- Sin `unwrap`/`expect`/`panic` en `gugolpi-core` (clippy lo deniega). Errores por `Error`.
- Sin `unsafe` fuera del módulo `simd` (aún no existe).
- El núcleo no imprime ni serializa a formatos de presentación; eso es de la CLI/GUI.
- Cambios que muevan tiempos: `score::SCORE_VERSION` + `CHANGELOG.md`.
- Decisiones de diseño: ADR en `docs/adr/`. Commits: Conventional Commits.
- Antes de dar algo por hecho: `cargo fmt --all`, `cargo clippy --workspace --all-targets
  --all-features -- -D warnings`, `cargo test --workspace`.

## Toolchain en esta máquina

`cargo` está en `/opt/homebrew/opt/rustup/bin` (añadir al PATH en cada shell).
