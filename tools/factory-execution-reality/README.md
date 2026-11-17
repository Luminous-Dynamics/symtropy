# Factory Execution Reality Reconciliation v0.1

**Owner:** Symtropy FACTORY-EXECUTION-REALITY-001 [#5758](https://github.com/Luminous-Dynamics/symtropy/issues/5758)  
**Fixture generation:** pump-valve-observed-route-v0.1  
**Status:** synthetic route and event fixtures authored; Rust reconciler and CI qualification not yet implemented.

## Research question

Can the system reconcile what was intended, what MES recorded, and what bounded physical observation streams support—without treating any one stream as unquestioned truth?

The primary output is a four-valued execution-evidence disposition, plus the affected operation/product/genealogy closure. The downstream production KPI is time from discrepancy detection to a correct disposition and then to first independently accepted output.

## Paradigm shift

- Plan = intent, not evidence of execution.
- MES = registration, not necessarily complete physical history.
- RTLS = sampled, uncertain location observations, not proof of machine-cycle completion.
- PLC/machine telemetry = equipment event, not necessarily exact product identity.
- Barcode / operator scan = identity evidence with its own time, source and error modes.
- Inspection record = quality claim requiring scope, calibration, source and authority.
- Missing observation with poor coverage = UNKNOWN, not evidence of non-occurrence.

The reconciler preserves intended, recorded, observed, and reconciled as separate layers.

Possible per-event dispositions:
- CONFIRMED: supported under the declared identity, source and coverage contract;
- CONFLICTED: sources materially disagree;
- UNKNOWN: evidence insufficient, stale, uncalibrated or outside coverage;
- NOT_OBSERVED_WITH_COVERAGE: the defined detector had adequate declared coverage but did not observe the event; this is not a proof of physical non-occurrence.

Possible product/release outcomes are distinct: ACCEPTED, REJECTED, UNKNOWN, BLOCKED. A reconciled route alone cannot yield product acceptance.

## Files

- route-contract-v0.1.tsv: six-step synthetic pump/valve route.
- scenarios-v0.1.tsv: 12 event-reconciliation cases, including paired hidden-future controls.
- observations-v0.1.tsv: synthetic MES, RTLS, machine-cycle, scan and inspection observations. Event time and ingest/observation time are separate.
- coverage-v0.1.tsv: declared sensor coverage, calibration state and location uncertainty windows.
- This is a public development corpus with its expected dispositions exposed. It is not a blind evaluation set.

All identities and event data are synthetic. Fixture references are identifiers, not cryptographic hashes or claims about actual equipment.

## v0.1 semantics

1. The intended route is read from the immutable route contract.
2. Recorded events are retained as claims from their source; their existence does not prove physical execution.
3. Observations are interpreted only within their declared identity, map generation, calibration, coverage and uncertainty scope.
4. Effective/event time determines physical chronology where the source semantics support it; ingest/observed time determines when a decision-maker could know it. Never substitute one for the other.
5. NOT_OBSERVED_WITH_COVERAGE is emitted only when the relevant source is declared calibrated, applicable, available and adequately covering the target time/zone. A gap yields UNKNOWN.
6. Multiple reports with one source-origin group do not count as independent corroboration.
7. Location samples near multiple zone boundaries or position flipping remain ambiguous; do not force a single machine assignment.
8. A machine event without a reliable product identity binding does not prove that the target product experienced that operation.
9. Unrecorded rework or a process-route deviation creates a claim/genealogy impact requiring hold, review or requalification as specified by the relevant owners.
10. A first-article inspection failure or unknown result prevents ACCEPTED even when the route chronology is otherwise reconciled.
11. Paired future scenarios with identical observable prefixes must have identical pre-event decision fingerprints.
12. The public fixture's hidden truth annotations are for development checks only; future generalization requires a separately generated evaluator-held set.

## Baselines

- E0: MES-only route view.
- E1: naive MES+RTLS join without coverage or uncertainty.
- E2: identity/time-aware fusion.
- E3: coverage/calibration-aware fusion.
- E4: provenance and source-independence-aware reconciliation.
- E5: claim/genealogy impact closure and correct disposition.
- E6: exact tiny trace oracle.
- E7: evaluator-only hindsight reference.

All deployable arms use the same observable event prefix, sensors, coverage information and budgets.

## Metrics

- intended/recorded/observed event agreement;
- event-registration precision/recall with coverage denominators;
- sequence-order agreement and route edit distance;
- identity/serial/lot join errors;
- unregistered / duplicated / unauthorized operation detection;
- effective-time vs ingest-time order errors;
- localization/zone accuracy under a declared uncertainty envelope;
- UNKNOWN vs NOT_OBSERVED_WITH_COVERAGE correctness;
- source independence and genealogy closure;
- false-preserve, false-quarantine, reinspection and requalification burden;
- time to detection, reconciliation and first accepted output;
- event-ingestion latency, compute/memory and evidence retention volume.

Do not report RTLS point accuracy as a substitute for product-route or genealogy correctness. Do not report event recall without sensor coverage.

## Hard gates

- zero accepted identity swaps or incorrect genealogy links;
- zero out-of-coverage observations promoted to confirmed execution;
- zero duplicate-origin reports counted as independent;
- zero unqualified route changes accepted without required claim-specific review/requalification;
- no first-article/quality release from route evidence alone;
- 100% paired-prefix non-anticipativity;
- exact agreement with supported tiny trace oracles;
- explicit disposition for each conflict, unknown, missing record and route deviation.

## Prospective targets (not results)

After baseline reproduction:
- at least 95% event/route reconstruction F1 on supported, in-coverage traces;
- 100% recall for finite-corpus critical identity and route deviations;
- zero false physical-absence claims under inadequate coverage;
- at least 30% lower median discrepancy-to-correct-disposition latency than MES-only on matched synthetic cases;
- at least 20% fewer unnecessary quarantines/reinspections than naive fusion without increased false preservation.

These are research hypotheses, not physical accuracy claims or safety thresholds.

## Reuse

Compose with Symtropy #5411 (reconfiguration chronology), #5432 (matched routes), #5520 (selective repair/inspection), #5622 (context grounding), #5704 (qualified path dependencies), #5729 (process-model obligation assurance), and Mycelix #1243 (source derivation). Do not duplicate their owners' systems.

## Claim boundary

This is a synthetic event-reconciliation benchmark. It does not establish actual RTLS performance, factory traceability, product conformity, physical inspection, production savings, human-monitoring compliance, safety, commissioning or autonomous execution authority. An RTLS position, MES event, scan, machine cycle, signature or hash is not proof by itself that a named product experienced a qualified process or is acceptable.

## External context

- Smolenaars, Akcay & Adan (2026), real photonic semiconductor foundry ramp-up study combining RTLS and MES, published online 2 October 2026: https://doi.org/10.1080/21693277.2026.2738937
- Bushagour et al. (2026), review of 93 facility-level resilience indicators, published online 29 September 2026: https://doi.org/10.1080/21693277.2026.2738298
- NIST SI for Manufacturing integration test bench and grounding/context ablations: https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing
