# EV battery second-life mission qualification v0.1

Status: read-only routing/qualification contract and synthetic fixture. Not a battery safety procedure, BMS implementation, test authorization, certification or measured second-life result.

## Mission

Decide whether an exact retired EV battery configuration can safely deliver a different second-life/third-life mission under its actual degradation history, cell heterogeneity, uncertainty, BMS/protection compatibility and independent test evidence.

Do not reduce the decision to a universal SOH percentage. A battery with an attractive capacity figure can still be unsuitable because of weak-cell dispersion, impedance mismatch, thermal history, latent faults, isolation issues, unavailable telemetry, unsafe BMS transitions or a mission that crosses the capability envelope.

## Ownership and composition

- Symtropy EV-BATTERY-CIRCULARITY #2155 owns pack integration, repair granularity and end-of-life recovery economics.
- Symtropy EV-BENCH #2164 owns matched industry/service comparisons.
- Symtropy EV-PLATFORM #2112 owns product/mobility mission definition.
- Symtropy EV lineage-to-outcome adapter #4791 owns EV-specific references to canonical material/process evidence.
- Symthaea EV-SOH-TRANSFER #11252 owns health/RUL datasets, estimators, uncertainty and cell-held-out evaluation.
- Symthaea EV-PACK-CAPABILITY #11271 and EV-MISSION-CAPABILITY #11589 own pack topology, limiting-cell and time-indexed mission capability.
- Symthaea EV-DYNAMIC-POWER-ECM #11485 owns the dynamic cell electrothermal envelope.
- Mycelix MOBILITY-BATTERY-PASSPORT #6062 / refresh #6076 own lifecycle/passport records and regulatory projections.

This work consumes those owners. It does not build a second SOH predictor, electrochemical solver, pack simulator, battery passport, repairability calculator, or lifecycle database.

## Standards and research context

**IEEE 2993-2025** is an active IEEE recommended practice for selection, design, operation and maintenance of second-life EV batteries in energy-storage systems at voltage levels up to 10 kV. It assesses technical indices for efficiency, safety and economic aspects. This benchmark is not a conformance claim; the licensed normative text and locally applicable requirements must be consulted by qualified practitioners. Official catalogue: https://standards.ieee.org/ieee/519/10617/

**EU Regulation 2023/1542** has distinct provisions for batteries subject to preparation for reuse, preparation for repurposing, repurposing or remanufacturing (Article 45), including quality-control and safety instructions and applicable compliance obligations. Article 73 addresses evidence that qualifying waste batteries prepared for reuse/repurposing are no longer waste, including an SOH/capability assessment and documentation of further use/transfer. Applicability and operator duties depend on the jurisdiction, battery category and dates; this tool is not a legal compliance engine. Consolidated text: https://eur-lex.europa.eu/eli/reg/2023/1542/2026-08-13/eng/

A 2026 stationary-storage study highlights cell heterogeneity, SOC, degradation history and safety constraints; it also notes that micro-health information can improve on capacity-only grading. Source: https://www.mdpi.com/1996-1073/19/9/2026

A 29 October 2026 BMS review distinguishes four second-life strategies: reuse unchanged, update software/firmware, use a gateway/interface, or replace the BMS. It identifies secure firmware updates, data access and re-certification as open challenges. DOI: https://doi.org/10.1109/ACCESS.2026.3748316

## Paradigm shift

Replace the question 'is the pack above 70% SOH?' with:

> Can this exact set of cells, in this exact configuration and with this degradation/fault history, meet this new duty profile for the required horizon while maintaining conservative voltage, power, temperature, isolation, imbalance and reserve margins—and can those claims be independently evidenced?

Routing outcomes:
- CONTINUE_FIRST_LIFE
- REPAIR_AND_REASSESS
- ELIGIBLE_FOR_SECOND_LIFE_MISSION
- ELIGIBLE_FOR_THIRD_LIFE_BACKUP (separate lower-duty mission)
- RECYCLE_OR_RECOVER
- ABSTAIN_INSUFFICIENT_EVIDENCE

Eligibility is a conjunction of hard gates. Unknown safety, history, BMS/protection or mission capability is not compensated by a favorable SOH or a weighted score.

## v0.1 contract

### Identity and history

