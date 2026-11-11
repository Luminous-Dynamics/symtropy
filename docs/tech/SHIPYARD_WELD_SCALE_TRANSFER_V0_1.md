# Shipyard weld scale-transfer gate v0.1

Tracking: [SHIPYARD-WELD-SCALE-TRANSFER-001 #4661](https://github.com/Luminous-Dynamics/symtropy/issues/4661)  
Parent: [SHIPYARD-WELD-PHYSICS-VVUQ-001 #4614](https://github.com/Luminous-Dynamics/symtropy/issues/4614)

## Research gap

Mancini & Skriko (2026), DOI 10.1007/s40194-026-02695-6, identify poor transferability of calibrated distortion models when joint geometry, thickness, heat input, sequence or restraint differs. They call for common benchmark structures and validation procedures. The review's scope is welding distortion; it explicitly excludes residual-stress prediction.

## Paradigm

Model validity is conditional on a typed domain signature and an exact physical output. A local coupon model cannot be promoted to a panel or block merely because it is physics-based or fits its calibration data. Cross-domain transfer requires a chain of independently validated bridge specimens connecting adjacent scale tiers:
`bead_or_coupon → joint_specimen → subassembly → block_assembly`.

Each bridge binds model revision, changed factors, calibration/validation specimen partitions, measurement and model uncertainty components, output scope, measurement coverage, error tolerance, source digest and decision cutoff. Calibration and validation specimens cannot overlap. A direct skip across multiple scale tiers is rejected.

Distortion, released free-state deformation, thermal history and residual stress are different outputs. Evidence for one does not validate another. This benchmark does not import residual-stress validity from the distortion review or any in-fixture measurement into released-state claims.

## V0 implementation

- Nested dependency-light Rust 2024 workspace.
- Draft 2020-12 schema for domain signatures, model records, bridge edges and transfer requests.
- Small-graph simple-path enumerator with adjacent-scale bridge requirement.
- Conservative additive uncertainty composition (no unjustified cancellation).
- Best admissible path selected by lowest cumulative uncertainty, then path length and stable ID order.
- 30 frozen synthetic controls (2 positives / 28 negatives).
- Focused Rust 1.96 format, test, Clippy and corpus-check workflow.

The path oracle is intentionally bounded to small test graphs. It is not a physical transfer experiment or a model solver.

## Metrics and benchmark targets

- 100% disposition agreement with exhaustive small-graph path oracle.
- Zero false admissions for unbridged categorical or scale changes.
- Zero output-scope promotion, specimen leakage, post-cutoff evidence or digest/identity mismatch.
- Report end-to-end prediction error per bridge edge, output scope and target domain.
- Report empirical prediction-interval coverage **and width**, per material, joint, thickness, restraint and scale stratum.
- Report cumulative measurement, parameter, boundary-condition, model-form and transfer uncertainty separately.
- Compare detailed TEP FEM, inherent strain, local/global, equivalent-load and surrogate models under matched data access, specimens, output operators, compute and calibration budgets.

A 10× speed target is only an aspiration when the exact same benchmark proves the runtime gain while preserving prediction error and uncertainty requirements. No cross-paper leaderboard is justified from heterogeneous cases.

## Claim ceiling

A synthetic PASS proves only transfer metadata and bridge graph contract correctness. It does not establish physical validity, a transferable model, WPS/PQR/WPQT qualification, NDT acceptance, class/flag approval or structural integrity.
