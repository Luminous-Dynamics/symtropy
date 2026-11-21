#!/usr/bin/env python3
"""Exact radial-reparameterization Noether identity for the 5D EGB 1D action."""
from __future__ import annotations

import json
import sympy as sp

r = sp.Symbol("r", positive=True, finite=True)
b, f, g, h, w = sp.symbols("b f g h w", positive=True, finite=True)
bp, fp, gp, hp, wp = sp.symbols("b_p f_p g_p h_p w_p", real=True)
bpp, fpp, gpp, hpp, wpp = sp.symbols("b_pp f_pp g_pp h_pp w_pp", real=True)
alpha = sp.Symbol("alpha", real=True)

zero_order = (b, f, g, h, w)
first_order = (bp, fp, gp, hp, wp)
second_order = (bpp, fpp, gpp, hpp, wpp)


def total_derivative(expression):
    return (
        sum(sp.diff(expression, q) * q_prime
            for q, q_prime in zip(zero_order, first_order))
        + sum(sp.diff(expression, q_prime) * q_second
              for q_prime, q_second in zip(first_order, second_order))
    )


measure = sp.sqrt(f*h/b)
L_einstein = measure * (
    bp*gp + g*bp*hp/(2*h) + b*gp**2/(2*g) + b*gp*hp/h
    + g*h*wp**2/2 + 2*b/f*(4 - h/g)
)
L_gauss_bonnet = measure/g * (
    4*h*bp*gp/g + 2*(4*g - 3*h)*(bp*hp/h + h*wp**2)
    - f*bp*hp*gp**2/(2*h) - f*h*gp**2*wp**2/2
)
L_eff = L_einstein + alpha*L_gauss_bonnet/4

momenta = {
    name: sp.diff(L_eff, q_prime)
    for name, q_prime in zip(("b", "f", "g", "h", "w"), first_order)
}
equations = {
    name: sp.diff(L_eff, q) - total_derivative(momenta[name])
    for name, q in zip(("b", "f", "g", "h", "w"), zero_order)
}

# The radial lapse f transforms with weight -2, giving 2(f E_f)'.
E_f_prime = total_derivative(equations["f"])
identity = (
    bp*equations["b"] + fp*equations["f"] + gp*equations["g"]
    + hp*equations["h"] + wp*equations["w"]
    + 2*(fp*equations["f"] + f*E_f_prime)
)
if sp.simplify(sp.factor(identity)) != 0:
    raise AssertionError("radial Noether identity did not reduce to exact zero")

print(json.dumps({
    "schema_version": 1,
    "identity": "b'*E_b+f'*E_f+g'*E_g+h'*E_h+w'*E_w+2*(f*E_f)'=0",
    "exact_symbolic_zero": True,
    "action": "L_Einstein + alpha*L_Gauss_Bonnet/4",
    "gauge_imposed_after_variation": True,
    "interpretation": "E_g is a propagated radial constraint when the other Euler-Lagrange equations hold and g'=2r != 0",
    "claim_ceiling": "This verifies the action-level Noether dependency. It does not prove Fuchsian regularity or boundary-value well-posedness."
}, indent=2, sort_keys=True))
