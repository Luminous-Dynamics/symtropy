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

The v0.1 corpus now includes 54 unique cases: two valid synthetic traces and 52 adversarial controls. The negative cases cover wrong component/joint/design identity, material lot, WPS/PQR scope and expiry, consumable incompatibility, equipment and calibration drift, program geometry mismatch, missing preprocessing, pass order/identity, missing process trace, out-of-envelope observations, synthetic-to-measured relabelling, in-process and final inspection gaps, NDT disposition escalation, repair/straightening lineage, incomplete document package, twin latency, projection writeback, operator qualification, and unlogged stop/restart.

Each root/fill/cap pass now references exact synthetic JSON trace files. The baseline trace set contains root, fill and cap; additional fill-pass-simulated and cap-pass-outside variants let the evaluator isolate evidence-class and process-envelope failures without an earlier trace-subject mismatch. The Rust verifier independently recomputes each file SHA-256, rejects unsafe relative paths, and verifies that the trace subject, serial, joint, pass role, revision, procedure and equipment match the pass record. The corpus remains synthetic and is not a physical welding benchmark.

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
- deterministic replay and exact-byte verification of the frozen synthetic process history;
- no production-superiority claim until there is a matched, held-out industrial baseline and an independent acceptance denominator.

## Claim ceiling

A synthetic PASS proves only that the declared state/evidence contract behaves correctly on frozen cases. It does not establish weld soundness, WPS/PQR validity, NDT sufficiency, robot physical capability, vessel/structure acceptability, class approval or authority to manufacture/release the item. Qualified welding engineers, inspectors, manufacturers and the governing project specifications remain the acceptance authorities.


## Exact enforcement extension — 2026-11-10

Four additional adversarial cases close gaps found during code review:
- equipment health must be `nominal_synthetic`, not merely calibrated;
- operator role scope must equal the workpiece's required scope;
- every pass's parameter-envelope reference must equal the approved procedure's envelope;
- observed digital-twin update interval must not exceed the declared expected interval.

The frozen corpus is now 54 cases (2 positive, 52 negative). The independent Rust evaluator and unit-test count are updated accordingly. These remain synthetic contract controls, not physical welding process qualifications.


## Normative effectivity: WPQ test date versus ship contract date — 2026-11-10

A single `subject_applicability_date_utc` is not enough to select and accept welding procedure evidence under IACS UR W28 Rev.3. The normative model now separates:

- **Evaluation cutoff** — when the engineering decision/replay is evaluated.
- **Normative source retrieval time** — when the source snapshot was retrieved; it cannot be later than the decision cutoff.
- **Ship contracted-for-construction date** — the owner/shipbuilder contract-signing date used by IACS implementation note 2 to identify the ship's applicable revision.
- **WPQ test date** — the date the welding procedure qualification test was carried out.
- **WPQR qualification revision** — the revision (or separately reviewed recognized standard) under which the test/record was made.
- **Prior-revision class acceptance and approved range** — explicit evidence that a previous WPS/WPQR was accepted and still covers the actual application.

IACS UR W28 Rev.3 clause 1.5 requires all **new welding procedure qualification tests** to use Rev.3 from **1 January 2027**. Implementation note 4 says Rev.3 is uniformly implemented by IACS Societies on ships contracted for construction from **1 January 2027**; note 2 defines the contract date as the date the owner and shipbuilder sign the build contract. Clause 1.7 expressly says Rev.3 does not invalidate WPS/WPQR records made and accepted by the Classification Society under previous revisions.

Consequences for the verifier:
1. A new WPQ test dated on/after 1 January 2027 cannot be silently admitted as a Rev.2 qualification.
2. A ship contracted for construction on/after 1 January 2027 must select Rev.3 as the applicable ship ruleset.
3. A pre-transition Rev.2 WPQR is not automatically invalidated for that ship, but acceptance by the applicable Classification Society and the approved range/conditions must be evidenced. Missing acceptance is not PASS; a non-matching range is not PASS.
4. A source snapshot retrieved after the decision cutoff cannot be used to justify what was known at that earlier decision time.
5. The applicable ship ruleset and the procedure's qualification revision are separate fields. Never infer one from the other.

