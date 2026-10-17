# Repairability Benchmark Protocol — v0.1

**Status:** preregistration template; not a completed physical study  
**Date:** 2026-10-17  
**Owner:** Symtropy repairability task-graph workstream #1773  
**Related contract:** [repair task graph schema](../../schemas/transport/repair_task_graph_v0_1.schema.json)

## 1. Purpose

Compare a candidate vehicle configuration with a named baseline using repeatable service tasks. Measure access and work effort, not just best-case wrench time. Report whether any improvement survives differences in technicians, task order, parts/tool availability, rework and post-repair checks.

This protocol is not a repair procedure. Each study task must have an applicable approved service procedure and risk review before technicians perform it. Do not create faults or bypass safety steps to make the comparison faster.

## 2. Preregister these fields before timing

Every benchmark package records:

- protocol ID/revision, preregistration timestamp and immutable digest;
- market, vehicle family/year/variant and exact configuration digests for baseline and candidate;
- repair-task graph IDs/digests, approved procedure references/revisions and procedure-use rights;
- technician eligibility, training/equipment, anonymized participant codes and conflicts of interest;
- tool inventory/calibration, workspace, parts status, environment, vehicle condition and known faults;
- task inclusion/exclusion rules, randomized/counterbalanced order, repetitions, stop conditions and primary/secondary outcomes;
- timestamp source, time-zone, measurement units, missing-data policy, censoring policy, uncertainty and analysis code revision;
- failure adjudication and independent reviewer.

Do not choose the baseline or drop slow trials after seeing the result. Any deviation gets a dated append-only amendment with reason and affected analyses.

## 3. Define the clocks before the first trial

Capture independent timestamps for these events. Do not collapse them into one number:

| Measure | Start | End | Includes |
|---|---|---|---|
| Diagnosis time | Technician starts the defined diagnostic task | Root cause/required action recorded to the preregistered criterion | Diagnosis work only; record waiting separately |
| Hands-on time | Approved task starts | Approved work step ends | Active technician labor; record parallel work by other technicians separately |
| Bay elapsed time | Vehicle enters the defined service bay/task state | Vehicle exits that state | Queue/interruptions only when explicitly marked; never substitute for labor time |
| Parts/tool wait | Need for unavailable part/tool is first recorded | Correct part/tool is available and verified | Lead time, procurement, travel and substitution as separate fields where applicable |
| Verification time | Approved post-work check starts | Its result is recorded | Check execution and troubleshooting after a failed check |
| Rework time | A defect/rework need is confirmed | Corrective work and successful required check complete | Kept separately and included in total ownership cost |
| Out-of-service time | Vehicle becomes unavailable for the task/defect | Vehicle is returned to the defined service state | Includes overnight/parts delays; report calendar elapsed time separately from staffed hours |

Specify event definitions more precisely for each task. Keep timestamps in a consistent format and record monotonic timing source where available. If a step is interrupted, preserve the interruption interval and reason instead of silently stopping or restarting the clock.

## 4. Study design and sample sizes

### Pilot (method calibration only)

Use at least 3 qualified technicians and 3 repetitions per technician per vehicle configuration per task where feasible. The pilot finds ambiguous event definitions, unsafe/irreproducible steps, instrumentation errors and task-order effects. **Do not use this small pilot alone to declare a P90 improvement or product victory.**

### Confirmatory comparison

Target at least 5 qualified technicians and 6 repetitions per technician for each baseline/candidate/task cell (30 observations per cell), subject to a preregistered feasibility and safety review. Pair technician/task experience where possible and counterbalance vehicle/task order. Randomize sequence within safety and practical constraints.

A power calculation based on pilot variance should supersede these planning defaults when a suitably defined primary outcome, minimum meaningful effect, type-I error and power are preregistered. If 30 observations are not feasible, mark the result exploratory and make confidence/precision limitations explicit. Never present repeated timing from one technician as independent technician diversity.

## 5. Outcomes

### Primary

For each preregistered task, report baseline and candidate:

- median diagnosis time;
- median hands-on labor time;
- median total bay elapsed time;
- median and P90 vehicle out-of-service time;
- first-pass post-repair verification success rate;
- rework incidence and rework time;
- task completion/failure count and every safety stop.

Report per-task deltas and uncertainty before creating any aggregated summary.

### Secondary

