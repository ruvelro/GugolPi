# GugolPi

Benchmark de CPU multiplataforma con tres módulos y verificación matemática del resultado:

| Módulo | Qué mide | Compatible con |
| --- | --- | --- |
| **Pi** | Dígitos de Pi por Gauss–Legendre con multiplicación FFT | SuperPi (1M, 32M…) |
| **Radical** | Raíces cuadradas de 1..N por Newton–Raphson | wPrime (32M, 1024M) |
| **Zeta** | Primos hasta N con criba de Eratóstenes segmentada | Propio de GugolPi (1G) |

Los tres módulos corren en modo single-core o multicore, se automatizan desde la CLI y producen un
fichero JSON sellado con un hash de integridad. Un cuarto modo, **Gúgol**, ejecuta cualquiera de
las tres cargas como stress test abierto (versión 1.2).

La especificación completa está en [SPEC.md](SPEC.md).

## Estado

En desarrollo hacia la 1.0. Disponible ahora:

- `gugolpi radical`: módulo Radical completo (single, multi, reparto dinámico o estático,
  afinidad, repeticiones, JSON).
- `gugolpi sysinfo`: ficha del sistema.

Pendiente para la 1.0: módulos Pi y Zeta, `suite` y `compare`.

## Uso rápido

```bash
cargo build --release
./target/release/gugolpi sysinfo
./target/release/gugolpi radical --size 32M
./target/release/gugolpi radical --size 32M --mode multi --threads auto
./target/release/gugolpi radical --size 1024M --mode multi --repeat 3 --out results/
./target/release/gugolpi radical --size 32M --json > run.json
```

Opciones comunes: `--mode single|multi`, `--threads auto|physical|N`, `--affinity`, `--repeat N`,
`--json`, `--quiet`, `--out DIR`.

Códigos de salida: `0` correcto, `1` error de uso, `2` verificación fallida (CPU inestable),
`3` recursos insuficientes.

## Comparabilidad

Cada resultado lleva `score_version`. Sólo son comparables entre sí los runs con el mismo valor,
el mismo test, tamaño y modo, y `official = true` (tamaño oficial, sin opciones que alteren la
carga, verificación correcta). Los tiempos absolutos no coinciden con SuperPi ni wPrime (son
binarios cerrados con su propia FFT y precisión x87), pero el algoritmo, los tamaños y el esquema
de informe son los mismos, así que la ordenación entre CPUs se mantiene.

## Estructura

```
crates/gugolpi-core/   biblioteca: benchmarks, resultados, ficha del sistema
crates/gugolpi-cli/    binario `gugolpi`
docs/adr/              decisiones de arquitectura
suites/                presets de suites (TOML)
```

## Desarrollo

```bash
cargo fmt --all
cargo clippy --workspace --all-targets --all-features -- -D warnings
cargo test --workspace
```

La CI exige formato, clippy sin avisos, tests en Linux, macOS y Windows, y `cargo deny`.
Las convenciones están en [CONTRIBUTING.md](CONTRIBUTING.md).

## Licencia

MIT. Ver [LICENSE](LICENSE).
