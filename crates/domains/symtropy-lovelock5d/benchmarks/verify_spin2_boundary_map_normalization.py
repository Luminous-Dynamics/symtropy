#!/usr/bin/env python3
"""Exact-rational metamorphic checks for boundary-map basis dependence.

Standard library only. This verifies coordinate covariance and metric-aware
conditioning identities; it does not calculate the finite-coupling J_1.
"""
from fractions import Fraction as F

def M(a, b, c, d):
    return ((F(a), F(b)), (F(c), F(d)))

def add(A, B):
    return tuple(tuple(A[i][j] + B[i][j] for j in range(2)) for i in range(2))

def scale(A, x):
    x = F(x)
    return tuple(tuple(x * A[i][j] for j in range(2)) for i in range(2))

def mul(A, B):
    return tuple(tuple(sum(A[i][k] * B[k][j] for k in range(2)) for j in range(2)) for i in range(2))

def tr(A):
    return A[0][0] + A[1][1]

def det(A):
    return A[0][0] * A[1][1] - A[0][1] * A[1][0]

def inv(A):
    d = det(A)
    assert d != 0
    return ((A[1][1] / d, -A[0][1] / d), (-A[1][0] / d, A[0][0] / d))

def transpose(A):
    return ((A[0][0], A[1][0]), (A[0][1], A[1][1]))

def generalized_eigen_invariants(J, Gp, Gc):
    # Eigenvalues of Gp^{-1} J^T Gc J are squared generalized singular values.
    K = mul(mul(inv(Gp), transpose(J)), mul(Gc, J))
    return tr(K), det(K)

J0 = M(F(1, 2), 0, 0, 2)
I = M(1, 0, 0, 1)

# Same differential map, different endpoint-amplitude coordinates.
R_identity = M(2, 0, 0, F(1, 2))
J_identity = mul(R_identity, J0)
assert J_identity == I

R_worse = M(1, 0, 0, 4)
J_worse = mul(R_worse, J0)
assert J_worse == M(F(1, 2), 0, 0, 8)
assert det(J_worse) == det(R_worse) * det(J0)

S = M(1, F(1, 3), F(1, 5), 1)
R = M(2, F(1, 4), F(-1, 3), 1)
J_transformed = mul(mul(R, J0), inv(S))
assert det(J_transformed) == det(R) * det(J0) / det(S)
assert det(J_transformed) != 0

# First-order transformation for parameter-dependent bases:
# J~_1 = R_1 J_0 S_0^-1 + R_0 J_1 S_0^-1
#        - R_0 J_0 S_0^-1 S_1 S_0^-1.
J1 = M(F(1, 3), F(2, 5), F(-1, 7), F(3, 4))
R0 = M(2, F(1, 5), F(1, 7), 1)
R1 = M(F(1, 4), 0, F(1, 6), F(-1, 3))
S0 = M(1, F(1, 4), 0, 2)
S1 = M(0, F(1, 5), F(-1, 7), F(1, 8))
S0_inv = inv(S0)
Jtilde0 = mul(mul(R0, J0), S0_inv)
Jtilde1_formula = add(
    add(mul(mul(R1, J0), S0_inv), mul(mul(R0, J1), S0_inv)),
    scale(mul(mul(mul(R0, J0), S0_inv), mul(S1, S0_inv)), -1),
)
R_series0, R_series1 = R0, R1
S_series0_inv = S0_inv
S_series1_inv = scale(mul(mul(S0_inv, S1), S0_inv), -1)
left0 = mul(R_series0, J0)
left1 = add(mul(R_series1, J0), mul(R_series0, J1))
product0 = mul(left0, S_series0_inv)
product1 = add(mul(left1, S_series0_inv), mul(left0, S_series1_inv))
assert product0 == Jtilde0
assert product1 == Jtilde1_formula

# Generalized singular values are invariant if metrics follow coordinates:
# Gp~ = S^-T Gp S^-1; Gc~ = R^-T Gc R^-1.
Gp = M(2, F(1, 5), F(1, 5), 3)
Gc = M(4, F(1, 7), F(1, 7), 2)
Gp_prime = mul(mul(transpose(inv(S)), Gp), inv(S))
Gc_prime = mul(mul(transpose(inv(R)), Gc), inv(R))
assert generalized_eigen_invariants(J0, Gp, Gc) == generalized_eigen_invariants(
    J_transformed, Gp_prime, Gc_prime
)

print("PASS: exact-rational boundary-map normalization metamorphic checks")
print("raw Euclidean condition numbers: 4 -> 1 and 4 -> 16 under output rescaling")
print(f"determinant covariance: det(J_transformed) = {det(J_transformed)}")
print("parameter-dependent endpoint-basis derivative transformation: PASS")
print("generalized singular-value invariants under metric-coordinate changes: PASS")
