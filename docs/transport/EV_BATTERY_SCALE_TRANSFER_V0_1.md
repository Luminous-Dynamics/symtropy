# EV battery scale-transfer benchmark v0.1

Status: staged scale-lineage contract and synthetic reference audit. No measured scale-up success, physical battery gain, yield improvement or product qualification is claimed.

## Objective

Test whether battery evidence and design rankings remain valid across increasing physical scales and formats. A cell that performs well in a coin cell may fail to transfer to a pouch; a 1–5 Ah prototype does not prove a 50 Ah+ cell or a module/pack will behave equivalently.

A scale transition must declare:
- which variables are physically invariant;
- which variables are explicitly rescaled;
- which variables are re-optimized for the new scale;
- which variables or mappings remain unknown.

Unknown mappings remain visible and prevent an unqualified transfer claim.

## Why this is a concrete engineering gap

Argonne's 2026 CAMP presentation describes a scale bridge from small material quantities (around 10 g active material) to pouch-cell prototypes from 20 mAh to 5 Ah and identifies a further transition to 50 Ah+ PEV formats. It also reports transfer of more than 15 years of slurry/electrode process records into an AI-oriented metadata database. This illustrates the existence of staged scale-up work; it is not evidence that results automatically transfer between stages. Source: https://www1.eere.energy.gov/vehiclesandfuels/downloads/2026_TTO_AMR/bat030_trask_2026_o.pdf

A 2026 pilot-scale aqueous hard-carbon / Prussian White sodium-ion pouch-cell study is another useful case: it reports roughly 1 Ah pouch-cell performance while discussing moisture, dehydration, rheology, coating uniformity and scale-up. These are chemistry- and process-specific results, not a universal battery manufacturing law. Source: https://doi.org/10.1002/aenm.202505344

## Canonical owners

- Symthaea BATTERY-INVERSE-DESIGN #11671 owns mission-conditioned candidate design.
- Symthaea BATTERY-DESIGN-LIFETIME-TRANSFER #11686 owns unseen-design lifetime transfer.
- Symthaea MAT-013 #4344 owns physical sample/process/characterization lineage.
- Symthaea MAT-PSPP #6224 owns process → structure → property → performance semantics.
- Symthaea MAT-UQ #6225 owns cross-fidelity discrepancy, calibration and uncertainty.
- Symthaea MAT-INTERFACE #6227 owns interface identity and condition evidence.
- Symthaea EV-FIELD-EVIDENCE PR #11421 owns data-operation rights.
- Symtropy EV-BATTERY-MFG-ROBUST #4833 owns process-window and lot/line robustness.
- Symtropy EV-BATTERY-LINEAGE-OUTCOME #4791 owns cell/pack/mission evidence links.
- Symtropy EV-BATTERY-CLOSED-LOOP-QUAL #4848 owns design-to-mission reference closure.
- Symtropy EV-BATTERY-ULTRASONIC-NDT #4840 owns sample-held-out electrode NDT.
- Symthaea SODIUM-BATTERY-MISSION #11694 owns the sodium-ion / sodium-metal chemistry-specific campaign. This benchmark can consume it as one scale-transfer case but does not reproduce its chemistry scope.

No new material ontology, process engine, UQ system, electrochemical solver, pack simulator or physical-test authority is introduced.

## Scale stages

The versioned stage taxonomy is extensible and namespaced. The initial reference ranks are:
1. material / precursor;
2. electrode coupon or short web;
3. coin cell;
4. small pouch / prototype cell;
5. large-format cell;
6. module;
7. pack.

The rank expresses an ordered stage class, not a conversion between grams, area, Ah and kWh. Each stage retains its own scale value and unit. Do not divide unlike units or pretend every transition has one universal scale factor.

A stage binds exact design/candidate identity, revision, material/process references, configuration digest, parent stage, scale quantity, unit and evidence reference.

## Benchmark questions

- Does the same candidate preserve its ranking from coin to pouch or from small to large format?
- Which physical changes require re-optimization rather than parameter copying?
- Do current distribution, thermal paths, pressure, wetting/electrolyte inventory and formation change across format?
- Are transfer discrepancies explained by known geometry/process changes or evidence gaps?
- Does the model abstain when it has no admitted bridge evidence for a new chemistry, process route or format?

Candidate transfer variables include areal loading, thickness/porosity, electrode area, current-collector/tab geometry, N/P ratio, electrolyte-to-capacity ratio, stack pressure, wetting, formation trajectory, thermal boundary and test protocol. Their physical owner determines validity and units.

## Benchmark tournament

Compare:
- naive direct transfer;
- dimensionless/physics-informed scale rules where assumptions apply;
- per-stage recalibration on a declared subset of target-scale observations;
- hierarchical or multi-task transfer;
- process-conditioned transfer using exact execution and realized-structure evidence;
- robust interval/set-valued transfer with abstention.

Use held-out physical design combinations, formats, process batches and production lines. Random row splits are not primary evidence of transfer.

## Required metrics

- target-variable error, bias and interval coverage at each exact physical boundary;
- scale-transition discrepancy by format, geometry, chemistry and process route;
- candidate rank preservation / rank reversal / top-K enrichment across each stage;
- Pareto regret on tiny known-truth portfolios;
- hard-gate false-feasible and missed-feasible rates;
- transfer abstention and selective risk versus coverage;
- process-conforming yield by independent lots/lines where data support it;
- capacity, power, DCIR, thermal rise, ageing and service only on matched boundaries/protocols;
- material, electrode, collector, electrolyte and inactive mass per accepted cell/pack;
- process energy, solvent/water, scrap and time per accepted kWh;
- experiments, active material, compute, time and cost required to qualify each bridge.

## Proposed acceptance gates — research targets, not results

- Zero transfer claims without exact source and target scale-stage identities.
- Zero small-cell measurement silently promoted to large-cell, module or pack evidence.
- Zero chemistry or process-family transfer claims outside declared support without abstention.
- 100% of scale variables explicitly marked invariant, rescaled, re-optimized or unknown.
- 100% of physical comparisons linked to admitted measurement evidence and metrology references.
- 100% deterministic reference replay and negative-control detection for the v0.1 audit.
- Prospective physical tolerances and sample sizes are set only after stage-specific baseline repeatability is measured.

## Claim ceiling

A passing synthetic audit establishes only staged lineage, declared mapping and evidence-class consistency. It does not show that a material, process, coin cell or prototype scales physically. Physical transfer claims require paired admitted data at the relevant stages, independent lots/lines where relevant, validated metrology and the appropriate engineering qualification.
