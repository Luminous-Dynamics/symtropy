# Component qualification reference evaluator v0.1

Dependency-free Rust reference evaluator for [Symtropy #3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331).

## Run

```sh
cargo test --locked --manifest-path benchmarks/component-qualification/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/component-qualification/Cargo.toml --all-targets -- -D warnings
```

## V0 scope

- Component / serial / material-lot / design / specification identity binding.
- Exact datum, unit, measurement-method and calibration binding.
- Evidence currentness, revocation and dispute handling.
- Dimensional acceptance using the complete measured interval, including expanded uncertainty.
- Conservative outcomes: accepted only when the complete interval is inside tolerance; rejected only when fully outside; otherwise indeterminate.
- Assembly membership/revision checks with explicit child qualification outcomes.

The evaluator does not choose tolerance, design codes, product classes, inspection methods or NDT acceptance rules. Those are supplied by the governing project/product specification and competent authority. It only evaluates the narrow scalar interval contract.

## Evidence ownership

Manufacturer, laboratory, metrology, custody, certification and approval records remain external sources. This crate checks subject/revision references and states, but does not authenticate signatures, verify a certificate authority, or replace Mycelix provenance and approval receipts. An evidence ID is a reference, not proof of truth.

## Claim ceiling

All values are synthetic. This is not a structural design/acceptance system, product certification, NDT interpreter, manufacturing execution system, IFC validator, statutory inspection, or engineer's approval.
