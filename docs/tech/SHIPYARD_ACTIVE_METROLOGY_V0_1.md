# Decision-aware active metrology for inter-pass welding v0.1

Tracking: [SHIPYARD-ACTIVE-METROLOGY-001 #4997](https://github.com/Luminous-Dynamics/symtropy/issues/4997)

## Problem

The inter-pass planner (#4649) abstains when the current geometry uncertainty exceeds its decision threshold. The next missing capability is to decide which additional measurement is worthwhile, using its guaranteed uncertainty bound, acquisition time, energy and interruption cost.

A fixed scan cadence wastes time where the current observation already supports a decision. A forced pass recommendation wastes trust where uncertainty is too high. The new policy exposes a third option: acquire more information, but only through a measurement profile whose scope and capability are explicitly qualified.

## Decision rule

A measurement option is eligible only if it is bound to the exact current workpiece, joint revision, layer, predecessor pass, scan ID, scan-byte digest and registration-receipt digest. Its profile must be qualified for residual-fill geometry, synthetic-only in v0.1, and within its declared operating domain.

Let U be the current uncertainty upper bound and T the fixed decision threshold. For measurement action m, let B_m be its declared post-measurement uncertainty upper bound, and B_profile be the profile's conservative capability floor. It must satisfy:

- 0 < B_profile ≤ B_m < U;
- its acquisition time is non-zero;
- resource and access constraints are feasible;
- it has not already been observed;
- it does not consume future information or request a live robot action.

A profile with B_profile=3 cannot justify an option claiming B_m=2. That would promise better uncertainty than the profile is qualified to guarantee.

If U ≤ T, the selector returns NO_ADDITIONAL_MEASUREMENT_REQUIRED. If U > T, it returns the nondominated measurement options. Options with B_m ≤ T are marked threshold-ready; options above T may still be useful partial reductions, but the next-pass planner remains blocked until a new observation satisfies T.

The four separate objectives are minimized: post-measurement uncertainty bound, acquisition duration, energy and operational interruption risk. The selector does not collapse these into a weighted score, choose a single action or authorize acquisition/execution.

**A proposed B_m is not an observed result.** After an actual acquisition, the caller must submit a new scan ID, real source-byte SHA-256 and a new registration/transform receipt SHA-256. Old recommendations cannot be replayed against a changed observation.

All values in v0.1 are symbolic synthetic units. The profile capabilities are contract fixtures, not calibrated sensor specifications.

## Frozen known-answer case

The synthetic corpus starts at uncertainty 20 with a fixed threshold of 6. Five options are eligible; two can meet the threshold. A slow high-cost option is dominated. The known answer is four Pareto options:
- HIGH-RES: post-bound 3, time 10, energy 8, risk 1;
- ALTERNATE-VIEW: post-bound 5, time 5, energy 3, risk 3;
- FAST-RESCAN: post-bound 8, time 2, energy 1, risk 1;
- LOW-RES: post-bound 12, time 1, energy 1, risk 0.

The independent replay should reproduce: 5 eligible options, 2 threshold-ready options, Pareto IDs [ALTERNATE-VIEW, FAST-RESCAN, HIGH-RES, LOW-RES]. This is a decision-contract known answer, not a claim about any physical scanner.

## V0 implementation

- `crates/tools/shipyard-active-metrology`: dependency-free Rust 2024 library in an isolated nested Cargo workspace.
- `docs/tech/SHIPYARD_ACTIVE_METROLOGY_V0_1.schema.json`: Draft 2020-12 input contract.
- `benchmarks/shipyard/active-metrology/corpus-v0.1.json`: frozen known-answer case.
- Focused CI: Rust 1.96 format, unit tests and Clippy with warnings denied.
- 24 tests cover threshold handling, Pareto output, scan/registration digest binding, stale identity, profile revision and output, profile qualification/evidence/domain, capability underclaim, non-reduction, resource/access constraints, already-observed/future/live-action denial, duplicate identities and immutable thresholds.

## Integration

- #4649 remains the next-pass planner. It consumes only a newly observed scan whose uncertainty bound is within its decision threshold.
- #4614 owns calibration/qualification of real measurement profiles.
- #4643 owns BAM data admission; the Zenodo raw archive remains rights-pending.
- #4433 owns pass and equipment identity.
- #4633 owns sequence/fixture/release optimization.

## Claim ceiling

PASS establishes deterministic selection of synthetic measurement options under the stated contract. It does not establish scanner accuracy, a real measurement uncertainty, physical residual-fill accuracy, weld quality, WPS qualification or permission to execute a robot or weld action.