Bind the original vehicle, pack, module, cell topology, exact configuration/firmware, chemistry, supplier/lot where known, first-life duty and environmental history, charge/temperature profile, maintenance/service, overcharge, deep-discharge, crash, vibration, water ingress, isolation and thermal-event records. Distinguish NO_EVENT_REPORTED from VERIFIED_ABSENT; missing history remains UNKNOWN.

### Health and degradation

Consume qualified cell-level and pack-level observations, not just a pack-average SOH: remaining capacity, DC resistance/impedance distribution, cell dispersion, self-discharge, voltage response, thermal behavior, limiting-cell identity, sensor/calibration evidence, observation age, uncertainty and OOD status. Preserve degradation mode/known failure history where evidence permits.

### New mission

Bind a distinct duty cycle, ambient/cooling boundary, charge-source and schedule, peak/continuous power, usable energy, reserve, throughput, calendar duration, availability, site/fire-system constraints and lifecycle/system boundary. Candidate missions include PV self-consumption, microgrid storage, industrial peak reduction, telecom/clinic essential loads, and low-duty UPS backup.

### BMS and protection compatibility

Record one proposed architecture: reuse original BMS unchanged; authorized software/firmware update; gateway/interface; or replace with a separately qualified BMS. Bind sensor mappings, cell topology, firmware digest, protection functions, isolation monitoring, contactors, communications, cybersecurity, update/rollback authority and validation evidence. The evaluator proposes and audits; it never flashes firmware, changes thresholds, closes contactors or commands a battery.

### Safety and eligibility gates

Require explicit evidence for identity closure; cell population coverage; complete categorized safety/fault history; physical evidence for verified-absent fault records; physical cell-health observations; isolation and protection; thermal and voltage margins; pack enclosure and propagation controls; physically evidenced BMS compatibility; transport/handling; mission capability and reserve; test/quality-control; and applicable regulatory/operator records. The lower-bound usable energy, peak power and reserve must meet the mission requirements, with non-negative worst-case thermal margin. Any required gate FAIL or UNKNOWN blocks an eligible repurpose route.

## Benchmark tournament

Compare on the same frozen retired-pack cohort:
1. Single SOH threshold baseline (diagnostic baseline only; never release authority).
2. Capacity + DC resistance/impedance rule.
3. Cell dispersion + thermal/fault-history rule.
4. Degradation-mode/micro-health-aware model where admitted evidence exists.
5. Mission-conditioned time-indexed capability with uncertainty, hard gates and ABSTAIN.

Split by original vehicle, pack, OEM, chemistry, production lot, first-life duty regime and repurposed configuration. Sibling modules/cells from one original pack must not appear in train and test when claiming unseen-pack transfer.

Report unsafe false-acceptance rate, false rejection and abstention coverage; mission energy/power/reserve feasibility; per-cell imbalance and weak-cell detection; observed second-life EFC/calendar life; BMS strategy integration/commissioning; cost per lifetime delivered kWh; downtime/availability; warranty/logistics/liability; and end-of-second-life recovery.

## Initial research targets — not industry standards or achievements

- Zero unsafe accepts in the frozen synthetic/adversarial suite for unknown safety, missing incident-history categories, synthetic `VERIFIED_ABSENT` claims, synthetic cell-health evidence, unsupported BMS configuration and mission-envelope violations.
- For nominal 90% capability intervals, target at least 90% empirical coverage on held-out pack families; also disclose interval width, worst-family performance and sample-size uncertainty.
- Report false-acceptance confidence bounds by chemistry, OEM, degradation mode and first-life duty; do not claim zero real-world risk from finite tests.
- Demonstrate prospective mission-routing benefit over capacity-only and capacity+DCIR baselines on independently tested retired packs.
- A commercial second-life case is positive only when measured commissioning yield, safety/quality-control, BMS integration, warranty, insurance, transport, service life and end-of-life costs beat matched new-storage alternatives on the same useful-service boundary.

## Claim ceiling

A passing audit proves only consistency of declared records, evidence references, hard gates and mission-comparison rules. It does not prove latent-defect absence, safety, regulatory compliance, second-life service life, bankability or economic/environmental superiority. Physical testing, qualified engineering review, local regulatory duties and responsible economic-operator obligations remain external.