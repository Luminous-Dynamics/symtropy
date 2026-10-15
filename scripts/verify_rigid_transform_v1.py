#!/usr/bin/env python3
"""Independent standard-library oracle for rigid-transform V1 fixtures."""
from __future__ import annotations

import math

TOL = 1.0e-12


def close(actual: list[float], expected: list[float]) -> None:
    for index, (left, right) in enumerate(zip(actual, expected)):
        assert abs(left - right) < TOL, (index, actual, expected)


def multiply(a: list[float], b: list[float]) -> list[float]:
    x1, y1, z1, w1 = a
    x2, y2, z2, w2 = b
    return [
        w1*x2 + x1*w2 + y1*z2 - z1*y2,
        w1*y2 - x1*z2 + y1*w2 + z1*x2,
        w1*z2 + x1*y2 - y1*x2 + z1*w2,
        w1*w2 - x1*x2 - y1*y2 - z1*z2,
    ]


def rotate(q: list[float], v: list[float]) -> list[float]:
    # Independent 3x3 rotation-matrix oracle, not the Rust cross-product path.
    x, y, z, w = q
    matrix = [
        [1.0 - 2.0*(y*y + z*z), 2.0*(x*y - z*w), 2.0*(x*z + y*w)],
        [2.0*(x*y + z*w), 1.0 - 2.0*(x*x + z*z), 2.0*(y*z - x*w)],
        [2.0*(x*z - y*w), 2.0*(y*z + x*w), 1.0 - 2.0*(x*x + y*y)],
    ]
    return [sum(matrix[row][col] * v[col] for col in range(3)) for row in range(3)]


def add(a: list[float], b: list[float]) -> list[float]:
    return [a[i] + b[i] for i in range(3)]


def main() -> None:
    s = math.sqrt(0.5)
    q_ab, t_ab = [0.0, 0.0, s, s], [1.0, 0.0, 0.0]
    q_bc, t_bc = [s, 0.0, 0.0, s], [0.0, 2.0, 0.0]
    q_ac = multiply(q_ab, q_bc)
    t_ac = add(rotate(q_ab, t_bc), t_ab)
    close(q_ac, [0.5, 0.5, 0.5, 0.5])
    close(t_ac, [-1.0, 0.0, 0.0])

    point_c = [1.0, 0.0, 0.0]
    point_b = add(rotate(q_bc, point_c), t_bc)
    point_a = add(rotate(q_ab, point_b), t_ab)
    close(point_a, [-1.0, 1.0, 0.0])
    close(rotate(q_ac, [1.0, 0.0, 0.0]), [0.0, 1.0, 0.0])

    wrong = multiply(q_bc, q_ab)
    close(wrong, [0.5, -0.5, 0.5, 0.5])
    assert any(abs(a - b) > 0.1 for a, b in zip(q_ac, wrong))

    q_inv = [-q_ab[0], -q_ab[1], -q_ab[2], q_ab[3]]
    test_b = [0.5, -1.0, 2.0]
    test_a = add(rotate(q_ab, test_b), t_ab)
    restored = rotate(q_inv, [test_a[i] - t_ab[i] for i in range(3)])
    close(restored, test_b)

    print("PASS: non-commuting XYZW composition and translation")
    print("PASS: point mapping and direction rotation-only semantics")
    print("PASS: reversed quaternion order is a distinct negative control")
    print("PASS: inverse point round-trip")


if __name__ == "__main__":
    main()
