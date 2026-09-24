# ADR-0003: El módulo Pi se comporta por defecto como SuperPi

- Estado: aceptada
- Fecha: 2026-09-24

## Contexto

La cifra de SuperPi 1M tiene veinte años de histórico. Para que la gente pueda situar su
resultado, el modo por defecto debe replicar al original.

## Decisión

`gugolpi pi` sin opciones = un hilo, tamaños clásicos (16K–32M), informe por loop y total. El modo
multi por defecto son instancias independientes (equivalente a "SuperPi ×N"); la FFT paralela es
una opción no oficial.

## Consecuencias

La cifra insignia es "Pi 1M single". Las mejoras de paralelismo no cambian el modo por defecto.
