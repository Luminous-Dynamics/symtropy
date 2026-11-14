# Black-hole source–observer input/output audit

A small standalone Rust reference crate for the finite-dimensional transfer map
that connects an operator pencil to a specified source and observation channel.

This crate is an **input/output control**, not a black-hole perturbation solver.
It complements the Gram-weighted resolvent reference in
`tools/bh-spectral-norm-audit`, and supplies known-answer controls for
Symtropy issues #4138, #4109, #4154 and #4191.

## Mathematical contract

For a frequency-domain pencil L, source map B, observation map C, and
direct/feedthrough map D, define

    H = D + C L^(-1) B.

The implementation solves L X = B and evaluates H = D + C X. It does not
explicitly construct L^(-1).

For a first-order perturbation of all four maps,

    δH = δD + δC G B - C G δL G B + C G δB,
    G = L^(-1).

The implementation evaluates this differential using linear solves. It keeps
operator, source, observation, and direct-channel changes separate.

For a state-space realization L(s) = s I - A, changing B or C changes the
transfer amplitudes / residues without changing the pole set of L. Changing A
changes the operator pencil and may move its poles. A matched internal state
transformation

    A' = S A S^(-1),  B' = S B,  C' = C S^(-1)

must leave H invariant. This is a representation invariance, not a physical
change to the black-hole background.

## Known-answer fixture

The two-state model

    A = diag(-1, -2),  B = [1, 1]^T,  C = [1, 1],  D = 0

has

    H(s) = 1/(s+1) + 1/(s+2).

Changing B to [1, 1/4]^T changes the second modal residue to 1/4 while
leaving L(s) unchanged. Changing C to [1, 1/2] changes the second observed
residue to 1/2 while leaving L(s) unchanged. Changing A changes L(s) and
its pole locations.

## Diagnostics and limits

Each frequency response returns the complex transfer matrix, the smallest
singular value of L, and its 2-norm condition estimate. Near-singular cases
are not silently regularized: callers receive the conditioning information
needed to abstain when a requested tolerance is unsupported.

The crate validates matrix dimensions and finite inputs. It does not select a
universal conditioning threshold, infer a physical energy norm, or map an
arbitrary matrix perturbation to a physically admissible metric perturbation.

## Run

    cargo fmt --manifest-path tools/bh-input-output-audit/Cargo.toml -- --check
    cargo clippy --manifest-path tools/bh-input-output-audit/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path tools/bh-input-output-audit/Cargo.toml --all-targets

## Claim ceiling

Passing these tests validates only the declared finite-dimensional transfer
and perturbation identities. It does not reproduce a Pöschl–Teller or Kerr
Green function, compute gravitational-wave observables, establish a causal
boundary model, or demonstrate detector distinguishability. Those require
independent continuum and time-domain qualification in #4109, #4154 and #4052.
