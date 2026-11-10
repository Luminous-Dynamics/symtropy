# Shipyard Robotic Multipass Welding Twin Benchmark v0.1

**Status:** synthetic contract and test corpus seed. No production process instructions, WPS/PQR, process parameters, NDT criteria, welding controller, physical qualification or authority to manufacture/release an item.

## Owner composition

Parent issue: [SHIPYARD-WELD-TWIN-001 #4433](https://github.com/Luminous-Dynamics/symtropy/issues/4433).

- [SHIPBUILD-CO-DESIGN #2048](https://github.com/Luminous-Dynamics/symtropy/issues/2048) owns shipyard DFM, block/assembly work packages and matched production outcome.
- [COMPONENT-DIGITAL-THREAD #3331](https://github.com/Luminous-Dynamics/symtropy/issues/3331) owns component identity, manufacturing evidence, metrology/NDT and as-built qualification.
- [MARITIME-ASSET-TWIN-COMPOSITION #4406](https://github.com/Luminous-Dynamics/symtropy/issues/4406) owns federated PLM/MES/QMS/EAM composition, identity/configuration/effectivity and twin commissioning.
- Symthaea [CIVIL-MARITIME-CONSTRUCTION-ROBOTICS #9870](https://github.com/Luminous-Dynamics/symthaea/issues/9870) owns robot platform design, tooling, DFM and physical qualification.
- Symtropy [CIV-INFRA-ROBOT #1834](https://github.com/Luminous-Dynamics/symtropy/issues/1834) owns capability matching; Symthaea [ROB-CELL #6039](https://github.com/Luminous-Dynamics/symthaea/issues/6039) owns workcell/fleet composition.
- Mycelix remains owner of applicable source authority, custody, inspection attestation and approval/revocation references.

This benchmark owns only the welding-process state/evidence contract. It does not create a parallel robot dispatcher, manufacturing execution system, NDT authority, component ledger, CAD/PLM or approval ledger.

## Standards and current industry baseline

### Direct process-twin use case

**ISO/TR 23247-101:2026**, published 13 May 2026, is a Technical Report on management of robotic multilayer and multipass gas-shielded metal arc welding. It describes the process twin, welding-equipment twin and workpiece twin as separate entities. Its process sequence includes provisioning, preprocessing, welding, inspection/post-weld operations and documentation.

Sources:
- ISO Online Browsing Platform: https://www.iso.org/obp/ui?_escaped_fragment_=iso%3Astd%3Aiso%3Atr%3A23247%3A-101%3Aed-1%3Av1%3Aen
- Public catalogue metadata: https://standards.iteh.ai/catalog/standards/iso/98e604ef-50a4-4d06-9477-67ebd6cb9e27/iso-tr-23247-101-2026

A Technical Report is a use-case reference, not a qualified welding procedure, acceptance code, NDT method, class approval or evidence that adaptive AI parameter changes produce an acceptable weld.

### Digital twin commissioning and composition

- ISO 23247-5:2026 (published June 2026) addresses manufacturing digital threads across design, planning, production and testing: https://www.iso.org/standard/87425.html
- ISO 23247-6:2026 (published July 2026) distinguishes integrated, unified and federated twin compositions: https://www.iso.org/standard/87426.html
- ISO/AWI 23247-7 was approved in October 2026 and registered in the ISO work programme on 13 October 2026. The work item addresses commissioning of manufacturing digital twins, including frequency, latency, fidelity and readiness. It is under development, not a published standard: https://www.iso.org/standard/94859.html

### Industry maturity

- Fincantieri's 14 October 2026 BOOST² update reports more than 450 employees engaged across 13 shipyards and almost €100 million in programme investment, with applications ranging from operational to under validation and future development. Those are company-reported scale and maturity indicators, not independent throughput, cost or weld-quality results: https://www.fincantieri.com/en/newsroom/press-releases/2026/fincantieri-accelerates-robotics-and-physical-ai-to-build-the-shipyard-of-the-future
- Fincantieri and Generative Bionics announced a four-year humanoid welding development partnership in February 2026, with initial on-site tests planned by end-2026. The announcement is not a passed physical trial or certified production capability: https://www.fincantieri.com/en/newsroom/press-releases/2026/fincantieri-and-generative-bionics-launch-an-industrial-partnership-for-the-development-of-a-humanoid-welding-robot-for-shipyards
- A February 2026 shipyard production-planning review found that algorithmic sophistication does not itself demonstrate operational impact; it highlights relatively stronger behavioural credibility for hybrid discrete-event simulation plus optimization, while yard-wide longitudinal KPI validation remains sparse: https://doi.org/10.3390/jmse14040396

## Paradigm shift: from robot activity to accepted production

Represent every weld pass as a revision-bound, process-qualified, inspection-linked event. Reconcile three different entities:

1. **Workpiece twin** — component/joint identity, parent assembly, material heat/lot, geometry/groove, design revision and effectivity.
2. **Equipment twin** — robot, power source, tool/fixture, controller/program revision, calibration, health, maintenance and identity.
3. **Process twin** — WPS/PQR reference and scope, consumables, pass/layer plan, observed process trace, inspection holds, repair/straightening and documentation.

A process agent may identify a candidate plan revision; it must not mutate a qualified procedure or live control parameter outside the owner-authorized engineering change and requalification path. A digital twin, simulation or AI recommendation cannot promote itself into the source-of-record.

## Contract scope v0.1

Each synthetic workpiece binds exact component, serial, joint, parent assembly, drawing/design revision, material reference/heat/lot, separately bound material-record lot and geometry digest. Every pass repeats the exact workpiece serial, joint and revision so the verifier can detect cross-subject trace reuse. Each process binds WPS/PQR references, revisions, scope, process family, approval status and symbolic parameter-envelope reference. The corpus deliberately uses symbolic envelope identifiers, not usable welding recipes or parameter settings.

Each equipment entity binds robot/welding-source/tool/fixture identities and revisions, program/postprocessor revision, calibration and health evidence. Each pass binds stable pass ID, pass sequence/role, exact joint and workpiece revision, exact WPS and equipment revision, symbolic process envelope, process-trace digest, source time and evidence class.

The lifecycle includes:
- provisioning: workpiece/procedure/equipment/operator/consumable compatibility;
- preprocessing: fit-up, joint preparation and procedure-required preconditions;
- operation: root/fill/cap pass order, process trace, controlled deviations and stop/restart lineage;
- in-process checks: root/cap and required fill-sampling hold points;
- post-weld: visual/NDT record, coverage, disposition, repair/straightening and reinspection;
- documentation: exact, revision-bound final package and source references;
- twin commissioning: schema/identity mapping, frequency, latency, fidelity, update delay and recovery.

Time is role-typed: process observation/source time, transaction/event time, retrieval/ingestion time and decision cutoff are separate. Synthetic/simulated/inferred values are never silently labelled as measured.

## Frozen synthetic corpus

The v0.1 corpus includes 34 unique cases: one valid synthetic trace and 33 adversarial controls. The negative cases cover wrong component/joint/design identity, material lot, WPS/PQR scope and expiry, consumable incompatibility, equipment and calibration drift, program geometry mismatch, missing preprocessing, pass order/identity, missing process trace, out-of-envelope observations, synthetic-to-measured relabelling, in-process and final inspection gaps, NDT disposition escalation, repair/straightening lineage, incomplete document package, twin latency, projection writeback, operator qualification, and unlogged stop/restart.

The corpus is a fixture/specification seed. It is not yet an independent Rust evaluator or physical welding benchmark.

## Metrics and fair industry comparison

Compare manual, mechanized and robotic alternatives only on the same joint family, material/thickness, position/access, approved WPS/PQR scope, fixture, process/quality acceptance criteria, NDT method/coverage, shift/calendar and output denominator. Portable/humanoid assistance is included only when the exact joint/process can be performed under equivalent requirements.

Measure:
- first-pass acceptance and final acceptance separately;
- accepted weld length/joints per labour-hour and cell-hour;
- total lead time/cycle time per accepted joint;
- setup, programming, fit-up, arc-on, idle, inspection, repair and reinspection time;
- reject/repair/rework rate under equal inspection coverage;
- process-trace completeness and exact workpiece/procedure/equipment lineage;
- excursion detection and false acceptance;
- equipment availability, intervention count, calibration failure and recovery;
- energy/consumables/material per accepted output only when measured;
- dimensional distortion only with declared datum, method and uncertainty;
- p50/p95/p99 process-twin latency, missed/late events and recovery convergence;
- reviewer time to reconstruct why one exact joint was accepted.

Initial promotion targets (not achieved results):
- zero false acceptance on all planted identity/procedure/process/inspection negatives;
- 100% pass-order and mandatory inspection hold-point enforcement;
- 100% process/inspection/repair events bind to exact joint and workpiece revision;
- zero twin/simulation/AI writeback into source-owned manufacturing/as-built state;
- deterministic replay of the frozen synthetic process history;
- no production-superiority claim until there is a matched, held-out industrial baseline and an independent acceptance denominator.

## Claim ceiling

A synthetic PASS proves only that the declared state/evidence contract behaves correctly on frozen cases. It does not establish weld soundness, WPS/PQR validity, NDT sufficiency, robot physical capability, vessel/structure acceptability, class approval or authority to manufacture/release the item. Qualified welding engineers, inspectors, manufacturers and the governing project specifications remain the acceptance authorities.
