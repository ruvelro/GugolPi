# ADR-0005: Dígitos balanceados en la convolución FFT

- Estado: aceptada
- Fecha: 2026-09-24

## Contexto

La multiplicación del módulo Pi es una convolución por FFT en `f64` sobre limbs en base 10^4. Con
limbs en `[0, 10^4)` el error de redondeo máximo crecía linealmente con el tamaño: 0,0003 en 64K,
0,0013 en 256K, 0,0059 en 1M y, extrapolando, unos 0,25 en 32M, justo el límite a partir del cual
el resultado deja de ser fiable (SuperPi abortaba con "not exact in round" en ese punto).

## Decisión

Antes de transformar, cada operando se convierte a dígitos balanceados en `[-5000, 5000)` con
acarreo. Los coeficientes de la convolución se cancelan en vez de acumularse y el error medido
baja por debajo de 10^-4 en todos los tamaños. El valor numérico no cambia; el acarreo final se
hace con signo (`rem_euclid` / `div_euclid`).

## Consecuencias

- Margen de varios órdenes de magnitud frente al límite de 0,25 en 32M, incluso en CPUs
  ligeramente inestables (que se siguen detectando por el hash de dígitos y por el error).
- Un bucle O(n) extra por operando, despreciable frente a la FFT; en la práctica el cálculo es
  algo más rápido porque los operandos se recortan de ceros iniciales y finales al balancear.
- Fija la carga de trabajo del `score_version` 1: cambiarlo exige subir la versión.
