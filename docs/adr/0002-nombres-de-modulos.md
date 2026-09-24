# ADR-0002: Nombres de los módulos: Pi, Radical y Zeta

- Estado: aceptada
- Fecha: 2026-09-24

## Contexto

SuperPi y wPrime son marcas ajenas. wPrime, además, no calcula primos: calcula raíces cuadradas
por Newton. Queríamos un módulo real de primos y nombres propios, cortos y relacionados con la
carga.

## Decisión

- **Pi**: reimplementación de SuperPi (Gauss–Legendre + FFT).
- **Radical**: reimplementación de wPrime; el nombre viene del signo radical (√).
- **Zeta**: módulo propio de primos (criba segmentada); por la función zeta de Riemann.

En la interfaz se indica la compatibilidad ("Radical, compatible con wPrime 32M").

## Consecuencias

Los nombres son estables desde la 1.0: cambiarlos rompería suites, ficheros de resultado y la
CLI.
