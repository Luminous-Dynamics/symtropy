# Shipyard lift-envelope preflight (experimental V0.1)

This isolated Rust crate performs **bounded evidence and arithmetic preflight checks** for a synthetic single-crane, single-block hoist. It is a child of [SHIPBUILD-CO-DESIGN-001 #2048](https://github.com/Luminous-Dynamics/symtropy/issues/2048) and [SHIPYARD-LIFT-ENVELOPE-001 #6077](https://github.com/Luminous-Dynamics/symtropy/issues/6077).

It is deliberately not a crane controller, rigging-design engine, structural solver, lift permit, method statement, or safety authority.

## Current scope

The evaluator checks:

- one block ID and exact design/configuration revision;
- source-record IDs, owner IDs, document revisions, exact subject IDs/revisions, SHA-256 syntax, source state and validity interval;
- upper mass bound and source-approved dynamic amplification factor;
- conservative per-leg maximum vertical load-share bounds and lower sling-angle sine bounds;
- upward-rounded integer load/tension arithmetic;
- each leg's configuration-specific axial tensile WLL;
- an exact crane chart cell keyed by crane, configuration revision, radius band and working quadrant;
- observed gust upper bound against the operation's declared weather limit;
- binding of an existing motion-envelope report and occupancy projection to the same block, revision, site epoch, frame, clock domain and full lift interval.

The crate consumes motion and occupancy identities from the existing construction trajectory/occupancy work. It does not create paths, swept volumes, collision results, crane reservations or actuator commands.

## Fixed-point arithmetic contract

Units are explicit:

- mass: integer kilograms;
- force: integer milli-newtons (mN);
- ratios: parts per million (ppm);
- wind speed: integer millimetres per second;
- operation time: the declared monotonic integer tick domain.

The profile uses (9{,}807) mN/kg as a standard-gravity conversion rounded upward from (g_0). For a source-declared upper mass (m_{kg}) and dynamic factor (D_{ppm}):

`W_upper_mN = ceil(m_kg * 9807 * D_ppm / 1,000,000)`

For each rigging leg (i), using an engineer-supplied upper bound (S_{i,ppm}) on the share of vertical load and lower bound (Q_{i,ppm}) on `sin(angle above horizontal)`:

`T_i_upper_mN = ceil(W_upper_mN * S_i_ppm / Q_i_ppm)`

All arithmetic is checked; demand is always rounded upward. Overflow or missing bounds produce `Indeterminate`, never a smaller load.

**Load-share semantics matter:** the per-leg values are *upper bounds*, not a nominal partition. Their sum must be at least 1,000,000 ppm so that the declared bounds can cover the complete vertical load. The sum may exceed 1,000,000 ppm because independent worst-case leg bounds need not be simultaneously attainable. Equal load sharing is never inferred.

The supplied axial WLL is for the exact rigging component/configuration. The sling-angle amplification is calculated separately in the formula; any other manufacturer-required deratings must already be reflected in the source-supplied WLL. The exact crane chart cell supplies a declared net allowable suspended load after hook-block/rigging deductions. No interpolation or extrapolation between chart cells is permitted.

The example unit case uses 40,000 kg, a 1.10 dynamic factor, two legs with 0.5 maximum vertical share each, and a lower sine bound of 0.707107. It yields a conservative total design-load bound of 431,508,000 mN and a per-leg tension bound of 305,122,139 mN. These are synthetic arithmetic fixtures, not a real rigging design or crane recommendation.

## Evidence and authority boundaries

An `EvidenceRef` binds a record ID, owner ID, document revision, exact subject ID/revision, digest and validity interval. Reusing a source record is allowed only when the complete reference metadata is identical; the same record ID with conflicting metadata is indeterminate.

The implementation checks that a digest *looks like* SHA-256 and that the declared record state/validity covers the lift interval. It does **not** authenticate the digest, signer, issuing authority, certificate, measured value, OEM chart, engineering calculation or physical truth. A caller-provided `Current` state is not proof of authenticity.

`ReadyForHumanReview` means only that the supplied references and declared arithmetic are internally consistent under this bounded contract. It is not equivalent to `SafeToLift`, `Approved`, a permit or operational authorization. The responsible engineer, lift planner, crane/rigging owners, site authority and applicable regulatory/class process remain authoritative.

The current V0 does not calculate:
- block structural strength, lifting-point strength, local buckling or temporary support;
- centre-of-gravity geometry or stability (a separately approved CG-envelope reference is required);
- rigging design, load-sharing distribution, sling geometry, spreader-beam strength or unequalisation dynamics (conservative load-share and angle bounds are externally supplied);
- crane structural capacity or dynamic response (the exact OEM chart cell is externally supplied);
- ground/outrigger bearing, pavement/soil capacity or support settlement;
- wind/wave loads, vessel motion, rope dynamics, pendulation or suspended-load control;
- multi-crane synchronization, tandem lifts, turning, rotation or complex lift sequencing.

These remain unsupported, not implicitly passed.

## Tests

The dependency-free unit suite currently covers:

- nominal arithmetic and human-review-only disposition;
- crane chart capacity and exact-cell mismatch/no interpolation;
- rigging WLL, missing share bounds and conservative over-coverage;
- stale/revoked/malformed/conflicting evidence;
- exact subject/revision binding for blocks and rigging components;
- weather limit and motion/occupancy binding;
- deterministic output under input permutation;
- upward rounding, overflow and invalid time intervals.

Run:

```sh
cargo fmt --manifest-path tools/shipyard-lift-envelope/Cargo.toml -- --check
cargo test --locked --offline --manifest-path tools/shipyard-lift-envelope/Cargo.toml
cargo check --locked --offline --manifest-path tools/shipyard-lift-envelope/Cargo.toml --all-targets
cargo clippy --locked --offline --manifest-path tools/shipyard-lift-envelope/Cargo.toml --all-targets -- -D warnings
```

The Rust toolchain is unavailable in the current authoring environment. These tests have been authored but have **not** been compiled or run locally; the exact-head workflow is the next gate.

## Next qualification steps

1. Independently review the arithmetic contract, especially load-share/angle semantics and conservative units.
2. Add machine-readable conformance vectors and an independently implemented rational oracle.
3. Add source-specific chart/certificate and validity/invalidation cases without embedding proprietary real charts.
4. Bind the exact #5345 motion report and #5369 occupancy projection, preserving all source references and preventing margin double expansion.
5. Only after rights review, obtain synthetic or permissioned lift cases with owner-approved engineering envelopes and blinded negative controls.
6. Keep downstream hoisting sequence/path optimisation comparisons separate from preflight correctness. The 2026 A*HNSA paper is a sequence/path-planning comparator, not a direct competitor for this arithmetic/evidence verifier.

## Literature anchors

- Qi et al., *Application of Digital Twin-Based Improved A*HNSA Algorithm to Ship Block Hoisting Tasks*, 2026: https://xuebao.sjtu.edu.cn/EN/abstract/abstract51242.shtml
- Shipyard crane weather-aware decision support, 2026: https://koreascience.or.kr/article/JAKO202621650483224.page
- Suspended-block model-predictive control for floating cranes under waves, 2026: https://doi.org/10.1093/jcde/qwag068
- Bagheri et al., multi-crane lift planning, 2024: https://doi.org/10.1016/j.autcon.2024.105731
- NASSCO shipyard rigging-analysis report (weight/centre-of-gravity context): https://www.nsrp.org/wp-content/uploads/2015/10/Deliverable-2010-620-Streamlining_Shipyard_Rigging_Analysis_Final_Report-NASSCO.pdf

The published results above are research comparators and engineering context, not proof of this crate's correctness or physical applicability.
