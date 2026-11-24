# Manufacturing Policy Challenge Corpus v0.2

**Benchmark:** MFG-POLICY-V02-CHALLENGE-001  
**Generator:** mfg-policy-v02-challenge-generator/0.2.0  
**Evidence class:** synthetic, offline  
**Owner:** [#7189 — MFG-POLICY-HARDNESS-001](https://github.com/Luminous-Dynamics/symtropy/issues/7189)

## Purpose and claim ceiling

This is a deterministic challenge-corpus generator, not a scheduler and not a result claiming that an adaptive policy beats a heuristic or industrial APS. It replaces the structurally trivial two-job, three-period v0.1 smoke corpus with a broader multi-operation, multi-resource, four-epoch benchmark design.

The generator emits 120 cases:
- six scenario families;
- 20 replication seeds per family;
- four entire development families (80 cases);
- two entirely held-out families (40 cases);
- a static policy-visible case corpus with no scenario identifiers or future event trajectories;
- a separate simulator-only sidecar containing four probability-weighted scenario streams per case;
- a third evaluator-only sidecar keyed by case and scenario IDs, carrying hidden terminal-quality outcomes.

Every policy in a future tournament receives the same static case bytes and decision-time budgets. The simulation harness—not the policy process—owns complete scenario streams and reveals only the current observation prefix at each epoch. Evaluator labels are joined only after actions have been committed and replayed. Replication seeds are paired across policy arms; policies may not receive complete future trajectories, scenario IDs, or evaluator-only terminal quality.

The family split is declared before case generation. The held-out families must never be used for policy selection, parameter tuning, model selection, or deciding which result to report.

The crossed-bottleneck development family gives RESOURCE-A and RESOURCE-B complementary capacity calendars (A is unavailable in epoch 1; B is unavailable in epoch 2), while their eligible operation sets overlap. This creates interacting time-window constraints rather than a single permanent scarce-machine toy pattern. Each operation's process family maps to an explicit setup family, and each resource has a complete directed transition matrix. The validator checks that operation setup targets, eligibility, processing-time keys and resource capability scopes agree.

The generator uses an explicit SplitMix64-v1 seed derivation. Its root seed, seed derivation, pairing rule, split rule and tuning rule are serialized into the policy-visible corpus. Re-running the generator must produce byte-identical policy JSON and byte-identical evaluator-sidecar JSON.

The default CLI output is the policy-visible corpus only; the simulation stream sidecar requires the explicit `--scenario-streams` argument and evaluator labels require `--evaluator-labels`. The schema and validator check that every sidecar case, scenario and operation maps to the policy corpus. A negative-control test serializes the policy input and asserts that hidden outcome fields and IDs are absent. The focused workflow generates both outputs twice and checks exact-byte determinism separately.

## Scenario families

| Family | Split | Mechanism | Decision tension under test |
|---|---|---|---|
| crossed-bottlenecks | Development | Overlapping eligibility and interacting capacity limits | Proactive reservation versus local greedy dispatch |
| common-cause-outage | Development | Two resources lost and restored under one failure domain | Resource diversity versus nominal processing cost |
| uncertain-recovery | Development | Outage duration uncertainty and delayed source observations | Bounded hold versus premature reassignment |
| qualification-transition | Development | Unknown resource qualification admitted only after a scoped receipt becomes visible | Defer Unknown routes until admissible evidence is visible |
| setup-wip-pressure | Held out | Sequence-dependent setups, committed and in-process work | Preserve WIP versus setup-aware rerouting |
| inspection-value | Held out | Inspection-capacity contention and hidden terminal quality | Inspect when measurement can change disposition |

The decision-tension field declares a research question, not a known optimal action or a claim that the generated instance necessarily rewards one policy. The exact oracle and policy tournament must establish whether and when the tension has measurable decision value.

## Instance structure

Each static policy case contains:
- three jobs with two precedence-linked operations each;
- three shared resources;
- four discrete decision epochs;
- overlapping resource eligibility (each operation has two eligible resources);
- per-resource processing times;
- resource capacity calendars;
- operation-specific setup families linked to each eligible resource;
- a complete directed setup-transition matrix per resource (all six non-identity transitions among three setup families);
- initial setup families;
- one in-process, resource-committed operation and one completed predecessor;
- explicit Unknown / Admissible qualification dispositions with evidence receipt identities;
- an inspection station with finite per-epoch capacity and multiple methods;
- a complete static planning context but no scenario IDs, event trajectories or evaluator labels.

The separate simulator-only sidecar carries four probability-weighted scenario streams per case. The simulator advances one epoch at a time and exposes only the corresponding observed event prefix. The policy executable must not mount the full stream sidecar. The evaluator-only sidecar contains terminal quality outcomes and is mounted only by the independent evaluator after policy decisions have been recorded/replayed.

Two resource families share POWER-DOMAIN-1, permitting explicit common-cause outages. Uncertain recovery is represented by a forecast interval on a visible failure event followed by a later restoration event. A late event preserves occurred_at_epoch separately from observed_at_epoch; the historical decision prefix is never rewritten.

The qualification-transition regime begins with RESOURCE-C / FORMING as Unknown. A scoped admission receipt occurs at epoch 1 but becomes visible only at epoch 2. A policy may not consume that qualification before its receipt is visible.

The inspection-value held-out family introduces required independent inspection on selected final operations and a synthetic inspection-capacity loss. The policy corpus contains only static job/resource/process configuration. In the scenario-stream sidecar, the first two scenario streams in each case have identical observable histories. In the evaluator sidecar, those stream IDs map to different terminal-quality dispositions. The evaluator joins those labels only after policy actions have been committed/replayed; the policy process receives neither complete trajectories nor terminal-quality fields.

## Strict invariants and negative controls

The generator validates its output against a Draft 2020-12 JSON Schema and Rust types that deny unknown fields. Its tests assert:
1. exactly six families, 20 seeds per family, 80 development cases and 40 held-out cases;
2. whole-family holdout (4 development families, 2 held-out families);
3. three jobs, three resources, six operations and four epochs per case;
4. overlapping eligibility and operation precedence;
5. explicit in-process commitment and remaining work;
6. nonzero sequence-dependent setup transitions;
7. event occurrence time never later than event observation time;
8. common-cause identity for correlated outages;
9. Unknown-to-Admissible qualification transition with a later visible receipt;
10. inspection-capacity contention in the held-out inspection regime;
11. paired scenario streams have identical early histories but different evaluator-only terminal dispositions;
12. scenario streams referencing unknown cases, resources or event scopes are rejected;
13. evaluator labels referencing unknown cases, scenario IDs or operations are rejected;
14. policy corpus serialization contains no scenario IDs, event histories or evaluator-label fields;
15. scenario stream serialization contains no evaluator-label fields;
16. all three JSON planes are deterministic under repeated generation.

These checks validate corpus construction only. They do not independently solve or replay a production schedule, prove that a generated action is feasible, or establish that any policy wins.

## v0.2 policy tournament protocol

### Required comparators

Use matched instances and matched online compute budgets for:
- SPT, EDD, least slack, most work remaining and critical-ratio dispatch;
- cheapest feasible and scarcity/deadline-aware dispatch;
- bottleneck-first and limited-lookahead rules;
- fixed-interval rolling horizon;
- event-triggered rolling horizon;
- full re-optimization;
- incumbent-aware selective repair, including the SLR2-style immutable / kept / released partition and reduced-neighborhood CP-SAT repair;
- precedence × capability/compliance × availability masks;
- nominal expected-cost and CVaR policies;
- finite ambiguity-set / worst-case policies;
- exact exhaustive/DP oracle on a separately declared tiny subset.

A clairvoyant wait-and-see policy is an information upper bound only, never a deployable comparator. All policies must use the same observable history, evidence cutoff and event stream. Tuning budgets must be reported and restricted to development families.

### Required scorecards

Keep these outcomes separate:
- makespan, tardiness, due-date service and backlog;
- accepted throughput, time to first independently accepted output and recovery-target time;
- right-censored recovery curves and restricted mean time to event;
- changed-operation ratio, changed-machine ratio, repair scope and timing displacement;
- capacity, precedence, setup and frozen-commitment violations;
- qualification false-admission, stale/Unknown handling and evidence coverage;
- inspection spend, measurement delay, false acceptance/rejection and rework;
- P50/P95/CVaR loss and worst-family regret;
- online decision latency, compute, memory and human escalation.

Do not report only a pooled average. Retain per-instance paired differences, family-stratified uncertainty intervals, worst-family regressions and failed / infeasible / abstaining runs.

### Exact-oracle and statistical gates

Before claiming an advantage:
- define a tiny-instance subset whose optimal policies can be independently enumerated;
- verify non-anticipativity from observable histories, not caller-supplied information-set labels;
- independently replay every decision, capacity use, precedence relation and objective;
- compare exact objective values against a separately implemented oracle;
- preregister primary outcomes and multiplicity treatment;
- use at least 20 paired seeds per family for initial held-out evaluation and increase samples when intervals are too wide;
- report paired effect sizes and confidence intervals;
- retain all timeouts, infeasible cases and abstentions in the denominator.

The v0.2 generator does not yet implement a simulator that executes policies epoch by epoch, setup-aware schedule execution, rework flows, an APS adapter, the selective-repair solver, exact DP, or statistical inference. Those remain explicit next work, not implied features.

## External benchmark and industry positioning

### Static scheduling

[FJSPLib](https://scheduleopt.github.io/benchmarks/fjsplib) is the appropriate public static flexible-job-shop reference. It documents 138 flexible-job-shop, 264 machine-independent and 50 flexible-assembly instances and a standardized 10-minute engine comparison including CP Optimizer, CP-SAT, CPLEX and OptalCP. Reproduce the published instance/objective/budget protocol through the static tournament; do not compare v0.2 synthetic dynamic-case scores directly against FJSPLib.

### Dynamic rescheduling

Lin and Yang's November 2026 SLR2 paper, [An incumbent-aware selective-repair framework for dynamic flexible job shop scheduling](https://doi.org/10.1016/j.cie.2026.112299), is a mandatory dynamic comparator. It partitions active-window operations into immutable, kept and released subsets, then applies reduced-neighborhood CP-SAT repair under online budgets, with a learned keep/release selector. The paper reports its own improvements on its own evaluation. Those numbers are author-reported and are not reproduced by this generator or directly comparable to v0.1. Reproduce the method or clearly label any approximation and deviations before making a competitive claim.

### Industrial scheduling and manufacturing digital twins

[SAP PP/DS](https://help.sap.com/docs/SAP_S4HANA_ON-PREMISE/f899ce30af9044299d573ea30b533f1c/4133c95360267614e10000000a174cb4-1590.html) already performs detailed resource scheduling with resource and component availability. [DELMIA Ortems](https://www.3ds.com/products/delmia/ortems/production-scheduler) offers finite-capacity multi-resource scheduling and rescheduling. The initial Symtropy position should therefore be an assurance / intelligence overlay around existing APS/MES, not a premature replacement claim.

[NIST's Super Intelligence for Manufacturing programme](https://www.nist.gov/programs-projects/super-intelligence-si-manufacturing) is an especially relevant alignment target because it is developing integration test benches, contextual grounding, human-machine teaming and digital-twin VVUQ evaluation. It is an ongoing programme, not an endorsement or an available result leaderboard. [ISO 23247-5:2026](https://www.iso.org/standard/87425.html) addresses the manufacturing digital thread, while [ISO 23247-6:2026](https://www.iso.org/standard/87426.html) addresses digital-twin composition and interoperability. This JSON corpus does not establish ISO conformance.

## Validation and status

The dedicated workflow checks formatting, validates all three schemas and their cross-plane identity/scope invariants, runs adversarial tests, independently generates the policy corpus, simulation streams and evaluator labels twice with byte-identical comparisons, runs policy-input leak negative controls, then runs strict Clippy. The full repository CI remains a separate qualification boundary.

The generated corpus is synthetic and offline. It does not establish real provider reliability, source authenticity, qualification authority, physical product acceptance, factory recovery, savings, production authority, industrial readiness or SOTA superiority.

**Blind-test limitation:** this public generator's root seed and derivation are published, so the evaluator sidecar can be deterministically regenerated by anyone with the repository. The separate CLI output prevents accidental policy-process access when the harness supplies only the policy corpus, but it does not provide secrecy or a cryptographically sealed holdout. A prospective blind performance claim must use evaluator-held entropy or an independently generated held-out corpus whose seed, labels and digest are withheld from policy developers until the final evaluation. The current two held-out families are structural regime holdouts, not a secret test set.
