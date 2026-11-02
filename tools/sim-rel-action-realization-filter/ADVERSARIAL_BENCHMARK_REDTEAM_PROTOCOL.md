# SIM-REL-053 — adversarial red-team certification for scientific benchmark integrity

Status: first typed red-team campaign verifier on a research branch. Exact-head qualification pending. It reports empirical attack/detection/containment counts and point estimates; it does not yet compute binomial confidence intervals or authenticate identities cryptographically.

## 1. Why benchmark integrity is now a first-class scientific result

SIM-REL-049 makes task generation adaptive and split-firewalled. SIM-REL-050 uses the scientific concept/recipe hierarchy as the inferential hierarchy. SIM-REL-051 links refreshed releases through protected anchors. SIM-REL-052 measures downstream decision value, cost and safety.

All four depend on a deeper assumption: the system being evaluated cannot gain an advantage by exploiting the measurement process itself.

That assumption is weak for research agents with web access, code execution, file access, tool orchestration, self-retries and access to public benchmark artifacts.

Recent work demonstrates the issue:
- Search-Time Contamination identifies benchmark metadata, question-context and explicit-answer leakage. In six public benchmarks the study reports performance inflation up to four percentage points, recommending controlled access, isolated sandboxes and transparent retrieval trajectories: https://arxiv.org/abs/2606.05241
- SciConBench uses a clean-room harness to prevent the benchmark's reference conclusions leaking through public search. Its live benchmark contains more than 9,000 conclusion-synthesis samples: https://sciconbench.cs.princeton.edu/
- SCI-AGENT-EXAM evaluates 120 workflow tasks across 11 fields and seven research task types; its reported highest overall score is 80.9, but only 26% of GPT-6 Astra runs exceed its 0.95 success threshold. It also examines trajectory stability: https://arxiv.org/abs/2610.32441
- Autonomous Science Bench separates experiment-discovery and held-out learning gates and reports system cost; with three trials per task, its authors explicitly caution that small leaderboard gaps are not meaningful: https://www.autonomous-science-bench.ai/

A strong scientific benchmark must therefore red-team not only the model but also its own release, oracle, scorer, data boundaries, tool interfaces, identity chain and resource accounting.

## 2. Mandatory threat taxonomy

The initial verifier requires all 12 declared families:

1. Search-time benchmark metadata leakage.
2. Question-context and source-paper leakage.
3. Explicit answer, oracle or hidden-label leakage.
4. Semantic near-duplicate task/recipe/concept contamination.
5. Split-label smuggling or cross-split artifact reuse.
6. Oracle/verifier collusion or shared-authority forgery.
7. Side-channel leakage through errors, timing, filenames, seeds or resource responses.
8. Resource laundering through unmetered tools, parallel work, hidden retries or unreported human effort.
9. Selective reporting, invalid-task deletion, abstention gaming and denominator manipulation.
10. Evaluator contamination, mutable scorer and post-freeze threshold changes.
11. Anchor exposure/reuse and cross-release memorization.
12. Identity/provenance substitution across agent, scaffold, tool, verifier or artifact.

The registry is versioned by a threat-model digest. Adding, removing or changing a family requires a new preregistered plan.

## 3. Typed campaign contract

The plan binds:
- benchmark, version, sealed release ID and digest;
- exact SIM-REL-050 hierarchy ID/digest and preregistration digest;
- threat model, attack catalog, detector artifact, attack harness and independent verifier digests;
- attack operator, detector owner and independent verifier identities;
- all 12 threat-family requirements;
- resource channel IDs and units;
- per-family attack/clean-control/exploit floors and empirical detection, false-positive and containment thresholds.

The source must be a SealedHoldout release. Development/Calibration attack recipes are never eligible as sealed attacks.

Each scenario binds:
- scenario ID and artifact digest;
- threat family and attack/clean-negative-control kind;
- attack generator digest and author identity;
- SealedHoldout split and Development-exposure flag.

Each trial binds:
- exact scenario ID and execution receipt;
- attack operator, detector owner, independent verifier and artifact identities;
- whether execution completed;
- whether the exploit succeeded;
- whether the detector alerted and attributed the correct family;
- whether the attack was contained before exposure;
- whether any leak occurred;
- complete metered resource usage for every preregistered cost channel.

## 4. Do not collapse the attack lifecycle

The receipt distinguishes:

- **Attempted:** one sealed scenario was run.
- **Executed:** the run completed with an execution receipt.
- **Exploit succeeded:** the attack actually bypassed the targeted control.
- **Detected:** the detector raised an alert.
- **Correctly attributed:** the detector identified the expected threat family.
- **Contained:** the attack was blocked before exposure.
- **Leaked:** protected information or an unauthorized state escaped.

