# ADR-0006: Tamaños oficiales de Zeta: 1G, 10G y 100G

- Estado: aceptada (sustituye la tabla inicial de la spec: 100M, 1G, 10G)
- Fecha: 2026-09-24

## Contexto

La criba segmentada con rueda 30 cuenta los primos hasta 10^9 en 0,36 s con un hilo en un
portátil actual, y en 0,06 s con diez. Un test tan corto está dominado por el arranque de hilos y
no discrimina entre CPUs.

## Decisión

Tamaños oficiales: 1G, 10G (cifra insignia, ≈ 4 s single) y 100G (multicore). 100M pasa a
extendido, útil para pruebas rápidas. Los bloques de la cola dinámica bajan de 64 a 16 segmentos
para repartir mejor en 1G.

## Consecuencias

- La referencia de 10^11 (π = 4 118 054 813, suma = 201 467 077 743 744 681 014) queda embebida y
  cotejada con la OEIS A046731.
- Con 100G como máximo, todos los primos de criba (≤ 316 228) caben en un segmento, así que el
  bucket sieve de la spec no hace falta en la 1.0; se añadirá si algún día se supera 10^12.
