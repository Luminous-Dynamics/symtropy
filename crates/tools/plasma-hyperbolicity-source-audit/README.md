# plasma-hyperbolicity-source-audit

A dependency-free Rust checker for a narrow but important source-admission invariant: the printed characteristic polynomial of an n×n matrix under the convention det(λI − A) must be monic of degree n, and its λ^(n−1) coefficient must equal −trace(A).

## Run

    cargo fmt --manifest-path crates/tools/plasma-hyperbolicity-source-audit/Cargo.toml -- --check
    cargo test --locked --manifest-path crates/tools/plasma-hyperbolicity-source-audit/Cargo.toml
    cargo clippy --locked --manifest-path crates/tools/plasma-hyperbolicity-source-audit/Cargo.toml --all-targets -- -D warnings

## Why this exists

Symtropy #6313 investigates the October 2026 Hermite–Laguerre drift-kinetic slab moment paper. In its P=Q=4 worked case, printed Eqs. (64)/(65) show two 5×5 matrices with zero diagonal, while Eqs. (66)/(67) show degree-6 characteristic-polynomial expressions with nonzero λ^4 terms. The same article's Eq. (57) says p_(i,j) has degree j, and Eq. (63) names p_(i,P+1), which gives degree 5 for P=4. The 2024 TU/e thesis predecessor states that each diagonal block is (P+1)×(P+1) and Theorem 7 uses He_(P+1).

The frozen source fixture records the printed matrix dimension, degree, trace and λ^4 coefficients, and this checker rejects both equation pairs.

This is a **source-consistency audit**, not a correction to the paper:
- it does not decide whether a matrix row/column is omitted or a polynomial exponent/index is mistyped;
- it does not establish what the authors intended;
- it does not reproduce the hyperbolicity region;
- it does not prove eigenvalue reality or diagonalizability;
- it does not validate a full tokamak / stellarator edge model.

Do not silently repair the source or treat the 2024 thesis as authority for what the 2026 journal article intended. Keep P=Q=4 known-answer fixtures blocked until the source matrix/polynomial pair is clarified.

## Frozen evidence

- Koellermeier, Mischenko & Proll (2026), DOI 10.1007/s10404-026-02946-4, journal Eqs. (57), (63)–(67), especially printed PDF p. 12 / journal p. 92:
  https://doi.org/10.1007/s10404-026-02946-4
- Mischenko (2024), TU/e master's thesis, Eq. (5.61) and Theorem 7 / Eq. (5.67):
  https://research.tue.nl/en/studentTheses/hyperbolicity-of-a-hermite-laguerre-moment-model-of-the-plasma-pe/

A passing audit only means the declared matrix dimension, monic degree and trace coefficient are mutually consistent within the declared tolerance. It does not authenticate the transcription or any source artifact.