A detector alert after a leak is not containment. A harmless attack attempt that never succeeded is not evidence that the defence blocked a real exploit. Every attack scenario remains in the denominator, and every successful exploit remains in the recall/containment denominators.

Clean negative controls are separate scenarios that should not trigger the detector. Any alert on a clean control counts as a false positive.

An incomplete trial remains visible and blocks family sample-floor qualification. It cannot be silently removed or counted as a successful defence.

## 5. Independence and identity boundaries

The plan requires distinct attack-operator, detector-owner and independent-verifier identities. Attack scenario authors and generator artifacts must not overlap the detector owner or independent verifier. Each family requires at least two attack authors and two generator variants. Clean controls have a separately counted author set.

These are structural identity-separation checks over declared identities. They do not authenticate the people, organizations, signing keys or access-control systems behind those identities. A production deployment needs independently audited identity and receipt-signing providers.

## 6. Metrics reported separately

For every threat family:
- attack scenario count;
- clean-control count;
- independent attack-author and generator-variant counts;
- completed/incomplete trial counts;
- successful exploit count and exploit success rate;
- detector alert count on attacks;
- correctly attributed detections among successful exploits;
- successful-exploit detection recall;
- false-positive count and rate on clean controls;
- contained-before-exposure count;
- uncontained exploit count and rate;
- leak count;
- cost total and mean per scenario for each resource channel.

Campaign certification requires all 12 family sample floors, all family thresholds, complete trials, exact one-to-one scenario/trial coverage, no Development-exposed scenarios, separated roles, complete resource accounting and zero leaks. Critical families require zero uncontained exploits.

No one-number average can hide a failed critical family. The family-level vector is the result.

## 7. Pilot targets versus statistical certification

The first implementation reports exact integer counts and empirical rates. Its pass/fail gates are preregistered empirical thresholds; **they are not confidence bounds**. A passing pilot receipt must not be described as a 95% probability guarantee or proof of benchmark security.

A 20-attack / 20-clean-control suite per family is an exploratory pilot floor only. It is useful for discovering obvious bypasses and false alerts, but it is far too small for a strong low-failure-rate claim.

For perspective, under a two-sided 95% Clopper–Pearson binomial interval with zero observed errors:
- 72 successful exploit trials with zero missed detections are needed for a pointwise lower recall bound of at least 95%.
- 368 independent clean controls with zero false alerts are needed for a pointwise upper false-positive bound of at most 1%.
- If all 12 family claims need simultaneous 95% coverage using a simple Bonferroni allocation, the corresponding zero-error floors rise to 121 successful exploits and 615 clean controls **per family**.

These are zero-error sample-size calculations, not claimed observed results. Any missed detection or false alert requires recalculation with the observed counts; larger sample sizes will be needed. The next statistical increment should add reviewed exact Clopper–Pearson intervals and simultaneous family-wise coverage before making probabilistic sensitivity/specificity claims.

## 8. Benchmark targets

Internal targets, not measured results:
- Pilot: at least 20 distinct attack scenarios and 20 clean controls per threat family, with at least two independent attack authors and two generator variants.
- Pilot empirical detection target: at least 95% correct attribution among successful exploits for every family.
- Pilot empirical false-positive target: no more than 1% of clean negative controls alerted.
- Critical family containment: zero successful exploits that leak or escape containment.
- Sealed oracle/answer leakage: zero.
- Metered resources: 100% of declared channels present for every trial.
- Statistical certification: use the stronger 72/368 pointwise or 121/615 simultaneous zero-error floors above, and recompute from actual counts where errors occur.
- Independent replication: a later sealed release, fresh attack payloads and a distinct red team.

## 9. Qualification and known limitations

Focused tests cover:
- all 12 threat families represented;
- attack and clean-control denominators kept separate;
- misses fail recall;
- alerts on benign controls fail specificity;
- detection after leakage fails containment;
- Development-exposed recipes rejected;
- duplicate payload artifacts rejected;
- missing/duplicate trials cannot shrink denominators;
- attack authors and generator variants must meet floors;
- attack operator, detector owner and verifier must remain distinct;
- every trial must meter every preregistered resource channel;
- incomplete trials remain visible and block qualification.

Qualification requires pinned Rust 1.96 formatting, focused tests, full locked crate tests, strict Clippy, exact formatted-head and clean-tree checks.

The first version does not:
- compute Clopper–Pearson confidence intervals;
- authenticate identities or signatures;
- independently execute arbitrary attack code in a sandbox;
- prove that attack generators are independent in the organizational sense;
- measure unknown/unregistered threat families;
- demonstrate absence of unknown attacks or side channels.

The valid claim is: **"This sealed benchmark release passed the preregistered empirical red-team suite under the declared threat model."** Never say "the benchmark is secure" or "unhackable."
