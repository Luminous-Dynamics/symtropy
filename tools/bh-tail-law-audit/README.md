# Black-hole tail law audit

Small standalone Rust reference for the **published spin-weighted Kerr sourced-tail
asymptotic branch selector** in:

Ling & Wong, *Nonlinear tails in the Kerr black hole ringdown*, Phys. Rev. D
114, 064092 (2026), DOI 10.1103/b44t-dlb7.

This is a typed equation-domain guard, not a Teukolsky solver or a physical
tail calculation.

## Transcribed asymptotic branches

For spin weight s != 0, angular mode (ell,m), and source radial falloff
r^(-beta), the paper reports:

- sourced tail: t^(-(ell + beta + s)) for beta <= 1 or beta >= ell - s + 2;
- Price tail: t^(-(2 ell + 3)) for 2 <= beta <= ell - s + 1.

The code returns an explicit UnsupportedBetaRegime error for parameter gaps.
It rejects s=0 because the paper's abstract separately describes a scalar
numerical demonstration; it does not establish that the spin-weighted formula
may be applied to s=0 by substitution.

## Tests

The nine tests cover:
- low-beta sourced branch;
- middle-beta Price branch;
- high-beta sourced branch;
- both uncovered beta intervals;
- inclusive endpoints at beta = 1, 2, ell-s+1, ell-s+2;
- empty Price interval for selected positive-spin modes;
- rejection of scalar s=0 formula substitution;
- invalid angular modes;
- non-finite beta.

## Run

    cargo fmt --manifest-path tools/bh-tail-law-audit/Cargo.toml -- --check
    cargo clippy --manifest-path tools/bh-tail-law-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/bh-tail-law-audit/Cargo.toml --all-targets

## Claim boundary

This only validates transcription and branch selection for the cited
spin-weighted asymptotic formula. It does not establish the paper's far-field
approximation, source construction, amplitude, scalar tail law, Kerr numerical
evolution, linear/nonlinear tail separation, or detector observability. Those
are covered by the physical reproduction gates in Symtropy #4172, #4109 and
#4052.
