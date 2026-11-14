# Black-hole source-spectrum audit

A small standalone Rust reference crate for the analytic spatial spectrum of a
localized Gaussian-carrier initial perturbation, and for composing that source
overlap with a declared mode gain.

This is the first executable slice of Symtropy #4209. It supports research on
source-shaped QNM excitation; it is **not** a black-hole QNM solver and its
Gaussian Fourier transform is not, by itself, a physical QNM excitation
coefficient.

## Frozen source convention

Use the Fourier convention

    f_hat(k) = integral f(x) exp(-i k x) dx

for

    f(x) = A exp(-(x-x0)^2 / (2 sigma^2)) cos(kc (x-x0)).

The exact analytic transform, including its entire continuation to complex k,
is

    f_hat(k) = A sqrt(2 pi) sigma / 2 exp(-i k x0)
               * [ exp(-sigma^2 (k-kc)^2 / 2)
                 + exp(-sigma^2 (k+kc)^2 / 2) ].

The exact squared L2 norm is

    integral |f(x)|^2 dx
      = A^2 sqrt(pi) sigma / 2 * [1 + exp(-sigma^2 kc^2)].

The two formulas make an important experimental distinction explicit:
holding peak amplitude A fixed while changing sigma does not hold integrated
source strength fixed. Use the declared L2 convention when comparing
bandwidth interventions at equal source norm.

## API

- GaussianCarrier::new validates finite parameters and positive width.
- sample evaluates the real spatial profile.
- fourier_transform evaluates the exact transform for real or complex k.
- l2_norm_squared evaluates the exact integrated squared source norm.
- evaluate_mode_overlap multiplies the source transform by an explicitly
  supplied intrinsic mode gain, returning the factors separately.

No QNM frequencies, intrinsic gains, physical energy norm, detector map, or
black-hole potential are inferred by the crate. Those must come from pinned
physical calculations and the #4109 / #4154 response owners.

## Run

    cargo fmt --manifest-path tools/bh-source-spectrum-audit/Cargo.toml -- --check
    cargo clippy --manifest-path tools/bh-source-spectrum-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/bh-source-spectrum-audit/Cargo.toml --all-targets

## Claim ceiling

Passing these tests validates a Gaussian Fourier oracle and reduced source /
mode composition only. It does not reproduce the 2026 source-spectral-filter
paper, compute Teukolsky excitation factors, validate QNMToolkit, or establish
a physical mode-excitation law for Kerr. Those require the pinned source
normalization, radial weighting, mode data and independent time-domain tests.
