# AERB: blind evaluation, recovery assurance and competitive benchmark plan v0.1

Status: research protocol and software candidate. The projection/audit module and CLI are authored on the associated PR branch; exact-head Rust qualification is pending. This document is a benchmark design, not a claim of SOTA performance, safe physical deployment, blind-service certification, HIL qualification or formal proof of upstream evidence truth.

## 1. The field gap

The relevant literature is moving beyond clean-start task success:

- **RoboRecover** (2026-09-24) reconstructs execution-induced deviation states by replaying action prefixes. Its benchmark includes 2,000 scenarios split across RoboTwin and LIBERO, with 800/200 train/test per platform. On LIBERO, UnifoLM's 98.83% initial-state success corresponds to 48.00% recovery, while pi0.5 reaches 64.40% recovery despite 94.17% initial-state success. Recovery is a distinct capability, not a derivative of clean-start score.
- **DiMuRe** (2026-11-03) reports 77.65% complete-task success for Local DiMuRe versus 58.68% for the original program with the same physical-step allowance. Full DiMuRe reaches 81.90% after extra step allowance, terminal replanning and early handoff. Only the first comparison is matched-budget.
- **SafeVLA-Bench** (2026; leaderboard last updated 2026-09-26) preserves host rollouts and scores eight task-aware temporal-safety families. It reports native success (SR), all-spec safety, successful-but-unsafe episodes (SBU), and worst-violation severity (VSI). Its published results show 13–15% unsafe episodes among high-success LIBERO baselines and 36–56% of RoboCasa-365 successful rollouts violating an active clause in the overview.
- **ActiveArena** (2026-09-21) contributes 35 active-perception tasks across five categories, multi-round information acquisition, memory annotations and ID/OOD scene protocols.
- **EmbodiedGovBench** (2026-04-13) makes governability measurable across seven dimensions, including capability boundaries, runtime drift, recovery, policy portability, upgrade safety, human override and audit completeness.

References:
- RoboRecover: https://arxiv.org/abs/2609.28952
- DiMuRe: https://arxiv.org/abs/2611.04226
- SafeVLA-Bench: https://arxiv.org/abs/2606.00773 and https://safevla.org/
- ActiveArena: https://arxiv.org/abs/2609.24124
- EmbodiedGovBench: https://arxiv.org/abs/2604.11174

## 2. The paradigm shift

AERB should be an **assurance benchmark that can be layered onto recovery benchmarks**, not a replacement for LIBERO, RoboTwin, RoboCasa, RoboRecover, LIBERO-Disturb, ActiveArena or SafeVLA-Bench.

The central unit is not a completed task. It is a **decision under unresolved execution history**:

1. What action was authorized, admitted, queued, dispatched and applied?
2. What effect multiplicity and scope are still possible?
3. Which observations are admitted, stale, unknown or disputed, and which source dependencies are genuinely independent?
4. Which new observations are worth acquiring, including the cost and physical risk of the probe itself?
5. Is there a physically qualified path to continue, hold, abort or stop observing?
6. Can the system prove a new effect is authorized at the native linearization point, or must it hold?

This produces five distinct research contributions:

### 2.1 Effect semantics over acknowledgement semantics

A transport acknowledgement is not a physical-effect receipt. Keep dispatch, actuator acceptance, physical effect, multiplicity, authority epoch, remaining command holders and reconciliation state distinct. Unknown is a first-class outcome, not a retry policy.

### 2.2 Open-world, set-valued belief over closed-world confidence

Maintain possible-world support and an explicit residual for unmodelled histories. Do not silently normalize incomplete support into a posterior. Use calibrated expected value only where priors and likelihoods are justified; otherwise use robust worst-case decision loss and explicit abstention.

### 2.3 Sensing as an action with a physical trajectory

A probe is not free information. It consumes time, energy, actuator authority and possibly safety margin. Evaluate preemptible probes with checkpoints, partial evidence, safe stopping and abort routes. Cancellation of a software task is not proof that the physical command is closed.

### 2.4 Authority at the actual effect boundary

Freshness checked before enqueue is insufficient. The final effect sink must atomically consume an authority/ticket version at the native application point. If revocation linearizes first, the sink call count must be zero. If application linearizes first, preserve that historical event and reconcile it. An unknown sink outcome remains unknown.

### 2.5 Blindness as an information-flow property

