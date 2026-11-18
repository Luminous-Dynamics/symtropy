#!/usr/bin/env python3
"""Independent symbolic oracle for the 5D equal-spin Myers-Perry control.

Research-only SymPy program. It is deliberately independent from the Rust
symtropy-lovelock5d crate and does not implement a production field solver.

Source conventions:
- Brihaye, Kleihaus, Kunz & Radu, JHEP 11 (2010) 089, arXiv:1010.0860.
- Agurto-Sepulveda et al., arXiv:2607.07418 (2026), Eq. (4) ansatz and
  five-dimensional EGB control family.

The radial coordinate is the areal-base gauge g(r)=r^2, not the
Boyer-Lindquist radial coordinate of the canonical Myers-Perry form.
"""

from __future__ import annotations

import json
import sympy as sp

r, r_h, omega, alpha = sp.symbols(
    "r r_H Omega_H alpha", positive=True, finite=True
)
q = r_h**2 * omega**2
u = (r_h / r) ** 2
denom = 1 - q + q * u**2

# Exact five-dimensional equal-spin Myers-Perry metric functions in g=r^2 gauge.
b = sp.factor(1 - u / denom)
f = sp.factor((1 - u) * (1 - q * (1 + u)) / (1 - q))
g = r**2
h = sp.factor(r**2 + r_h**2 * q * u / (1 - q))
w = sp.factor(omega * u**2 / denom)

mu = r_h**2 / (1 - q)
a = r_h**2 * omega
U = -r_h**2 / (1 - q)
V = r_h**6 * omega**2 / (1 - q)
W = r_h**4 * omega / (1 - q)

bp = sp.factor(sp.diff(b, r))
fp = sp.factor(sp.diff(f, r))
hp = sp.factor(sp.diff(h, r))
wp = sp.factor(sp.diff(w, r))

# Independent closed derivative formulas.
bp_expected = 2 * u * (1 - q - q * u**2) / (r * denom**2)
fp_expected = 2 * u * (1 - 2 * q * u) / (r * (1 - q))
hp_expected = 2 * r - 2 * r_h**2 * q * u / ((1 - q) * r)
wp_expected = -4 * omega * u**2 * (1 - q) / (r * denom**2)

def zero(name: str, expression: sp.Expr) -> None:
    reduced = sp.factor(sp.cancel(expression))
    assert reduced == 0, f"{name} failed: {reduced}"

def record(name: str, expression: sp.Expr) -> str:
    return str(sp.factor(expression))

# Exact metric identities.
zero("f conventional MP form", f - (1 - mu / r**2 + mu * a**2 / r**4))
zero("h conventional MP form", h - (r**2 + mu * a**2 / r**2))
zero("b from completed-square g_tt", b - (1 - mu / r**2 + h * w**2))
zero("b derivative", bp - bp_expected)
zero("f derivative", fp - fp_expected)
zero("h derivative", hp - hp_expected)
zero("w derivative", wp - wp_expected)
zero("f/b exact ratio", f / b - denom / (1 - q))

# Horizon values and nonextremal expansion coefficients.
horizon_values = {
    "b_H": sp.factor(b.subs(r, r_h)),
    "f_H": sp.factor(f.subs(r, r_h)),
    "h_H": sp.factor(h.subs(r, r_h)),
    "w_H": sp.factor(w.subs(r, r_h)),
}
zero("b_H", horizon_values["b_H"])
zero("f_H", horizon_values["f_H"])
zero("w_H-Omega_H", horizon_values["w_H"] - omega)
zero("h_H", horizon_values["h_H"] - r_h**2 / (1 - q))

b1 = sp.factor(bp.subs(r, r_h))
f1 = sp.factor(fp.subs(r, r_h))
w1 = sp.factor(wp.subs(r, r_h))
zero("b1 horizon expansion", b1 - 2 * (1 - 2 * q) / r_h)
zero("f1 horizon expansion", f1 - 2 * (1 - 2 * q) / (r_h * (1 - q)))
zero("w1 horizon expansion", w1 - 4 * omega * (q - 1) / r_h)

# Asymptotic coefficients and the exact MP identity V=-W^2/U.
zero("f asymptotic exact coefficients", f - (1 + U / r**2 + V / r**4))
zero("h asymptotic exact coefficients", h - (r**2 + V / r**2))
zero("asymptotic V=-W^2/U", V + W**2 / U)
assert sp.limit(r**2 * (b - 1), r, sp.oo) == U
assert sp.limit(r**4 * w, r, sp.oo) == W

