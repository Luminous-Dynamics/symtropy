# Interval Oracle and Conformance Boundary v0.1

**Tracking:** [PHYS-UQ-CONFORMANCE-001 #2348](https://github.com/Luminous-Dynamics/symtropy/issues/2348)  
**Implementation branch:** `research/physical-interval-oracle-v0-1`  
**Scope:** arithmetic differential oracle for the bounded-input propagation module

## Research and pinned references

1. **Independent Rust comparator:** `inari = 2.0.0`, pinned exactly as a development dependency with default features disabled. Its published crate declares the MIT license; the exact checksum is recorded in `research/physical-analytical-adapter/Cargo.lock`. Its own documentation says it conforms to IEEE 1788.1-2017 and implements a subset of IEEE 1788-2015. It is an independent comparator, not presumed infallible ground truth.
2. **Public conformance-test framework:** [ITF1788](https://github.com/unageek/ITF1788), pinned reference commit `d8c2a64478ebdc9cbde6ccef33eaad3bed60ed81`. Its Rust `inari` plugin configuration and test template identify the operation and exception categories the public suite can exercise. The repository identifies its test framework and template as Apache-2.0 licensed. This v0.1 slice does not claim to execute the entire ITF1788 corpus; it starts with a Rust-native finite arithmetic subset to avoid misrepresenting unsupported interval models or decorations as equivalent.
3. **Test methodology:** Benet, Ferranti and Revol, “A framework to test interval arithmetic libraries and their IEEE 1788-2015 compliance,” DOI [10.1002/cpe.7856](https://doi.org/10.1002/cpe.7856). Their discussion separates endpoint arithmetic from set-based interval semantics, unbounded/empty intervals, decorations and exception handling.
4. **Standards status snapshot (2026-10-20):** IEEE 1788-2015 is shown by IEEE as inactive-reserved; IEEE 1788.1-2017 is shown as active. This adapter does not claim either standard's full conformance.

## What is tested

The Rust-native oracle tests compare Symtropy's endpoint enclosure against Inari 2.0.0's independent result for:

- 64 pairings of ordinary, negative, zero-crossing, signed-zero, very small and broad finite intervals under multiplication;
- 40 pairings of numerator intervals with denominator intervals that exclude zero under division;
- positive integer powers built from repeated interval products;
- denominator intervals that include zero, which Symtropy explicitly rejects under its stricter physical-input contract;
- arithmetic overflow, which is deliberately rejected rather than emitting a non-finite enclosure.

For matched finite domains, the assertion is *containment*, not bitwise equality: Symtropy's lower endpoint must be no greater than the independent lower endpoint and its upper endpoint no less than the independent upper endpoint. Wider intervals can be sound but less useful, so interval width and overestimation remain relevant future metrics. Exact endpoints are not assumed to match across libraries.

The tests complement the physical-domain suite: corner combinations, seeded interior points, zero-crossing load cases, positive-denominator checks, formula known-answer tests, and receipt invariants. Those mechanics tests remain necessary because a low-level interval operation can be correct while an expression-level propagation function is wrong.

## Explicit semantic mapping and non-claims

Symtropy v0.1 accepts only finite, ordered common intervals and rejects any operation whose calculated enclosure becomes non-finite. The public IEEE interval standards also specify behaviours for models and cases—such as empty or unbounded intervals and decorations—that this input-bounds API intentionally does not expose. A rejection in this API is not evidence of an IEEE conformance failure when the operation lies outside the declared supported subset.

This is **not** a claim of IEEE 1788-2015 or IEEE 1788.1-2017 conformance. It is a bounded differential check over named operations and inputs against a pinned independent library. It also does not validate any structural model or physical measurement.

## Acceptance targets

- Zero known false enclosures across the frozen matched-domain test vectors.
- Every rejected input has an explicit, testable failure reason.
- Keep every discovered discrepancy as a minimized permanent regression test.
- Run the same fixture set on pinned toolchains and declared CPU architectures before making a cross-platform statement.
- Later extend coverage to more ITF1788 test categories only when the interval-model, decoration and exception semantics are explicitly mapped.