A candidate can infer truth from file order, opaque-ID suffixes, serialized lengths, clock offsets, filenames, hashes, cluster membership, error/log text, process environment, resource behavior or stateful campaign history. The protocol therefore needs:
- candidate/evaluator data separation;
- campaign-salted pseudonyms and safe relative clocks;
- no raw evaluator identifiers in candidate files or paths;
- randomized assignment and order;
- grouped held-out metadata-only attacks;
- stronger adaptive/stateful attack suites;
- commitment/reveal and independent custody;
- candidate runtime isolation and a post-run surface audit.

The present implementation is only the first diagnostic layer. A low metadata-classifier score is not proof of no leakage, and the new projection function is not yet wired into the legacy public deterministic campaign generator.

## 3. Benchmark architecture

| Track | Question | Primary host benchmark / fixture |
|---|---|---|
| AERB-Core | Can the policy retain the true possibility and avoid unsafe retries under ambiguous completion? | Exhaustive process-model worlds and mutation tests |
| AERB-Recovery | Can it re-establish task conditions from an execution-induced deviation? | RoboRecover, RoboTwin, LIBERO |
| AERB-Disturb | Does it distinguish commanded disturbance, actual disturbance, partial effect and task recovery? | LIBERO-Disturb / DiMuRe-compatible scenarios |
| AERB-Probe | Does it acquire decision-relevant evidence while preserving a qualified stop/abort route? | ActiveArena plus probe-specific process/simulator worlds |
| AERB-Safety | Does task success coexist with safety-spec violations or unsafe continuation? | SafeVLA-Bench / RoboCasa-365 signals |
| AERB-Governance | Does revocation, override, runtime drift, upgrade and audit remain enforceable? | EmbodiedGovBench-inspired governance scenarios |
| AERB-Blind | Can any candidate-visible metadata predict hidden truth above a preregistered baseline? | Sealed evaluator-only corpora, grouped and stateful attacks |
| AERB-Bridge | Does the assurance layer compose with real controllers and robot middleware? | ROS 2 Control, hardware/firmware lifecycle, HIL then physical |

Every result must state its evidence tier: **formal/exhaustive, process model, simulator, HIL or physical**. Scores from different tiers must not be pooled.

## 4. Competitive baselines and fair comparisons

Evaluate at least these baselines on every applicable host split:

1. Open-loop continuation and ordinary retry-after-timeout.
2. Stop-and-human-reconcile / manual recovery.
3. Deterministic endpoint-journal reconciliation.
4. Greedy information gain or entropy reduction (as an intentionally imperfect probe baseline).
5. Expected-value probe selection with calibrated priors, only on distributions where those priors are justified.
6. Robust minimax / minimax-regret policy.
7. A clairvoyant evaluator oracle as an unattainable upper bound, never a deployable baseline.
8. The host benchmark's strongest published or reproducible policy: RoboRecover policy baselines, DiMuRe, SafeVLA benchmark policies, ActiveArena configurations, and EmbodiedGovBench reference controls as appropriate.

For DiMuRe-style comparisons, publish two tracks:
- **Matched budget:** identical physical steps, sensing calls, compute/token budget, wall-clock deadline and permitted recovery skills.
- **Expanded budget:** extra sensing, replanning, physical steps and human handoff disclosed as separate resources.

For RoboRecover, preserve the published train/test separation and source-policy grouping. Do not train or tune on test deviation states. Report per-platform results; do not collapse RoboTwin and LIBERO into one opaque average.

For SafeVLA, preserve the host rollout, signal extraction, task-aware applicability and per-episode verdict semantics. Report AERB outcomes alongside, not instead of, SR/Safety/SBU/VSI.

## 5. Metrics: no single score may hide a hard failure

### Task/recovery
- Native task success and recovery success.
- Progress preserved / progress lost.
- Recovery by deviation type, recovery stage, embodiment and source policy.
- Matched-budget success and expanded-budget success.
- Time, physical steps, sensing calls, inference tokens/compute and energy to recovery.

### Effect/evidence assurance
- False NoEffect claim rate.
- False effect-finality and false scope-closure rate.
- Duplicate/partial/stale-authority effect detection.
- Unsafe fresh-attempt recommendation count.
- Hidden-truth support coverage and open-world-residual retention.
- Stale/unknown/disputed evidence acceptance.
- Source-independence and common-cause detection.
- Decision receipt invalidation under late, reordered, revoked and superseded evidence.

### Probe policy
- Robust decision loss and loss regret against the evaluator oracle.
- Evidence value per joule, second, byte and actuator movement.
- Fraction of probes stopped once decision-sufficient.
- Time/energy to safe hold or abort.
- Partial-scope claim rate and false completeness rate.
- Unsafe probe continuation and unqualified stop-route rate.

