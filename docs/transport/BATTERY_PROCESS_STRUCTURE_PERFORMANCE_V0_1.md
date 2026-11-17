# EV battery lineage-to-outcome adapter v0.1

Status: read-only application contract and synthetic fixture. Not a generic materials ontology, sample-lineage authority, manufacturing process model, cell solver, safety qualification, or measured battery-performance claim.

## Objective

Connect canonical material/process/structure evidence to the EV product thread: exact cell configuration, test evidence, pack assembly and mission outcome. Symtropy consumes upstream references; it does not redefine them.

## Mandatory upstream owners

- Symthaea MAT-013 #4344: physical sample/process/characterization lineage DAG.
- Symthaea MAT-PSPP-001 #6224: process → realized structure → conditioned property → engineering performance.
- Symthaea MAT-INTERFACE-001 #6227: interface identity, condition and evidence.
- Symthaea MAT-UQ-001 #6225: uncertainty, calibration and cross-fidelity discrepancy.
- Symthaea BATTERY-INVERSE-DESIGN-001 #11671: mission-conditioned candidate design and experiment selection.
- Symthaea battery cell dynamics #11485, pack capability #11271 and mission evaluation #11589.
- Symtropy battery test/metrology audit #4524, BMS authenticity #4656, EV platform #2112, EV benchmark #2164 and battery circularity #2155.

Do not create another process DAG, sample identity system, realized-structure vocabulary, interface ontology, uncertainty/calibration framework, generic evidence store or test-metrology authority.

## EV adapter contract

### Cell configuration

Each exact cell configuration binds:
- cell and configuration identity;
- opaque MAT-013 sample-lineage ID, schema version, scale, source evidence and digest;
- MAT-PSPP subject/projection, exact process-lineage link, realized-structure reference, conditioned-property evidence and MAT-UQ profile;
- MAT-INTERFACE subject, version, interface class, condition state, evidence and UQ profile;
- exact process-window and chemistry/material-state references.

The adapter checks identity closure and compatibility only. It does not re-evaluate upstream property values or infer causal process effects.

### Cell tests

Each cell test binds cell ID, exact cell configuration, test protocol, metrology evidence, raw-data SHA-256 and declared evidence class. A test for configuration R2 cannot qualify configuration R1. The audit checks the declared lineage and syntax; it cannot prove the digest corresponds to raw bytes or the calibration is authentic.

### Pack assembly

A pack binds pack/configuration/topology and a set of cell IDs with the exact cell configuration expected by the pack build. Unknown cells, repeated membership and configuration mismatches are rejected. Pack-level metrics remain distinct from cell-level metrics.

### Mission outcome

A mission outcome binds exact pack, duty-cycle profile, test-campaign, system boundary, raw result and metrology references, and evidence class. Cell, pack and installed-vehicle boundaries must not be silently substituted. A declared physical evidence class still requires independent verification before scientific or commercial promotion.

## Benchmark

Use the same exact cell configuration across the cell test, pack build and mission outcome. Report completeness of reference closure, broken links, configuration mismatches, evidence-class distribution, missing metrology, scale/boundary mismatches and deterministic replay.

Compare against the existing EV-BENCH #2164 and circularity #2155 outputs; do not build a second optimizer. A successful reference audit is not a battery-performance benchmark.

## Synthetic fixture

The supplied fixture demonstrates one synthetic cell with MAT-013, MAT-PSPP, MAT-INTERFACE and MAT-UQ references; one cell test; one pack membership; and one synthetic mission outcome. The references/digests are placeholders and do not resolve to physical samples or measured datasets.

## Claim ceiling

A passing v0.1 audit proves only internal consistency of declared EV cell/pack/mission references. It does not establish physical accuracy, chemistry merit, process causality, safety, pack qualification, lifetime, yield, cost, or commercial readiness.

## Next increment

Implement the small Rust consumer-side audit: schema/version identity checks, upstream reference closure, exact configuration matching, cell-to-pack membership, pack-to-mission link checks, physical-evidence metadata requirements, and permutation tests. Keep it read-only and let MAT-013 / MAT-PSPP / MAT-INTERFACE / MAT-UQ remain the authorities for their respective domains.