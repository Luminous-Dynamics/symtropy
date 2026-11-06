# Maritime manoeuvring contract v0

This dependency-free Rust crate implements the first small piece of
[MARITIME-MANOEUVRING-001](https://github.com/Luminous-Dynamics/symtropy/issues/3865).

It provides:

- context-of-use-specific evidence-role requirements bound to the exact frozen case;
- exact-case binding plus time-bounded manifest checks with a decision cutoff;
- deterministic NeedsEvidence / Unsupported outcomes;
- trajectory structural validation;
- independent geometric reconstruction of turning-circle advance, transfer and
  tactical diameter at 90° / 180° heading changes;
- interpolation of a declared stopping-speed threshold and track-relative
  displacement;
- 10°/10° and 20°/20° zig-zag first/second overshoot reconstruction from explicit
  rudder events and observed yaw-rate extrema.

It does not simulate ship hydrodynamics, calculate manoeuvring coefficients,
predict rudder/tug forces, or decide whether a ship can safely transit a channel.
The ReadyForComparison disposition means only that the manifest passed this
crate's structural, timing and role-admission checks. It is not Qualified,
a navigation clearance, a pilot's judgement, or statutory approval.

## Run

~~~sh
cargo fmt --manifest-path benchmarks/maritime-manoeuvring/Cargo.toml -- --check
cargo test --locked --manifest-path benchmarks/maritime-manoeuvring/Cargo.toml
cargo clippy --locked --manifest-path benchmarks/maritime-manoeuvring/Cargo.toml --all-targets -- -D warnings
~~~

The crate is an isolated Cargo workspace and has no third-party dependencies. The separate `tests/independent_oracle.rs` integration test implements test-only reference calculations without calling production metric helpers; it cross-checks turning-circle, stopping and zig-zag outputs on wrapped headings and nonuniform sample times.

## Metric conventions

Trajectory input is normalized into a declared local Cartesian frame:

- position in metres;
- +x east-like and +y north-like;
- heading in radians counter-clockwise from +x;
- time in seconds;
- surge/sway velocity in metres per second;
- yaw rate in radians per second.

The source frame, CRS/datum transformation, time basis, unit conversion and raw
artifact digest must be bound by the upstream run manifest. This crate does not
perform or authorize geospatial transformations.

Turning-circle metrics use the first non-zero heading-change direction and
interpolate the first 90° and 180° crossings. The input sampling interval must
be fine enough that heading changes between samples are unambiguous. Advance
and transfer are expressed in the initial-heading frame; tactical diameter is
the absolute lateral displacement at 180°.

Zig-zag metrics require exactly three alternating, equal-magnitude rudder commands. V0 requires command event times to align with trajectory samples. It verifies both target-heading crossings and requires a yaw-rate sign change to demonstrate that each overshoot peak was actually observed. Peak heading is interpolated at the yaw-rate zero crossing. The metric is not a pass/fail verdict against IMO criteria. The initial implementation supports the first two overshoots only; a complete trial adapter must preserve all source events and prove the third execute and acquisition interval.

Stopping metrics start at an explicit command sample and interpolate the first
crossing of a caller-declared speed threshold. The threshold is an analysis
parameter; it is not presented as an IMO-prescribed stopping definition.
Returned along-/cross-track quantities are signed displacements, not an
assumption about a universal stopping distance.

Route-clearance evaluation, uncertainty propagation, and specialist solver adapters
are intentionally not implemented in v0. The trajectory and evidence contract is
designed to receive those as later, separately qualified capabilities.


## Industry baselines

See [industry-baselines.md](industry-baselines.md) for the 2026 standards, peer-reviewed research comparators, matched optimizer protocol and service-level metrics. This note distinguishes study-specific author-reported gains from directly comparable benchmark results.

## Frozen test coverage

The current suite contains 25 hand-checkable synthetic controls for:

- right- and left-hand turning circles;
- heading wrap across 2π;
- missing turning thresholds and ambiguous heading increments;
- 10°/10° zig-zag first and second overshoots;
- invalid rudder sequence, early reversal, unobserved target/peak, unconfirmed zero-yaw plateau and non-aligned command-time controls;
- interpolated stopping threshold;
- stopping from a non-origin command sample;
- missing, stale, unknown, unsupported and future-published evidence;
- malformed digest, duplicate role and operation-window coverage failure;
- independent oracle comparisons for turning circles in both directions, rotated stopping tracks, and wrapped/irregular-time zig-zag trajectories.

These fixtures test bookkeeping and metric reconstruction only. They are not
physical validation data and do not establish model accuracy.

## Research and standards references

- PIANC WG 171, *Ship Handling Simulation Dedicated to Channel and Harbour
  Design* (September 2026): https://www.pianc.org/publication/ship-handling-simulation-dedicated-to-channel-and-harbour-design/
- ITTC 7.5-02-06-03, *Validation of Manoeuvring Simulation Models*,
  2024 Revision 05: https://ittc.info/media/11868/75-02-06-03.pdf
- IMO MSC.137(76), *Standards for Ship Manoeuvrability* (2002):
  https://wwwcdn.imo.org/localresources/en/KnowledgeCentre/IndexofIMOResolutions/MSCResolutions/MSC.137%2876%29.pdf
- IHO S-100 product profiles for bathymetry, water-level information and
  currents: https://iho.int/en/enc-ecdis

The ITTC validation ladder distinguishes direct mathematical-model and force
evaluation, deep-water standard manoeuvres, shallow-water standard manoeuvres,
restricted-water manoeuvres, free-running trajectories, and real-time
pilot/expert assessment. These evidence classes must remain distinct.

## Next increments

1. Add a versioned raw-output/normalized-trajectory bundle with transform
   provenance and uncertainty metadata.
2. Add independent metrology and grouped held-out campaign / vessel /
   operating-regime validation intake.
3. Compare the contract metrics against OpenNAOE and rights-reviewed specialist
   models on matched ITTC / SIMMAN / PIANC cases; keep simulation accuracy
   distinct from metric-verifier correctness.
4. Pass only context-qualified manoeuvring envelopes to Symtropy
   [MARITIME-MOBILE-OPS-001 #3798](https://github.com/Luminous-Dynamics/symtropy/issues/3798)
   and the integrated service-chain benchmark
   [#3683](https://github.com/Luminous-Dynamics/symtropy/issues/3683).
