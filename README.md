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

Camino a la 1.0: los tres módulos y la CLI completa están implementados y verificados. Falta
la tabla de correlación con los originales y los instaladores (versión 1.3 en la spec).

## Uso rápido

```bash
cargo build --release
./target/release/gugolpi sysinfo
./target/release/gugolpi pi --size 1M                      # la cifra insignia, como SuperPi 1M
./target/release/gugolpi radical --size 32M --mode multi    # como wPrime 32M
./target/release/gugolpi zeta --size 10G --mode multi       # primos hasta 10^10
./target/release/gugolpi radical --size 32M --scaling       # barrido 1, 2, 4 … hilos
./target/release/gugolpi suite trio --repeat 3              # pi 1M + radical 32M + zeta 10G
./target/release/gugolpi compare results/trio-*/            # tabla comparativa
./target/release/gugolpi pi --size 1M --json > run.json
```

Opciones comunes: `--mode single|multi`, `--threads auto|physical|N`, `--affinity`, `--repeat N`,
`--scaling`, `--json`, `--quiet`, `--out DIR`.

Códigos de salida: `0` correcto, `1` error de uso, `2` verificación fallida (CPU inestable),
`3` recursos insuficientes.

Tiempos orientativos en un Apple M5 (10 núcleos): Pi 1M single 3,0 s, 32M 201 s · Radical 32M
multi 0,22 s · Zeta 10G single 3,8 s, multi 0,6 s. Pi 32M necesita unos 1,8 GB de RAM.

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
