# Complex special-function audit

This is a small Rust reference kernel for the Pöschl–Teller Jost-function
research in issue #4243.

## Implemented

### Complex Gamma

Lanczos approximation with reflection for Re(z) < 1/2. Exact non-positive
integer poles return a typed error. The public API accepts num-complex
Complex64 and returns Result rather than NaN/Inf sentinels.

### Complex-parameter Gauss hypergeometric function

The API accepts complex a, b, c and z. Three numerical regions are explicit:

1. Direct defining series for |z| < 0.85 (and terminating polynomials).
2. Pfaff transformation when z/(z-1) maps into the direct-series disk.
3. Non-resonant connection formula near z=1, using complex Gamma factors and
   two convergent series in 1-z.

The principal cut [1, infinity) is rejected. The near-one connection formula
is rejected when c-a-b is an integer (the logarithmic/resonant case). For
resonant points that remain inside |z|<1, the evaluator falls back to the
defining series; if the configured term budget cannot establish convergence,
it returns NoConvergence. Resonant points outside the direct-series disk still
require a separately validated logarithmic continuation or another transform.

The series result reports its method, term count and a global geometric
majorant for the analytic truncation tail. The majorant follows from bounds on
all future recurrence ratios once N>|c|. It is not an interval-arithmetic
certificate for the computed floating-point result and does not include
accumulated roundoff.

### Ferrers associated-Legendre function

The crate now evaluates P_l^mu(z) and its first two z-derivatives for real
z strictly inside (-1,1), using the formula

    P_l^mu(z) = exp(mu atanh(z)) / Gamma(1-mu)
                * 2F1(-l, l+1; 1-mu; (1-z)/2).

Complex degree l and order mu are supported. The implementation uses
contiguous ₂F₁ derivative identities, so the associated-Legendre ODE residual
can be tested independently from the differential-equation identity.

For the Pöschl–Teller tortoise coordinate x, set

    z = tanh(x/b)
    l = -1/2 + sqrt(1/4 - lambda)
    mu = i b omega

Then P_l^mu(z) has the right-outgoing asymptotic factor e^(i omega x) up to
its known 1/Gamma(1-mu) normalization. P_l^mu(-z) supplies the corresponding
left-outgoing basis. The crate also constructs the unit-normalized homogeneous Jost solutions
for the Pöschl–Teller equation

    u'' + [omega² - lambda/b² sech²(x/b)] u = 0.

With mu=i b omega:

    f_right(x) = Gamma(1-mu) P_l^mu(tanh(x/b))
    f_left(x)  = Gamma(1-mu) P_l^mu(-tanh(x/b)).

It returns the field and first/second tortoise derivatives, and evaluates the
Wronskian W[f_left,f_right] at a requested coordinate. Tests include the
free-potential plane-wave limit, the homogeneous ODE residual for both sides,
and position-independence of the barrier Wronskian. These are local analytic
Jost controls; the scattering matrix, transmission/reflection amplitudes,
frequency-plane pole search and Green-function contour are still not
implemented.

The endpoint values z=±1 and complex z arguments are rejected.
The unit-disk resonant fallback covers the pinned Pöschl–Teller Matsubara
fixtures n=0,1,2 at finite test positions x=±1,±2,±3. This is a bounded
parameter-domain result, not a universal continuation theorem; larger
arguments, branch-cut crossings and other resonant domains remain explicit
abstention cases until separately qualified.

### Independent Pöschl–Teller connection Wronskian

The unit-normalized Jost pair also admits an analytic connection-formula Wronskian,
independent of evaluating both Jost solutions and their derivatives at a finite
tortoise position. With ell=-1/2+sqrt(1/4-lambda) and mu=i b omega:

    W = -(2/b) Gamma(1-mu)^2
        / (Gamma(1-mu+ell) Gamma(-ell-mu)).

The implementation exposes this as a separate reference function and compares
it with the direct Jost Wronskian over multiple complex frequencies and
tortoise positions. It also checks W=2 i omega in the free-potential limit,
the principal-root pole labels on both sides of lambda=1/4, and the
second-order Wronskian zero at threshold. The threshold test classifies the
local zero order only; it does not infer an exceptional point without a
separate resolvent/eigenfunction analysis.

## Independent fixtures

Four complex fixtures were generated with mpmath 1.3.0 at 80 decimal digits
and then rounded into the test corpus:
- Gamma(0.3 + 1.2 i);
- Gamma(-1.2 + 0.7 i);
- complex-parameter 2F1 in the direct-series region;
- complex-parameter 2F1 in the near-one connection region;
- complex-parameter 2F1 through Pfaff continuation.

Additional tests exercise Gamma recurrence, reflection and conjugation;
hypergeometric a/b symmetry, polynomial termination, derivative identity,
branch-cut rejection, real- and complex-parameter 100-digit resonant ₂F₁
fixtures, insufficient-term-budget abstention, and the Pöschl–Teller Matsubara
Jost cases for n=0,1,2.

## Scientific claim boundary

Passing these tests is numerical-kernel evidence only. It does not validate
associated-Legendre Jost solutions, a Wronskian, a Green function, scattering
coefficients, QNM residues or any physical Kerr/Schwarzschild response.

Before integration into #4109, the next requirements are:
- validate the f64 truncation majorant against an independent interval or
  ball-arithmetic implementation, including accumulated floating-point error;
- test the full actual Jost parameter envelope (including high overtones and
  near-threshold/near-pole conditions);
- extend the independent high-precision fixture corpus over all evaluation
  regions and branch conventions;
- add logarithmic continuation for resonant c-a-b values outside |z|<1;
- add stable scaled/logarithmic coefficient paths for large complex arguments.

## Run

    cargo fmt --manifest-path tools/bh-complex-special-audit/Cargo.toml -- --check
    cargo clippy --manifest-path tools/bh-complex-special-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/bh-complex-special-audit/Cargo.toml --all-targets