- parts and consumables cost, with currency and quote date;
- labor cost from dated local labor-rate source and declared treatment of parallel technicians;
- unavailable-part/tool waiting time;
- number of steps, required tools, special tools and supplier/repair-network concentration;
- task failures due to access, fastener damage, missing diagnostics, incompatible part, procedure ambiguity or other prespecified categories;
- waste/parts replaced, where the observation can be recorded consistently;
- technician-reported workload or usability, using a preregistered instrument rather than invented scoring.

### Uncertainty and reporting

For each metric show sample size, missing/excluded observations, median, distribution/quantiles and a confidence interval. Use a preregistered bootstrap or suitable paired/hierarchical analysis that respects technician and repeated-task clustering. P90 is descriptive at the pilot stage; confirmatory claims require adequate sample count and a disclosed interval. Show individual task results and the distribution of technician effects—not just the overall average.

## 6. Safety and validity gates (cannot be traded for speed)

A trial is not a successful faster repair if it:
- omits an approved isolation or safety precondition;
- uses an unapproved/incompatible part or tool;
- skips or fails required post-repair verification;
- returns the vehicle to service after an unresolved check failure;
- damages another component, conceals rework or records a failed trial as completed.

A qualified supervisor stops work on a safety concern. Record stopped/censored trials, causes, component damage and near misses. Report the denominator. No time saving offsets an increase in unresolved safety-critical failures.

A task graph that is structurally valid is not proof that the procedure itself is correct, authorized, complete or safe. An approved procedure and the required professional/engineering review govern the trial.

## 7. Analysis and economic linkage

Calculate time savings per task as both absolute difference and relative difference. Do not combine hours, money, safety status and utility into one opaque score.

For aggregate lifetime cost:
1. preregister the task-frequency/usage weights and explain their source;
2. bind each observation to exact vehicle, configuration, component and procedure revision;
3. include diagnosis, labor, parts, rework, verification, vehicle downtime and wait-time cost without double-counting;
4. preserve dates, currency, region, quote/source provenance and uncertainty;
5. conduct sensitivity analysis for task frequency, local labor rates, part lead time, repair incidence and discounting assumptions;
6. report Pareto tradeoffs: owner cost, vehicle availability, tool accessibility and rework alongside hard safety constraints.

The output may feed Symtropy whole-life transport optimization (#1601) as a versioned assumption/evidence set. It may not overwrite measured observations with optimizer estimates.

## 8. Provisional success target

A future candidate design may aim for at least **25% lower median and P90** on a preregistered high-impact service-task set relative to a matched baseline. That is a hypothesis, not a current achievement. Claim success only if:
- the task set and weights were fixed before comparative analysis;
- required safety checks and completion quality are non-inferior under a preregistered rule;
- no critical safety-step omissions occur;
- uncertainty and technician/task clustering are reported;
- parts availability, rework and downtime do not hide a regression elsewhere;
- an independent reviewer can reproduce the analysis from the disclosed, rights-cleared evidence package.

If the target is missed, report the measured result and redesign the access/component/task graph. Do not adjust the benchmark after seeing the answer.

## 9. Privacy, rights and publication

Use anonymized technician IDs, consented recording and the minimum necessary vehicle/owner data. Do not publish raw owner identity, registration/VIN, precise location, customer invoice, or identifiable video by default. Restrict raw observations to authorized study custodians and publish only rights-cleared aggregates/derived graphs.

Record manual, standard, instrument and dataset reuse conditions. A public URL does not itself grant rights to redistribute its contents. Do not copy proprietary repair manuals or normative standard text into fixtures.

## 10. Required artifact bundle

Before a result can be labeled *confirmatory* publish or retain, subject to privacy/rights constraints:

1. preregistration and its digest;
2. baseline/candidate configuration and procedure/graph digests;
3. raw timestamped observations under access controls and a de-identified export;
4. inclusion/exclusion ledger and deviation log;
5. analysis program/revision and reproducible derived report;
6. post-repair verification/failure records;
7. source/rights register and independent review;
8. explicit evidence class: `observed_workshop_trial` for the measured event, not a generalized roadworthiness/certification claim.

**Claim ceiling:** This document proposes a study method. It contains no real workshop measurements and establishes no repair-time reduction, commercial saving, workmanship, roadworthiness, crashworthiness or regulatory approval.