### Safety / governance
- All-spec Safety, SBU and worst-violation severity (retain the host metric definitions).
- Unauthorized capability invocation and stale-authority effects.
- Override/revocation latency, including native apply races.
- Audit completeness and reproducibility.
- Upgrade/runtime-drift regression rate.
- Count all invalid, unresolved, timed-out and unscorable episodes as first-class outcomes.

### Blindness / adversarial integrity
- Metadata-only held-out accuracy and balanced accuracy, per-feature and combined.
- Grouped/temporal split leakage, not random row splits.
- File/path/order/size/hash/clock/identifier/error/log/process metadata attack success.
- Stateful carry-over and adaptive-query attacks.
- Candidate/evaluator secret exposure count.
- Commit/reveal mismatch, invalid artifact, and custody-violation count.
- Report attack confidence intervals and baseline class balance.

## 6. Proposed qualification gates (targets, not current results)

| Tier | Proposed gate |
|---|---|
| Formal/exhaustive | 100% finite-state oracle coverage; zero false NoEffect, false terminality, unsafe fresh-attempt branches or accepted invalid receipts in the enumerated fixture set |
| Software/process | 100% denominator conservation; invalid and unresolved cases never disappear; mutation tests detect each safety-critical field/guard removal |
| Simulator | Zero hard safety violations in the preregistered suite; recovery success non-inferior within 2 percentage points of the strongest matched-budget baseline before claiming efficiency gains |
| Simulator improvement | Aim for +5–10 percentage points recovery at the same budget, or maintain recovery within 2 points while reducing probe/compute/physical-step cost by at least 20%; publish Pareto curves rather than a composite |
| Probe policy | Robust loss regret <=5% of the evaluator dynamic oracle on the defined finite support; 100% required physical-supervisor path compliance |
| Blindness | Eight-class balanced chance is 12.5% only for an exactly balanced evaluation set; target the one-sided 95% upper confidence bound for metadata-only balanced accuracy to remain within 5 percentage points of the preregistered baseline on held-out groups |
| HIL | Zero unqualified native effect admissions and zero false terminality in the declared HIL envelope; independent safety supervisor remains authoritative |
| Physical | Start with reduced-speed/energy, nonhazardous tasks and independent interlocks; no promotion from simulator/HIL scores alone |

The numeric targets above are engineering hypotheses to preregister and challenge, not established industry thresholds or certification requirements.

## 7. Statistical power and denominator discipline

For zero observed failures in n independent Bernoulli trials, the one-sided upper confidence bound is:

p_upper = 1 - alpha^(1/n)

At 95% confidence, 3,000 independent zero-failure trials put the bound near 0.1%. If eight world classes need simultaneous familywise 95% coverage using a simple Bonferroni allocation, about 5,073 independent zero-failure trials per class are needed for the same <0.1% bound. This is not proof of safety. Trajectories sharing a seed, scene, robot, source policy, operator or physical environment are clustered, not independent; use cluster-aware bootstrap or hierarchical models and report per-stratum worst cases.

Never drop invalid, unresolved, aborted, timed-out or unscorable cases from denominators. Report scheduled, attempted, completed, invalid, unresolved and evaluated counts separately.

## 8. Immediate work sequence

1. Complete blind candidate projection and metadata-only leakage diagnostics (this PR).
2. Integrate projection into an actual CSPRNG-backed corpus materializer; keep evaluator truth, projection receipts and source-cluster mappings evaluator-only.
3. Add an evaluator-only surface scanner over every serialized file and path, not just typed JSON values.
4. Add temporal/stateful attack generators: ordering, replay, carry-over, side-channel timing and adaptive probing.
5. Connect probe checkpoint receipts to the physical supervisor and native apply linearization boundary.
6. Run matched-budget RoboRecover and DiMuRe-compatible evaluations; reproduce host baselines before comparing.
7. Add SafeVLA/EmbodiedGovBench mappings without relabeling their host scores.
8. Graduate process → simulator → HIL → physical only with explicit evidence-tier receipts and independent safety authority.

## 9. Current claim ceiling

The current AERB public deterministic generator is a conformance fixture, not a blind benchmark. The new blind assignment primitive creates opaque IDs and a committed evaluator reveal, but it is not itself an isolated service. The new projection/audit code is authored; its Rust tests, formatter, Clippy and canary workflow must pass on the exact current head before it can be called qualified. No blind benchmark result, SOTA superiority, robot safety certification, HIL qualification or physical qualification is claimed here.
