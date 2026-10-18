# Proof-Carrying Repair Outcomes and Recurrence-Aware Benchmarks v0.1

**Status:** benchmark and evidence-contract proposal. The included ledger is synthetic contract data only; it is not a repair trial, a safety assessment, or evidence that any architecture is superior.  
**Tracker:** [REPAIR-OUTCOMES-001 #1908](https://github.com/Luminous-Dynamics/symtropy/issues/1908). Related: [large-appliance portfolio #1825](https://github.com/Luminous-Dynamics/symtropy/issues/1825), [decision engine #1862](https://github.com/Luminous-Dynamics/symtropy/issues/1862), [fault curriculum #1875](https://github.com/Luminous-Dynamics/symtropy/issues/1875), [cross-appliance morphology #1893](https://github.com/Luminous-Dynamics/symtropy/issues/1893), [draft PR #1827](https://github.com/Luminous-Dynamics/symtropy/pull/1827).

## 1. Paradigm shift: repairability is an outcome, not an opening angle

A repairability score or disassembly-depth measure is useful for screening. It does not establish that the fault was diagnosed correctly, that a repair restored the useful function, that safety tests passed, that the total restoration cost was acceptable, or that the fault stayed fixed. Research comparing reparability scoring systems with measured repair times has raised questions of criterion validity; a separate study of disassembly scoring identified limitations in translating scores into design decisions.

- M. F. et al., **Empirical evaluation of reparability scoring systems for validity and reliability** (2025), *Resources, Conservation & Recycling*, [doi:10.1016/j.resconrec.2025.108211](https://doi.org/10.1016/j.resconrec.2025.108211).
- **Analysis of disassembly parameters in repairability scores** (2023), *Procedia CIRP*, [doi:10.1016/j.procir.2023.02.124](https://doi.org/10.1016/j.procir.2023.02.124).
- The EU tumble-dryer repairability index is intentionally an aggregated score built from disassembly depth, fasteners, tools and repair information; it is a useful policy-specific reference, not a substitute for Symtropy's outcome evidence and not a universal South African compliance rule: [consolidated Commission regulation](https://eur-lex.europa.eu/legal-content/EN/TXT/HTML/?uri=CELEX%3A02023R2534-20251124).

**New design unit: the restoration episode.** Each episode joins fault identity, competing diagnoses, safety boundary, intervention, part/labour delay, functional and safety verification, cost coverage, and recurrence follow-up into an auditable chain. The objective is not to require more paperwork for its own sake; it is to make each comparative claim reproducible and prevent favorable metrics from hiding failed repairs.

## 2. Non-negotiable accounting rules

1. **Denominator integrity.** Every started repair attempt stays in the attempted-repair denominator, including failed repairs, abandoned work and attempts stopped for safety. Report not-started diagnosis/referral separately; never report only successful jobs.
2. **Handoff success is not durability.** A repair may pass the immediate test and still recur. Report immediate verified restoration and durable success separately.
3. **Censoring is not success.** Lost-to-follow-up, pending, and right-censored episodes carry an unknown recurrence result. They cannot be coded as “no recurrence.” Follow-up includes calendar time *and* a product-appropriate use exposure such as washer cycles, dryer cycles, operating hours or heat-pump operating hours.
4. **Hard gates are non-compensatory.** Useful function and required safety checks must pass for a positive restoration claim. Better energy/water, lower part cost, or faster disassembly cannot compensate for failed core function or failed safety checks.
5. **Unknown is not zero.** Unknown cost fields are `null`; measured zero is numeric zero. Partial and complete cost coverage are distinct states.
6. **Cost the whole episode.** Track parts, active labour, queue delay, parts delay, travel, freight, failed attempts, repeat visits, rework, downtime, waste and material discarded. Where monetary downtime cannot be defensibly monetized, report hours separately and do not silently assign zero.
7. **Protocol before outcome.** Freeze task, control, useful-output threshold, test sequence, follow-up horizon/exposure, exclusions and analysis method before testing candidate units. Each evidence pack includes a protocol identifier/hash and measurement receipts.
8. **Claim level follows evidence level.** Schema qualification, simulation, teardown, bench tests, independent replication and field follow-up are separate rungs. A synthetic fixture or a passed mutation test cannot promote a product claim.

## 3. Benchmark suite and initial targets

All numeric targets below are **proposals**, not frozen commitments or results. First run the same frozen task set on at least three physical units of the same control model/revision to estimate repeatability and instrument uncertainty. Choose the candidate/control and analysis before seeing candidate results; freeze acceptance margins based on control repeatability and useful-output needs.

| Benchmark | Initial target hypothesis | Guardrail |
|---|---|---|
| Median time to verified restoration | At least 25% lower than matched control | Report active labour, queue and parts delay separately; include all started attempts |
| P90 time to verified restoration | At least 20% lower than matched control | Publish sample size and uncertainty interval; do not hide the long tail |
| Same-fault recurrence | At least 20% relative reduction after defined time and use-exposure windows | Report censoring, recurrence type, follow-up completeness and product use |
| Diagnostic safety | Zero false-safe recommendations on a frozen finite hazard/contradictory-control corpus | Finite software corpus result is not product safety certification |
| Positive restoration evidence | 100% of claimed restorations have both core-function and safety receipts | Missing receipts invalidate the positive claim |
| Approved user maintenance | At least 90% task completion or correct safe referral | Include representative users; unsafe attempts reported separately |
| Whole-episode cost per useful output | Lower than control | Only compare after hard function/safety/durability gates; include failed work and downtime |
| Resource efficiency | Lower kWh/water per useful output | Only eligible if the preregistered useful-output function floor passes |

Use paired or stratified comparisons by fault family, appliance revision, skill tier, and user/service profile. Report median and P90 times, confidence intervals, per-task results, failure taxonomy and raw denominators. Avoid a universal weighted sum: retain per-stakeholder Pareto frontiers and non-compensatory gates from the decision-engine work.

## 4. Minimum event contract

The ledger at `fixtures/appliances/repair_restoration_evidence_v0_1.json` illustrates:

- Stable pseudonymous unit/model/revision tokens and family-specific fault/task IDs.
- Competing diagnostic hypotheses and a declared hazard-screen disposition.
- Whether a repair attempt actually started, the service role, repair action, parts, active labour, queue wait, parts wait and travel.
- A verified useful-output metric, required-versus-passed tests, explicit function and safety gates, and measurement receipt IDs.
- Follow-up status, calendar duration, operating exposure and a tri-state recurrence result.
- Whole-episode costs with explicit complete/partial/unknown coverage, plus preregistration hash and evidence receipt IDs.

The sample episodes are deliberately synthetic examples to exercise the contract: four illustrate started attempts (including one failed repair); three illustrate successful handoff verification; two show completed follow-up; one recurrence is represented; one “restoration” has lost follow-up. The Rust report labels all such counts as examples and hard-reports **zero empirical episodes**.

## 5. Statistical and data-integrity requirements

- Predeclare the primary endpoint, observation window, practical difference and treatment of repeated repairs. Avoid treating several visits to one physical unit as independent units.
- Record an appliance-unit identifier and model/revision; cluster uncertainty by physical unit where repeated episodes exist.
- Separate first-attempt success, all-attempt eventual success, repeat-visit burden and durable success; state the denominator beside every rate.
- For recurrence, publish follow-up completeness and time/use exposure. Where sample size supports it, use survival analysis with right-censoring rather than counting incomplete follow-up as success.
- Report missingness by field and reason; do not impute cost, time, recurrence, or useful output as zero.
- Preserve raw measurement receipt identifiers, preregistration digest, schema/version digest and a reproducible analysis command. Hashes prove identity of bytes, not truth or independence.
- Use privacy-preserving unit tokens. Do not place customer names, addresses, contact details or unnecessary technician personal data in the public ledger.

## 6. Low-capex execution ladder

1. **Contract gate:** schema, semantic invariants, mutation tests and deterministic digest report.
2. **Desk validation:** freeze fault/tasks, parts-fit evidence, control revisions and required safety/function tests.
3. **Control-only pilot:** establish measurement repeatability on the same model/revision; no candidate ranking yet.
4. **Matched bench trial:** test control and candidate under the same declared workload, instruments and role boundaries. Keep safety-critical/refrigerant/electrical tasks within qualified scope.
5. **Blind evidence review:** independently check restoration receipts, exclusions, recurrence coding and denominator integrity.
6. **Follow-up:** report both immediate restoration and time/use-adjusted durability. Only then consider a design change a demonstrated improvement.

## 7. Current boundary

This v0.1 work creates an evidence format and a software contract audit. It does **not** claim a prototype exists, a repair became easier, any physical repair succeeded, costs were reduced, product energy/water improved, any hazard is safe, or any product meets a jurisdiction's law. Numeric targets remain hypotheses until the control protocol and physical measurements are frozen.