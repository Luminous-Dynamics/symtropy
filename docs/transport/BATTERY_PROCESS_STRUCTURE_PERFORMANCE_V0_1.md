# Battery process–structure–performance genealogy v0.1

Status: research contract and synthetic fixture seed. Not a manufacturing procedure, operating recipe, safety release, production execution system, or measured battery-performance claim.

## Objective

Connect a particular cell's measured performance to its complete, immutable physical genealogy: material lots, intermediate electrode specimens, equipment and calibration, process execution, cell build, formation, inspection, and downstream test evidence. The goal is to identify reproducible process windows across batches—not to select one lucky cell.

## Composition

Compose with Symtropy EV-PLATFORM-001 #2112, EV-BENCH-001 #2164, EV-BATTERY-CIRCULARITY-001 #2155, battery test/metrology PR #4524 and BMS authenticity PR #4656. Compose with Symthaea MAT-RD-001 #4997, BATTERY-INVERSE-DESIGN-001 #11671, dynamic cell model #11485, pack capability #11271 and mission evaluator #11589. This package owns the manufacturing genealogy before the existing test/metrology evidence boundary; it does not duplicate that boundary.

## Paradigm shift

Move from a nominal recipe or isolated QC measurement to a robust process window whose resulting electrode structure and complete-cell outcomes remain acceptable across batches, equipment, operators and normal process variation.

A candidate recipe is not a process window. A cell result cannot be attributed to a recipe unless material lots, intermediate artifacts, process execution, calibration state, cell build and downstream test resolve to the same physical genealogy.

## Genealogy stages

Represent each applicable stage with immutable IDs and parent/child links:

1. Raw material: supplier and lot identity, composition/grade declaration, certificate/source, rights/admission and receipt inspection.
2. Slurry/formulation batch: component lot references and masses, sequence identity, mixing equipment/run identity, time/temperature/environment records and dispersion observations.
3. Coating run: line/equipment identity, roll/web segment, recipe revision, declared process observations, wet/dry thickness, areal loading and spatial sampling plan.
4. Drying: equipment/run identity, measured profile artifact, residence-time evidence, residual solvent/moisture test and process-energy record where available.
5. Calendering: equipment/run identity, force/gap/speed/temperature evidence, electrode thickness, density, porosity, adhesion and rebound observations.
6. Slitting/notching and handling: specimen geometry, edge/particle/contamination inspection, scrap and disposition.
7. Cell assembly: electrode/separator IDs, alignment and stacking/winding record, electrolyte lot/quantity identity, enclosure and build configuration.
8. Formation/aging: exact protocol identity, equipment and calibration evidence, time/temperature/current/voltage record, rest periods and exceptions.
9. Cell QC and test: exact specimen/configuration, protocol/metrology evidence reference, calibrated measurement chain, raw-data digest and result uncertainty.

Not every chemistry uses every stage. Missing, unknown, withheld and not-applicable stages are distinct states; do not silently omit them.

## Field and evidence semantics

Every field is classified as measured, calculated, model-predicted, operator-declared or unknown. Preserve raw readings and derived projections separately. A process setting is not a measured process condition. A valid sensor signature is not proof of physical accuracy. A test result is not accepted if specimen or calibration lineage is broken.

Each genealogy edge binds parent/child IDs, event time, recorder identity, source artifact digest, exact equipment/software/recipe revision, and correction/supersession history. Corrections append a new event; they do not rewrite historical observations.

Track spatial variability: roll/web position, edge/center sampling, within-electrode maps and cell-to-cell variation. A mean coating thickness must not erase measured distribution or sampling coverage.

Material inputs, retained outputs, scrap, waste and unresolved loss reconcile only within a declared system boundary and tolerance. Unknown mass is not silently forced to zero.

## Process-window benchmark

Compare a declared reference recipe, conventional DOE (factorial/response-surface where appropriate), a regression/surrogate baseline with batch/line holdouts, and the proposed process–structure–performance analysis.

Report within-window yield and between-batch variance; electrode loading/thickness/porosity/adhesion/microstructure variation; formation loss and first-cycle efficiency where relevant; full-cell energy/power/impedance/lifetime under matched protocols; scrap/material utilization/solvent or water use/process energy/takt time; process-window width and sensitivity; cost per accepted usable cell and per lifetime delivered energy; and unresolved lineage/calibration failures.

Use batch, roll/web segment, line/equipment, and physical-cell holdouts. Do not split cells from the same manufacturing batch across train and test and call it unseen-batch transfer.

A process-window claim requires repeat batches and reported between-batch variation. One best cell cannot establish a reproducible process.

## Software acceptance

- Every accepted cell-to-test path resolves to material, batch, process run, intermediate specimen, equipment, calibration, cell configuration and test identities.
- Zero accepted test results with broken, ambiguous, temporally impossible or configuration-mismatched genealogy.
- No silent material-lot, recipe-revision or equipment substitution.
- Material flows reconcile or remain explicitly unresolved.
- Calibration validity is evaluated at the actual measurement time and relevant measurement channel.
- Canonical genealogy reports are deterministic under input-order permutations.
- Synthetic adversarial controls cover orphaned artifacts, recipe changed after execution, expired calibration, duplicate/substituted specimens, missing spatial samples, material imbalance, test mismatch and result leakage.

These are data-integrity targets, not physical yield or battery-performance achievements.

## Industry context

IEA Global EV Outlook 2026 reports 2025 global EV battery deployment of about 1.2 TWh and more than 4 TWh of global manufacturing capacity, with over 80% of capacity in China. It notes that some new facilities take more than five years to approach nominal utilization. This makes repeatable process capability, quality, yield and localization strategically important; it is not proof that any particular digital-twin vendor achieves a given yield: https://www.iea.org/reports/global-ev-outlook-2026/electric-vehicle-batteries

The 2026 review Digital Twins for Battery Cell Manufacturing describes strong coupling among mixing, coating, drying, calendering, assembly and formation, while noting that many existing efforts remain fragmented by process step: https://www.mdpi.com/2227-9717/14/13/2158

## Claim ceiling and next increment

A passing v0.1 audit proves declared genealogy completeness and internal consistency only. It does not establish causal process effects, production yield, manufacturing capability, safety, cycle life, cost or scalability. Those require lawful real factory/lab data, repeat production batches and independent physical validation.

Next: schema/fixture validation, deterministic genealogy graph audit, event-order and material-flow checks, exact-cell test linkage, negative controls, permutation tests and an exact-head Rust qualification workflow. Keep all interfaces read-only.