Sources:
- IACS UR W28 current/future revision page and effective date: https://iacs.org.uk/resolutions/unified-requirements/ur-w/ur-w28-rev2-cln
- IACS UR W28 Rev.3, clauses 1.5–1.7 and implementation notes 2–4 (ClassNK-hosted PDF): https://www.classnk.or.jp/hp/pdf/info_service/iacs_ur_and_ui/ur_w28_rev.3_sep_2025_complete_revision.pdf
- Lloyd's Register Class News 14/2026 (3 September 2026), which confirms new WPQ from 1 January 2027 and preserves previous accepted WPS/WPQ records subject to range and conditions: https://www.lr.org/en/knowledge/class-news/14-26/

### ISO 15614-1 transition watch

ISO 15614-1:2017 with Amendment 1:2019 remains the published standard currently identified by ISO. The third edition is ISO/FDIS 15614-1, at FDIS formal approval stage (50.00) on 21 October 2026; it is not yet a published International Standard. Keep its draft text/version separate from the current published edition:
- Current published edition: https://www.iso.org/standard/51792.html
- Third-edition FDIS stage: https://committee.iso.org/standard/86026.html

### New W28 adversarial cases

The updated corpus distinguishes:
- new WPQ test on/after 1 January 2027 but still labeled Rev.2 → Rev.3 required;
- a Rev.2 WPQR carried into a Rev.3-governed newbuild with explicit Class Society acceptance and matching approved range → historical qualification may remain usable within that scope;
- the same legacy WPQR without Class Society acceptance → acceptance not established;
- the same legacy WPQR where its approved range does not cover the subject → scope not confirmed;
- Rev.3 selected before its effective date → future ruleset not yet applicable;
- a normative source retrieved after the evaluation cutoff → cannot be used for that decision.

The standard's applicability is not a date-only lookup. It is a claim about the ship, qualification event, approval authority and approved scope, each with its own source evidence.

## Material-lot and raw-trace integrity — 2026-11-10

The independent verifier now checks that the workpiece heat/lot matches receiving evidence and the material record, that the approved procedure's qualified material-heat-lot set contains the exact heat/lot, and that every pass and its raw trace bind the same lot. It rejects duplicate JSON object keys at every nesting depth before evaluating trace semantics, checks corpus identity/version and exact fixed case count, and rejects duplicate case IDs or an unexpected base-fixture name.

The frozen corpus is now **54 unique cases (2 synthetic positives, 52 negative controls)**. The three pass-trace SHA-256 values and the wrong-pass fixture digest were recomputed from the exact updated raw JSON bytes. These remain synthetic contract checks; they do not authenticate real material certificates or qualify a welding procedure.


## Final v0.1 corpus accounting — 2026-11-10

The authoritative frozen-corpus count is **54 unique cases: 2 valid synthetic contract cases and 52 negative controls**:
- `valid_root_fill_cap_trace`: valid synthetic pass/process/inspection/evidence chain;
- `legacy_rev2_wpqr_preserved_for_rev3_ship_when_class_accepted`: a previously qualified Rev.2 record is not rejected solely because the ship is governed by Rev.3; explicit Class Society acceptance and matching range are required.

The Rust test uses the strict duplicate-key parser for both the raw process traces and the top-level frozen corpus. The CLI report includes the SHA-256 of the exact corpus bytes. Earlier 41-/46-/51-case counts and 1-positive/50-negative interim notes are historical snapshots, superseded by this 54-case v0.1 baseline.


## Multipass sequence generalization — 2026-11-10

The executable verifier now accepts **one root pass, one-or-more ordered fill passes, and one cap pass**. Pass indices must be contiguous. Each pass must have a canonical UTC source time no later than the evaluation cutoff; pass times must be monotonic through the sequence. Every pass retains exact raw-trace SHA-256 and subject binding.

The frozen corpus now contains **56 unique cases: 3 synthetic-valid cases and 53 negative controls**. New cases:
- a four-pass root/fill/fill/cap trace, expected to remain `WELD_TRACE_VALID_SYNTHETIC_ONLY`;
- a time-reordered pass, rejected as `PASS_TIME_ORDER_INVALID`.

The additional fill pass has its own exact-byte trace fixture and digest. This remains a synthetic contract test, not a welding procedure qualification or physical process result.
