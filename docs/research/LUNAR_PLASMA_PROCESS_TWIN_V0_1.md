# LUNAR-PLASMA-PROCESS-TWIN-001 — process evidence contract v0.1

**Status:** Integration design and synthetic refusal fixture. No plasma solver, process model, or physical lunar-performance claim is implemented.  
**Parent research:** [Symthaea #12294](https://github.com/Luminous-Dynamics/symthaea/issues/12294)  
**Symtropy issue:** [#5343](https://github.com/Luminous-Dynamics/symtropy/issues/5343)  
**Fixture:** [lunar-plasma-isru-process-v0.json](../../benchmarks/plasma/lunar-plasma-isru-process-v0.json)

## Decision

The lunar fluorinated-plasma pathway needs a strict boundary between plasma diagnostics and resource-product claims. This contract composes the existing plasma run receipt with the resource-to-product graph. It does not solve plasma chemistry or prove that a process works.

The central chain is:

`feedstock batch → regime-qualified plasma scenario → species observations → captured gas streams → separation/purification → accepted O2 product`

A parallel reagent/material chain must preserve opening inventory, makeup, recovered inventory, residue-bound material, purge, unknowns and waste destination. A third chain records grid-side utilities and equipment support requirements.

## Existing authorities

- [PLASMA-TWIN-001 #1685](https://github.com/Luminous-Dynamics/symtropy/issues/1685): solver-neutral scenario and capability contract.
- [PLASMA-TWIN-002 #1703](https://github.com/Luminous-Dynamics/symtropy/issues/1703): immutable run receipt, artifact integrity and refusal tests.
- [BENCH-SPACE-008 #1803](https://github.com/Luminous-Dynamics/symtropy/issues/1803): resource-to-product graph and mass/coproduct accounting.
- [LUNAR-SEED-UTILITY-001 #1970](https://github.com/Luminous-Dynamics/symtropy/issues/1970): batch-level dispatch and energy accounting.
- [PHYS-ENERGY-PORTS-001 #3615](https://github.com/Luminous-Dynamics/symtropy/issues/3615): typed energy transfers and boundary semantics.
- [CISLUNAR-INDUSTRIALIZATION-GATE-001 #5341](https://github.com/Luminous-Dynamics/symtropy/issues/5341): accepted off-Earth product and supportability gate.

This package owns only the **plasma-fluorination process projection** and its application-specific refusal semantics. It does not replace any generic owner.

## Contract concepts

### 1. Feedstock identity

A feedstock batch must bind:
- simulant / sample / material identity and source;
- lot, conditioning, dry mass, mineralogical and compositional evidence;
- particle-size and physical-form metadata when available;
- test article / process run identity;
- uncertainty and source rights.

A generic label such as "lunar regolith" is insufficient for a qualified comparison. Unknown mineralogy or missing batch identity narrows the claim.

### 2. Observation versus product

Keep the following as different typed records:
- OES / optical diagnostic observation;
- XPS / EDS / XRD solid-phase observation;
- quantified captured gas species;
- separated gas stream;
- purified O2 batch;
- accepted delivered O2 product.

An oxygen spectral feature is not a flow measurement. A CO or CO2 observation cannot populate an O2 product field. Product mass, purity, pressure, sampling interval, calibration identity and raw artifact lineage must all be present before a product contract can be evaluated.

### 3. Reagent inventory

The reagent ledger must distinguish:
- opening circulating inventory;
- fresh makeup;
- amount returned to reusable inventory;
- amount bound in solids or residues;
- purge and waste;
- ending circulating inventory;
- unknown or unreconciled inventory.

A process may be arithmetically balanced at the total-mass level while its reagent sub-ledger is incomplete. Both checks are required. Unknown does not mean zero. A loop is not closed merely because the source paper describes a future recycling concept.

### 4. Energy boundary

Grid energy is an explicit set of non-overlapping meters / loads:
- source and power conditioning;
- vacuum and pumping;
- feed and material handling;
- gas circulation and capture;
- separation and purification;
- thermal management;
- controls, instrumentation, standby and recovery.

The synthetic fixture deliberately records no physical power values. For a real case, each meter needs boundary identity, calibration, time interval and source evidence. Plasma terminal input must not be compared with a competitor's wall-plug input.

### 5. Product and coproduct

The O2 customer contract defines purity, pressure, delivery interval, storage compatibility and accepted mass. CO, CO2 and other gases remain separately identified.

Potential coproducts remain uncredited until their composition, recovery, purification, product grade, receiving process and downstream demand are supported. A theoretical element balance is not accepted product supply.

## Terminal dispositions

The fixture uses explicit dispositions:
- `SYNTHETIC_ACCOUNTING_PASS`: internal arithmetic / contract checks passed for a synthetic example only.
- `BLOCKED`: evidence or boundary terms are missing, unknown, uncalibrated or incomplete.
- `REJECTED`: an invariant is contradicted (for example, species substitution or mass-balance mismatch).
- `NOT_COMPARABLE`: both cases may be individually describable, but their product / feedstock / boundary contracts do not match.
- `NON_CONVERGED`, `FAILED`, `INCOMPLETE_OUTPUT`: reserved for future externally run solver cases.

None of these dispositions is hardware authorization or a physical qualification.

## Synthetic fixture

`benchmarks/plasma/lunar-plasma-isru-process-v0.json` contains 13 synthetic scenarios: one accounting positive control; blocked cases for missing reagent inventory, qualitative-only oxygen evidence, omitted energy, unqualified coproduct, expired calibration, excessive uncertainty and unverified sampling recovery; rejected cases for CO2-to-O2 promotion, feedstock mass nonclosure, calibration species-scope mismatch and sampling-recovery-corrected mass mismatch; and one NOT_COMPARABLE product-contract case.

All quantities in the positive control are invented solely to exercise arithmetic. They do not represent published plasma experiments, an operating condition, a process recipe, or expected lunar performance. The fixture sets `physical_measurements=false` and `plasma_solver_outputs=false`.

## Required evaluator checks

1. Unique scenario, feedstock, process, stream, meter and artifact identities.
2. Known schema version and explicit evidence class.
3. Feedstock mass ledger closure with a declared tolerance and uncertainty basis.
4. Separate reagent inventory closure; no hidden makeup, purge, residue binding or ending inventory.
5. Species identity is immutable through observation → capture → purification → product mapping.
6. O2 product requires a quantitative species-specific measurement, exact calibration identity/scope, calibration validity at observation time (including actual calendar/time ranges), uncertainty within one-third of the declared decision margin, independent sampling-recovery check, cross-sensitivity pass, interval identity and raw trace.
7. Complete, non-overlapping grid-side meter coverage before computing system energy intensity.
8. Coproduct credits require a product acceptance and receiving-process identity.
9. Comparison requires the same feedstock strata, accepted product contract and energy boundary.
10. Synthetic outputs cannot deserialize or project as physical observations or qualified product.
11. Mutation of source, input, calibration or artifact identity invalidates its receipt.
12. Every attempted scenario receives a disposition; no silent zero, omitted failure or fallback to generated "physical" values.
13. Sampling-recovery-corrected measured gas mass must reconcile with accepted O2 product mass.

## Adversarial cases

- OES observation promoted to quantified O2.
- CO / CO2 relabelled as O2.
- Opening reagent inventory, purge, or residue-bound reagent omitted.
- Unknown recovery silently replaced with 100%.
- Grid power conditioning or vacuum load omitted.
- Total meter combined with branch meters.
- Duplicate material stream or meter counted twice.
- Product purity or pressure absent.
- Unqualified silicon / metal / salt coproduct credited.
- Same material family label hides a changed feedstock lot or mineralogy.
- Stale solver or article identity reused with a new process configuration.
- Synthetic pass represented as a lunar measurement.
- Hash validity presented as solver accuracy or physical validation.

## Next implementation tranche

- Add a narrow Rust-native evaluator under a standalone tool boundary, reusing shared quantity, digest, evidence and resource-balance conventions.
- Keep source corpus and synthetic fixtures separate from measured evidence.
- Add a mutation suite with independent arithmetic checks and expected terminal dispositions.
- Connect the external solver only after a supported plasma regime, exact reference case, source data and required diagnostics are pinned.
- Add a read-only bridge to Symthaea for decision-aware comparison and experiment selection.

## Claim ceiling

The current artifact is an integration contract plus synthetic fixture. It establishes no fluorine recycling performance, oxygen yield, purity, energy intensity, lunar resource abundance, cost competitiveness, environmental acceptability, process safety, device qualification or flight readiness. No hazardous gas recipe, device actuation or process operation is specified or authorized.