# 2010 Eq. (2.12): angular Noether flux in the general g(r) gauge.
p_w_general = wp * h * sp.sqrt(f * h / b) * (
    g - alpha / 4 * (f * sp.diff(g, r)**2 / g - 16 + 12 * h / g)
)
p_w_gauge_factor = sp.factor(
    g - alpha / 4 * (f * sp.diff(g, r)**2 / g - 16 + 12 * h / g)
)
zero(
    "g=r^2 flux bracket",
    p_w_gauge_factor - (r**2 - alpha * (f - 4 + 3 * h / r**2))
)

# At alpha=0 the exact MP flux is independent of r. Work with its square
# to avoid asking a CAS to choose a square-root branch; w'<0 for Omega_H>0.
p_w_mp_squared = sp.factor(wp**2 * h**2 * (f * h / b) * r**4)
zero("MP angular flux square", p_w_mp_squared - 16 * W**2)

# The horizon limit of the flux has the same magnitude. Since Omega_H>0,
# q<1/2 implies w1<0 and hence P_w=-4W, not +4W.
horizon_flux_squared = sp.factor(w1**2 * (r_h**2 / (1 - q))**2
                                   * (f1 * (r_h**2 / (1 - q)) / b1)
                                   * r_h**4)
zero("horizon flux square", horizon_flux_squared - 16 * W**2)

# Brihaye et al. (2010), Eq. (2.14), gives a source-balance identity.
# The full radial current includes alpha-dependent terms; it is not constant
# at finite alpha. At alpha=0 it reduces to the Einstein current 2*U.
time_cb = f - 4 + h / r**2 + r * f * sp.diff(h, r) / h
time_cw = 4 - 3 * h / r**2 - f
p_t_full = sp.factor(
    sp.sqrt(f * h / b) * (
        r**2 * (h * w * wp - bp)
        + alpha * (
            time_cb * bp
            + time_cw * h * w * wp
            + r * f * h * wp**2
        )
    )
)
p_t_mp_alpha0 = sp.factor(p_t_full.subs(alpha, 0))
p_t_mp_squared = sp.factor(p_t_mp_alpha0**2)
zero("MP alpha-zero time-sector current square", p_t_mp_squared - 4 * U**2)
horizon_p_t_squared = sp.factor(
    (r_h**2 * sp.sqrt(f1 * (r_h**2 / (1 - q)) / b1)
     * ((r_h**2 / (1 - q)) * omega * w1 - b1))**2
)
zero("MP alpha-zero horizon time-sector current square", horizon_p_t_squared - 4 * U**2)

# EGB bracket is kept symbolic. The quadrature w'=P_w/(h sqrt(fh/b) K)
# is valid only after K is enclosed away from zero by an independent verifier.
K = sp.factor(r**2 - alpha * (f - 4 + 3 * h / r**2))
p_w_gauge = sp.factor(
    wp * h * sp.sqrt(f * h / b) * K
)

# Exact static Boulware-Deser Einstein branch, rationalized to avoid cancellation.
bd_amplitude = alpha + 2 * r_h**2
bd_radical = sp.sqrt(1 + alpha * bd_amplitude / r**4)
bd_lapse = 1 - bd_amplitude / (r**2 * (1 + bd_radical))
bd_original = 1 + r**2 / alpha * (1 - bd_radical)
zero("Boulware-Deser rationalization", bd_lapse - bd_original)

# Independent analytic derivatives and the static 5D Gauss-Bonnet scalar.
bd_lapse_prime = 2 * bd_amplitude / (
    r**3 * bd_radical * (1 + bd_radical)
)
bd_radical_prime = -2 * alpha * bd_amplitude / (bd_radical * r**5)
bd_lapse_second = sp.factor(
    bd_lapse_prime * (
        -3 / r
        - bd_radical_prime * (1 + 2 * bd_radical)
        / (bd_radical * (1 + bd_radical))
    )
)
bd_gauss_bonnet = sp.factor(
    12 / r**3 * (
        (bd_lapse - 1) * (r * bd_lapse_second + 2 * bd_lapse_prime)
        + r * bd_lapse_prime**2
    )
)
zero("Boulware-Deser first derivative", bd_lapse_prime - sp.diff(bd_lapse, r))
zero("Boulware-Deser second derivative", bd_lapse_second - sp.diff(bd_lapse, r, 2))

