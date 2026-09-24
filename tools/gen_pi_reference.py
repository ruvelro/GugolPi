#!/usr/bin/env python3
"""Genera la tabla de referencia de dígitos de Pi para el módulo Pi.

Calcula 32M dígitos (2^25) con Gauss–Legendre sobre `decimal` (libmpdec, multiplicación NTT,
independiente de la FFT en f64 de GugolPi) y guarda, para cada tamaño oficial y extendido, el
SHA-256 de los D primeros decimales. Uso:

    python3 tools/gen_pi_reference.py [max_digits] [salida.json] [fichero_digitos]

Los dígitos completos se guardan aparte para cotejos manuales (no van al repositorio).
"""
import hashlib
import json
import math
import sys
import time
from decimal import Decimal, ROUND_DOWN, getcontext

GUARD = 64


def agm_pi(digits: int) -> str:
    """Decimales de Pi (sin el 3.) por Gauss–Legendre, truncando en cada paso."""
    getcontext().prec = digits + GUARD
    getcontext().rounding = ROUND_DOWN
    a = Decimal(1)
    b = (Decimal(1) / Decimal(2)).sqrt()
    t = Decimal(1) / Decimal(4)
    p = 1
    loops = math.ceil(math.log2(digits) - 1.448)
    for i in range(loops):
        t0 = time.time()
        a1 = (a + b) / 2
        b = (a * b).sqrt()
        t -= p * (a - a1) ** 2
        p *= 2
        a = a1
        print(f"loop {i + 1}/{loops}: {time.time() - t0:.1f}s", file=sys.stderr, flush=True)
    pi = (a + b) ** 2 / (4 * t)
    text = str(pi)
    assert text.startswith("3.")
    return text[2 : 2 + digits]


def main() -> None:
    max_digits = int(sys.argv[1]) if len(sys.argv) > 1 else 1 << 25
    out_path = sys.argv[2] if len(sys.argv) > 2 else "tools/pi_reference.json"
    digits_path = sys.argv[3] if len(sys.argv) > 3 else None
    t0 = time.time()
    decimals = agm_pi(max_digits)
    print(f"total {time.time() - t0:.1f}s", file=sys.stderr)
    assert decimals.startswith("14159265358979323846264338327950288419716939937510")
    if max_digits >= 1_000_000:
        # Dígitos 999 996 a 1 000 000 tras la coma: 5 8 1 5 1 (hecho conocido).
        assert decimals[999_995:1_000_000] == "58151", decimals[999_990:1_000_000]
    if digits_path:
        with open(digits_path, "w", encoding="ascii") as fh:
            fh.write("3." + decimals + "\n")
    sizes = []
    k = 14
    while (1 << k) <= max_digits:
        d = 1 << k
        label = f"{d >> 10}K" if d < (1 << 20) else (f"{d >> 20}M" if d < (1 << 30) else f"{d >> 30}G")
        sizes.append(
            {
                "label": label,
                "digits": d,
                "sha256": hashlib.sha256(decimals[:d].encode("ascii")).hexdigest(),
                "last_digits": decimals[d - 20 : d],
            }
        )
        k += 1
    with open(out_path, "w", encoding="utf-8") as fh:
        json.dump({"method": "Gauss-Legendre, Python decimal (libmpdec)", "sizes": sizes}, fh, indent=2)
        fh.write("\n")
    print(f"escrito {out_path} con {len(sizes)} tamaños", file=sys.stderr)


if __name__ == "__main__":
    main()
