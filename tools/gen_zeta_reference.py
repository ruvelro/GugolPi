#!/usr/bin/env python3
"""Genera la tabla de referencia del módulo Zeta: π(N) y suma de primos ≤ N (mod 2^64).

Criba segmentada sobre `bytearray` (sólo impares), independiente de la criba con rueda 30 de
GugolPi. Uso:

    python3 tools/gen_zeta_reference.py [salida.json] [N1 N2 ...]

Sin argumentos calcula 10^8, 10^9, 10^10 y 10^11 (el último tarda unos minutos).
"""
import json
import sys
import time
from itertools import compress

SEGMENT = 1 << 24  # impares por segmento (33,5 M números)


def small_primes(limit: int) -> list[int]:
    sieve = bytearray([1]) * (limit + 1)
    sieve[0:2] = b"\x00\x00"
    for i in range(2, int(limit**0.5) + 1):
        if sieve[i]:
            sieve[i * i :: i] = bytes(len(range(i * i, limit + 1, i)))
    return [i for i in range(limit + 1) if sieve[i]]


def count_and_sum(n: int) -> tuple[int, int]:
    """π(n) y suma de todos los primos ≤ n."""
    if n < 2:
        return 0, 0
    root = int(n**0.5) + 1
    primes = [p for p in small_primes(root) if p > 2]
    count, total = 1, 2  # el 2
    # Índice i del segmento representa el impar low + 2*i.
    low = 3
    while low <= n:
        high = min(low + 2 * SEGMENT - 2, n if n % 2 else n - 1)
        size = (high - low) // 2 + 1
        seg = bytearray([1]) * size
        for p in primes:
            if p * p > high:
                break
            start = max(p * p, ((low + p - 1) // p) * p)
            if start % 2 == 0:
                start += p
            first = (start - low) // 2
            if first < size:
                seg[first::p] = bytes(len(range(first, size, p)))
        count += seg.count(1)
        total += sum(compress(range(low, high + 1, 2), seg))
        low = high + 2
    return count, total


def main() -> None:
    out_path = sys.argv[1] if len(sys.argv) > 1 else "tools/zeta_reference.json"
    targets = [int(x) for x in sys.argv[2:]] or [10**8, 10**9, 10**10, 10**11]
    rows = []
    for n in targets:
        t0 = time.time()
        count, total = count_and_sum(n)
        rows.append(
            {
                "n": n,
                "prime_count": count,
                "prime_sum": str(total),
                "prime_sum_mod_2_64": f"{total % (1 << 64):016x}",
            }
        )
        print(f"N={n}: π={count} sum={total} ({time.time() - t0:.1f}s)", file=sys.stderr, flush=True)
        with open(out_path, "w", encoding="utf-8") as fh:
            json.dump({"method": "criba segmentada de impares, Python bytearray", "sizes": rows}, fh, indent=2)
            fh.write("\n")


if __name__ == "__main__":
    main()
