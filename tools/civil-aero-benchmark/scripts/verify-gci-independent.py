#!/usr/bin/env python3
"""Independent stdlib oracle for a six-grid quadratic GCI known-answer fixture."""
from __future__ import annotations

import csv
import math
import sys
from pathlib import Path

SPACINGS = [4.0, 2.0, 1.0, 0.5, 0.25, 0.125]
VALUES = [2.0 + 0.8 * h**2 for h in SPACINGS]
SAFETY = 1.25


def require(ok: bool, message: str) -> None:
    if not ok:
        raise ValueError(message)


def observed_order(ratio: float, r_cm: float, r_mf: float) -> float:
    require(ratio > math.log(r_cm) / math.log(r_mf), "no positive order")
    low, high = 0.0, 1.0

    def function(order: float) -> float:
        return (r_cm**order - 1.0) / (1.0 - r_mf**(-order))

    while function(high) < ratio:
        high *= 2.0
        require(high <= 128.0, "order bracket exceeded")
    for _ in range(100):
        mid = (low + high) / 2.0
        if function(mid) < ratio:
            low = mid
        else:
            high = mid
    return (low + high) / 2.0


def main() -> int:
    if len(sys.argv) != 2:
        print("usage: verify-gci-independent.py output.csv", file=sys.stderr)
        return 2
    with Path(sys.argv[1]).open(newline="", encoding="utf-8") as handle:
        rows = list(csv.DictReader(handle))
    require(len(rows) == 4, f"expected 4 sliding triplets, got {len(rows)}")
    require(rows[0]["quantity"] == "CD", "unexpected quantity")
    require(rows[0]["grid_family_kind"] == "structured", "grid-family type mismatch")
    require(float(rows[0]["safety_factor"]) == SAFETY, "structured safety factor mismatch")
    for index, row in enumerate(rows):
        h0, h1, h2 = SPACINGS[index:index + 3]
        p0, p1, p2 = VALUES[index:index + 3]
        r_cm, r_mf = h0 / h1, h1 / h2
        ratio = abs((p0 - p1) / (p1 - p2))
        order = observed_order(ratio, r_cm, r_mf)
        gci_m = SAFETY * abs(p0 - p1) / (r_cm**order - 1.0)
        gci_f = SAFETY * abs(p1 - p2) / (r_mf**order - 1.0)
        extrapolated = p2 + (p2 - p1) / (r_mf**order - 1.0)
        expected = {
            "h_coarse": h0,
            "h_medium": h1,
            "h_fine": h2,
            "observed_order": order,
            "extrapolated_value": extrapolated,
            "medium_gci_absolute": gci_m,
            "fine_gci_absolute": gci_f,
            "asymptotic_ratio": gci_m / (gci_f * r_mf**order),
        }
        for field, target in expected.items():
            actual = float(row[field])
            require(
                math.isclose(actual, target, rel_tol=1e-10, abs_tol=1e-12),
                f"triplet {index} field {field}: {actual} != {target}",
            )
        require(math.isclose(order, 2.0, rel_tol=1e-10), "known order must be two")
        require(math.isclose(extrapolated, 2.0, rel_tol=1e-10), "limit must be two")
        require(
            math.isclose(expected["asymptotic_ratio"], 1.0, rel_tol=1e-10),
            "quadratic family must be asymptotic",
        )
    print("independent_gci_oracle=pass grids=6 triplets=4 order=2 extrapolated_limit=2 asymptotic_ratio=1")
    return 0


if __name__ == "__main__":
    try:
        raise SystemExit(main())
    except (ValueError, OSError, KeyError) as error:
        print(f"independent_gci_oracle=fail reason={error}", file=sys.stderr)
        raise SystemExit(1)
