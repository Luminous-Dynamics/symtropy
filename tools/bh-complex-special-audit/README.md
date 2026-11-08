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
is rejected when c-a-b is an integer (the logarithmic/resonant case), rather
than returning a potentially incorrect result. Inputs outside these regions
return ContinuationRegionUnsupported. These limits are intentional and should
be extended only with separately validated formulas.

The series result reports its method, term count and an a-posteriori geometric
tail estimate. That estimate is a diagnostic, not a rigorous global error
bound for arbitrary complex parameters.

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

The endpoint values z=±1 and complex z arguments are rejected. Resonant
denominator cases remain unsupported until the logarithmic continuation is
implemented.

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
branch-cut rejection and resonant abstention.

## Scientific claim boundary

Passing these tests is numerical-kernel evidence only. It does not validate
associated-Legendre Jost solutions, a Wronskian, a Green function, scattering
coefficients, QNM residues or any physical Kerr/Schwarzschild response.

Before integration into #4109, the next requirements are:
- compare with a second independent arbitrary-precision implementation;
- test the full actual Jost parameter envelope (including high overtones and
  near-threshold/near-pole conditions);
- verify the associated-Legendre ODE residual and Wronskian;
- add logarithmic continuation for resonant c-a-b values;
- add stable scaled/logarithmic coefficient paths for large complex arguments.

## Run

    cargo fmt --manifest-path tools/bh-complex-special-audit/Cargo.toml -- --check
    cargo clippy --manifest-path tools/bh-complex-special-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/bh-complex-special-audit/Cargo.toml --all-targets