# Full Eq. (2.14) current on static BD: h=r^2, w=0, b=f=F.
bd_time_current = sp.factor(
    -r**3 * bd_lapse_prime
    + 3 * alpha * r * (bd_lapse - 1) * bd_lapse_prime
)
zero(
    "Boulware-Deser time-current source balance",
    sp.diff(bd_time_current, r) - alpha * r**3 * bd_gauss_bonnet / 6
)
bd_U = -(r_h**2 + alpha / 2)
zero(
    "Boulware-Deser asymptotic time-current endpoint",
    sp.limit(bd_time_current, r, sp.oo) - 2 * bd_U
)
assert sp.factor(sp.diff(bd_time_current, r).subs(alpha, 0)) == 0
# At r=r_H the square root is the positive branch 1+alpha/r_H^2.
# Prove the radicand identity and then substitute that positive root exactly.
bd_horizon_radical_square = sp.factor(
    (1 + alpha * (alpha + 2 * r_h**2) / r_h**4)
    - (1 + alpha / r_h**2)**2
)
zero("Boulware-Deser horizon radical identity", bd_horizon_radical_square)
bd_horizon_value = sp.factor(
    1 - (alpha + 2 * r_h**2)
    / (r_h**2 * (1 + (1 + alpha / r_h**2)))
)
zero("Boulware-Deser horizon", bd_horizon_value)
zero("Boulware-Deser Einstein limit",
     sp.limit(bd_lapse, alpha, 0, dir="+") - (1 - r_h**2 / r**2))

# Exact Akribeia control: r_H=1, Omega_H=33/100.
numeric_subs = {r_h: sp.Integer(1), omega: sp.Rational(33, 100)}
sample_radius = sp.Integer(3)
sample_subs = {**numeric_subs, r: sample_radius}
q_num = sp.factor(q.subs(numeric_subs))
U_num = sp.factor(U.subs(numeric_subs))
V_num = sp.factor(V.subs(numeric_subs))
W_num = sp.factor(W.subs(numeric_subs))

sample = {
    "radius": "3",
    "b": record("b3", b.subs(sample_subs)),
    "f": record("f3", f.subs(sample_subs)),
    "g": record("g3", g.subs(sample_subs)),
    "h": record("h3", h.subs(sample_subs)),
    "w": record("w3", w.subs(sample_subs)),
    "b_prime": record("bp3", bp.subs(sample_subs)),
    "f_prime": record("fp3", fp.subs(sample_subs)),
    "h_prime": record("hp3", hp.subs(sample_subs)),
    "w_prime": record("wp3", wp.subs(sample_subs)),
}

assert q_num == sp.Rational(1089, 10000)
assert U_num == sp.Rational(-10000, 8911)
assert V_num == sp.Rational(1089, 8911)
assert W_num == sp.Rational(3300, 8911)
assert sp.factor(b1.subs(numeric_subs)) == sp.Rational(3911, 2500)
assert sp.factor(f1.subs(numeric_subs)) == sp.Rational(15644, 8911)
assert sp.factor(horizon_values["h_H"].subs(numeric_subs)) == sp.Rational(10000, 8911)
assert sp.factor(w1.subs(numeric_subs)) == sp.Rational(-294063, 250000)

# q<1/2 makes both exterior factors positive for 0<u<1:
# b=(1-u)(1-q(1+u))/D and f=(1-u)(1-q(1+u))/(1-q).
# This is the exact static-exterior domain of the nonextremal MP control.
factorization = {
    "b": "(1-u)*(1-q*(1+u))/(1-q+q*u^2)",
    "f": "(1-u)*(1-q*(1+u))/(1-q)",
    "h": "r^2+r_H^2*q*u/(1-q)",
    "w": "Omega_H*u^2/(1-q+q*u^2)",
    "f_over_b": "(1-q+q*u^2)/(1-q)",
    "domain": "0<=q<1/2 and 0<u<=1"
}

