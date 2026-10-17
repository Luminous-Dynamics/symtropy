# Washer Fault-to-Repair Benchmark Curriculum v0.1

**Status:** benchmark design only. None of these fault states has been injected into or diagnosed on a physical washer in this project. The curriculum is coverage-balanced, not weighted by inferred prevalence.  
**Tracker:** [WASHER-FAULT-CURRICULUM-001 #1875](https://github.com/Luminous-Dynamics/symtropy/issues/1875)  
**Parent work:** [washer reference architecture](WASHER_REFERENCE_ARCHITECTURE_V0_1.md), [decision engine](REPAIRABILITY_ARCHITECTURE_DECISION_ENGINE_V0_1.md), [stakeholder profiles](STAKEHOLDER_CONSTRAINED_REPAIRABILITY_FRONTIERS_V0_1.md).

## 1. Research input and its limit

A published study analyzing about 11,000 diagnoses from an independent repair centre reported recurring washing-machine fault classes including electronics, shock absorbers, bearings, doors, carbon brushes and pumps. Its observed repairability varied by class; bearings/drums/tubs, circulation pumps and electronics were among lower-repair-rate classes in that dataset. See [Understanding lifetimes and failure modes of defective washing machines and dishwashers](https://pmc.ncbi.nlm.nih.gov/articles/PMC6472650/).

That is useful evidence for **which difficult fault families should be represented**. It is not a calibrated South African failure distribution, not a reliable prevalence prior for a model we have not selected, and not proof a service-first design can repair these faults successfully. Repair-centre referrals, model/age mix, reasons for discarding devices and local parts/salary markets can change the observed rates substantially.

Accordingly, v0.1 demands coverage across all eight task classes. It does not weight test frequency according to the paper's incidence estimates. Local weights can only be considered later, after prospective repair records from the target market have documented selection, missingness and unit denominators.

## 2. Eight fault families to cover

| ID | Family | Diagnostic ambiguity to preserve | Architecture trade under test |
|---|---|---|---|
| WFT-01 | User-accessible filter / foreign object | Blocked filter vs hose restriction vs pump fault | User access, water containment, filter clean-out, referral clarity |
| WFT-02 | Door closure / interlock / seal | Obstruction or alignment vs lock vs seal vs other control fault | Access to visible wear items; safe boundary around internal interlock |
| WFT-03 | Drain path / pump | Blockage vs impeller/pump failure vs level-control issue | Replace filter/impeller/module vs larger assembly; misdiagnosis waste |
| WFT-04 | Inlet valve / water-level sensing | Supply/filter issue vs valve vs sensor/harness | Accessible external checks, exact-fit parts, qualified internal diagnosis |
| WFT-05 | Suspension / imbalance / vibration | Uneven load or leveling vs shock absorber vs bearing/tub | Access and part scope balanced against vibration, noise and rework |
| WFT-06 | Heater / temperature sensing / control | Programme/supply conditions vs sensor vs heater vs control | Repairability of internal assemblies without unsafe open-mains testing |
| WFT-07 | Drive / motor / electronics | Belt/coupling vs motor vs speed feedback vs drive/controller | Repair part vs controller/drive module; cost and electronic waste |
| WFT-08 | Bearing / tub / drum | Foreign object vs suspension vs bearing vs drum/tub defect | Bearing/seal replacement vs serviceable tub vs larger tub assembly |

These are benchmark families, not assertions that every washer uses the same internal parts or that these are the most common faults in South Africa.

## 3. A task is a diagnosis-to-restoration sequence, not a fault code

Each task record includes:
- one or more plausible competing hypotheses and the observed symptom;
- safe, discriminating checks that could distinguish hypotheses without defeating protection;
- candidate repair scopes from component-level work to qualified assembly replacement;
- service-role boundary, required isolation/guarding and stop/referral conditions;
- post-repair functional and, where applicable, electrical/leak checks;
- recurrence follow-up state and observation window;
- required raw observations, actual parts and exact unit/procedure identifiers.

Do not use a fault code as ground truth. Diagnosis is supported only by discriminating observations and the eventual confirmed cause where confirmation is possible. A result can be correct abstention, indeterminate or referral—not just “diagnose a specific part.”

## 4. Keep the test levels separate

**Synthetic diagnostic contract:** test whether the decision path rejects missing, stale, contradictory, low-confidence or unsafe observations; preserve positive, negative and indeterminate controls. Synthetic signals test software logic only.

**De-energized access/teardown:** document fasteners, access order, service interfaces, required tools, damaged clips, assembly replacement scope, and mass discarded. A dead donor supports descriptive accessibility study, not operational performance comparison.

**Qualified physical fault trial:** a qualified technician introduces or works on an approved fault state using an approved safety plan, then completes the exact diagnostic/repair task. Never operate with covers/guards removed, bypass interlocks, or expose live mains for this benchmark. Internal mains, heater, drive, stored energy and heavy tub/bearing tasks remain qualified-service work.

**Post-repair validation:** run the preregistered functional check, required leak/electrical/guard checks and a defined follow-up window. Powered-on status alone is not proof of restoration. Recurrence follow-up is pending until the full window is complete.

## 5. Measurements and proposed gates

Record, separately for every task and candidate:
- diagnosis outcome: correct cause, incorrect cause, unresolved, safe abstention/referral;
- discriminating-test count, diagnosis time and total restoration time (hands-on and elapsed);
- task completion, part ordered vs actually installed, compatibility receipt, part delay and shipping;
- repair scope: component, service module, major assembly or replace appliance—with reason and uncertainty;
- post-repair function, appropriate safety checks, rework and repeat visits;
- repair cost split into parts, labour, freight, failed attempts and rework;
- discarded mass, recovered/reused parts and packaging where measurable;
- recurrence state: not due, pending, complete-without-recurrence, recurrence, failed follow-up or inconclusive.

**Non-compensatory gates:** unsafe next action or failed critical check rejects the task/design; required function failure prevents efficiency or restoration claims. An architecture does not win by averaging safety failures against faster easy tasks.

### Initial benchmark aims (not measured achievements)

- **Full diagnostic coverage:** every candidate architecture faces every applicable task; a not-applicable task requires a documented architectural reason, never a silent omission.
- **No false certainty:** zero cases in the finite negative/indeterminate safety corpus where missing or conflicting evidence yields a confident unsafe repair directive.
- **Post-repair proof:** 100% of claimed successful trials include the required task-specific functional and safety receipt; missing receipts are not successes.
- **First-visit restoration and recurrence:** report observed ratios with denominator and uncertainty; freeze numeric targets after the control-only pilot and before candidate comparisons.
- **Economics:** compare the smallest justified repair scope against component/module/assembly replacement including part scarcity, freight, failed attempts, labour, rework, downtime and waste.
- **Coverage is not prevalence:** report macro-averaged task-class results and per-class outcomes first. Do not let easy filter/door tasks hide poor bearings, pumps or electronics performance in a single average.

Any further numeric claim—such as a target diagnosis accuracy or restoration-time reduction—requires a defined user/technician population, task mix, sample plan and frozen acceptance margin. Do not back-fit the margin to candidate results.

## 6. What could make this genuinely field-changing

1. **Diagnosis quality becomes measurable:** tests should reward asking the one discriminating question, safe abstention and correct referral—not confident guessing or indiscriminate module swaps.
2. **Repair scope becomes an economic choice:** benchmark the smallest safe, technically defensible intervention against a full-module and major-assembly replacement.
3. **The difficult 20% cannot be averaged away:** report by fault family, especially expensive/low-restorability classes, as well as aggregate outcomes.
4. **Repair claims follow through:** link exact unit, part, procedure, post-check and bounded recurrence via existing Mycelix evidence owners, not a duplicate ledger.
5. **Real local data can replace assumptions:** once qualified repair records exist, use their documented selection/coverage limitations to update curriculum emphasis and Symthaea's existing EIG/EVSI hypothesis priors. Never take a repair-centre distribution and silently treat it as the target-market population.

## 7. References

- [Knobloch et al., Understanding lifetimes and failure modes of defective washing machines and dishwashers](https://pmc.ncbi.nlm.nih.gov/articles/PMC6472650/) — approximately 11,000 repair-centre diagnoses; useful fault-family coverage, not South African prevalence.
- [Rodriguez, Peeters and Favi, Design for repair: insights to develop functional modules in household appliances (2025)](https://doi.org/10.1016/j.procir.2024.12.004) — design-for-repair solutions informed by functional-module analysis and physical disassembly tests in several cooking-appliance categories; findings require validation before transferring to washers.
- [IEC 60456:2024](https://webstore.iec.ch/en/publication/70049) — performance measurement for household washing machines; use applicable current requirements and test methods, not this curriculum, for formal performance qualification.

## 8. Claim ceiling

This curriculum makes repair-task coverage explicit. It establishes no local fault frequencies, repair rates, product-specific diagnostic accuracy, repair savings, endurance, product safety or conformity. Those require the physical unit, frozen methods, raw evidence and appropriate independent review.
