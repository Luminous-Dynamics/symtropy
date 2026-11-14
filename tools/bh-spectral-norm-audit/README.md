# Black-hole spectral norm audit (small-matrix reference)

This is a standalone Rust reference crate for the finite-dimensional Gram-weighted
resolvent calculation used in non-normal pseudospectral work.

It is deliberately **not** a Kerr / Teukolsky solver. Its purpose is to make
the easy-to-get-wrong linear-algebra layer executable and independently
testable before a large two-field hyperboloidal operator is implemented.

## Mathematical contract

Let A be a complex square operator, z a complex frequency, and G a Hermitian
positive-definite Gram matrix defining the declared inner product. Factor

    G = L L*

and define Q = L*. Since Q*Q = G, the induced operator norm is

    ||R||_G = ||Q R Q^-1||_2.

For the resolvent R = (A-zI)^-1,

    ||(A-zI)^-1||_G = 1 / sigma_min(Q (A-zI) Q^-1).

The implementation computes the log10 resolvent norm from the smallest
singular value of the transformed shifted operator. It does not explicitly
form the inverse of A-zI.

The alternative placement

    Q^-1 (A-zI) Q

computes a different dual-metric quantity. For non-normal operators it is not
an innocuous convention change. The API requires the caller to select
NormPlacement::Gram or NormPlacement::Dual explicitly.

A Cholesky factor is used instead of the principal Hermitian square root.
This is equivalent for singular values: if Q1*Q1 = Q2*Q2 = G, then
Q2 = U Q1 for a unitary U, and the transformed matrices are related by
unitary similarity.

## Controls

The unit suite covers:

- identity Gram matrix against the Euclidean norm;
- a fixed non-normal 2x2 matrix where Gram and dual placements give
  independently calculated, different log-resolvent values;
- state similarity with the Gram matrix transformed consistently;
- rejection of the same state similarity with the old Gram matrix retained;
- rejection of non-Hermitian and indefinite Gram matrices;
- empty and dimension-mismatched inputs;
- square-root pseudospectral scaling for a size-two Jordan / EP normal form;
- linear pseudospectral scaling for a simple non-EP control;
- square-root eigenvalue branch exchange after one parameter-loop winding and return after two.

The fixed 2x2 reference uses

    A = [[-1, 4], [0, -2]]
    z = -0.8 + 0.7 i
    G = diag(4, 0.25)

and gives log10 ||(A-zI)^-1||_G = 1.2012767555437331 and the
dual-placement value 0.243102895186264.

## Run

    cargo test --manifest-path tools/bh-spectral-norm-audit/Cargo.toml
    cargo clippy --manifest-path tools/bh-spectral-norm-audit/Cargo.toml --all-targets -- -D warnings
    cargo fmt --manifest-path tools/bh-spectral-norm-audit/Cargo.toml -- --check

## Claim boundary

Passing these tests validates only this finite-dimensional linear-algebra
primitive. It does not validate:

- the Kerr Teukolsky operator or its two-field hierarchy;
- the physical derivation of a proposed Gram / energy inner product;
- continuum convergence of any pseudospectrum;
- the physical energy represented by a Sobolev norm;
- a source-specific waveform or detector-visible instability.

Those are separate scientific gates tracked in Symtropy #4166 and #4138.