fixture = {
    "schema_version": 1,
    "artifact_kind": "independent_symbolic_5d_equal_spin_myers_perry_reference",
    "sympy_version": sp.__version__,
    "sources": [
        "Brihaye et al., JHEP 11 (2010) 089, arXiv:1010.0860, Eqs. (2.8)-(2.19), (2.12), (3.6), (3.8)",
        "Agurto-Sepulveda et al., arXiv:2607.07418 (2026), 5D EGB equal-spin benchmark"
    ],
    "conventions": {
        "dimension": 5,
        "signature": "(-++++)",
        "gauge": "g(r)=r^2",
        "coordinates": ["t", "r", "theta", "phi_1", "phi_2"],
        "action": "I=(16*pi*G)^-1 integral sqrt(-g)*(R+alpha*L_GB/4)",
        "coupling": "alpha=4*alpha_2/alpha_1",
        "metric_ansatz": "ds^2=-b dt^2+dr^2/f+g dtheta^2+h sin(theta)^2(dphi_1-w dt)^2+h cos(theta)^2(dphi_2-w dt)^2"
    },
    "definitions": {
        "q": "r_H^2*Omega_H^2",
        "u": "(r_H/r)^2",
        "D": "1-q+q*u^2",
        "mu": "r_H^2/(1-q)",
        "a": "r_H^2*Omega_H"
    },
    "myers_perry_metric_functions": {
        "b": record("b", b),
        "f": record("f", f),
        "g": record("g", g),
        "h": record("h", h),
        "w": record("w", w),
        "derivatives": {
            "b_prime": record("b_prime", bp),
            "f_prime": record("f_prime", fp),
            "g_prime": record("g_prime", sp.diff(g, r)),
            "h_prime": record("h_prime", hp),
            "w_prime": record("w_prime", wp)
        },
        "factorized_forms": factorization
    },
    "horizon_data": {
        "b_H": "0",
        "f_H": "0",
        "w_H": "Omega_H",
        "h_H": record("h_H", horizon_values["h_H"]),
        "b1": record("b1", b1),
        "f1": record("f1", f1),
        "w1": record("w1", w1),
        "nonextremal_domain": "0<=q<1/2"
    },
    "asymptotic_parameters": {
        "U": record("U", U),
        "V": record("V", V),
        "W": record("W", W),
        "identity": "V=-W^2/U",
        "mass_parameter": "M=r_H^2/(2*(1-q))",
        "physical_energy": "E=-3*(2*pi^2)*U/(16*pi*G)",
        "angular_momentum_per_plane": "J=(2*pi^2)*W/(8*pi*G)"
    },
    "noether_first_integral": {
        "source_equation": "w'*h*sqrt(f*h/b)*(g-alpha/4*(f*g'^2/g-16+12*h/g))=constant",
        "gauge_fixed": "P_w=w'*h*sqrt(f*h/b)*(r^2-alpha*(f-4+3*h/r^2))",
        "asymptotic_relation": "P_w=-4*W",
        "alpha_zero_mp_flux": "-4*Omega_H*r_H^4/(1-q)",
        "quadrature_gate": "K(r)=r^2-alpha*(f-4+3*h/r^2) must be enclosed away from zero before division"
    },
    "time_sector_radial_identity": {
        "source_equation": "R_t^t+alpha/4*(H_t^t+L_GB/2)=(1/(2*r^2))*sqrt(f/(b*h))*d/dr[P_t(r)]",
        "full_current": "sqrt(f*h/b)*(r^2*(h*w*w'-b')+alpha*((f-4+h/r^2+r*f*h'/h)*b'+(4-3*h/r^2-f)*h*w*w'+r*f*h*w'^2))",
        "on_shell_source_balance": "dP_t/dr=(alpha/6)*r^2*sqrt(b*h/f)*L_GB",
        "asymptotic_endpoint": "P_t(infinity)=2*U",
        "alpha_zero_mp_limit": "P_t(r)=2*U; the source vanishes only at alpha=0",
        "finite_alpha_status": "not conserved; varies with the Gauss-Bonnet source",
        "boulware_deser_control": "P_t=-r^3*F'+3*alpha*r*(F-1)*F'; P_t'=alpha*r^3*L_GB/6",
        "use": "source-balance residual and optional b' reconstruction with the complete alpha-dependent coefficient"
    },
    "boulware_deser_control": {
        "b_equals_f": "1+r^2/alpha*(1-sqrt(1+alpha*(alpha+2*r_H^2)/r^4))",
        "h": "r^2",
        "w": "0",
        "einstein_limit": "1-r_H^2/r^2"
    },
    "akribeia_control_rH1_Omega033": {
        "q": str(q_num),
        "mu": record("mu", mu.subs(numeric_subs)),
        "M": record("M", (mu/2).subs(numeric_subs)),
        "b1": record("b1_num", b1.subs(numeric_subs)),
        "f1": record("f1_num", f1.subs(numeric_subs)),
        "h_H": record("h_H_num", horizon_values["h_H"].subs(numeric_subs)),
        "w1": record("w1_num", w1.subs(numeric_subs)),
        "U": str(U_num),
        "V": str(V_num),
        "W": str(W_num),
        "P_w": str(-4*W_num),
        "sample_r3": sample
    },
    "symbolic_checks": {
        "metric_reconstruction": "passed",
        "all_five_metric_functions": "passed",
        "first_derivatives": "passed",
        "horizon_values": "passed",
        "horizon_derivatives": "passed",
        "asymptotic_coefficients": "passed",
        "V_equals_minus_W_squared_over_U": "passed",
        "alpha_zero_noether_flux_squared": "passed",
        "horizon_flux_magnitude": "passed",
        "alpha_zero_time_sector_flux_squared": "passed",
        "horizon_time_sector_flux_squared": "passed",
        "boulware_deser_time_current_source_balance": "passed",
        "boulware_deser_time_current_asymptotic_endpoint": "passed",
        "boulware_deser_rationalization": "passed",
        "boulware_deser_horizon_and_einstein_limit": "passed"
    },
    "claim_ceiling": "Exact analytic reference and symbolic checks only. No nonzero-alpha rotating EGB solution, Rust test execution, interval enclosure, radii-polynomial proof or stability claim."
}

print(json.dumps(fixture, indent=2, sort_keys=True))
