# ROAD-HYPOTHESIS-PORTFOLIO-001 v0.1

A small Rust reference for qualifying a finite causal/world-model portfolio, its challenge process and residual decision-risk envelope.

## The boundary

This is deliberately separate from:

- trajectory support (SAM #10514): is this complete joint history represented by the admitted trajectory corpus?
- event correspondence and stochastic coupling (#3743 and preceding contracts): what is the event-address and cross-world randomness relation?
- reciprocal episode export (#3792): how is a specific ego/actor episode assembled and handed to SAM?
- evidence eligibility (#3635): what kind of empirical or model-generated evidence can support which claim?

This crate asks a different question: have the declared model/mechanism portfolio's coverage obligations been accounted for, have competing models been subjected to an equal and blinded falsification process, and is residual unknown-mechanism risk bounded by independent evidence?

A path can be supported under every enumerated hypothesis while the portfolio is incomplete or its residual risk is unbounded.

## Paradigm shift

Replace “enumerate several plausible models, then optimize over them” with:

1. declare scope, mechanism classes, exclusions, lineage and the four coverage axes;
2. enumerate explicit obligations including critical attribute combinations and interaction regimes;
3. challenge every hypothesis using random, coverage-guided, expert and RL-based search on one sealed holdout and equal budgets;
4. preserve counterexamples, verify replay and retain the discovery curve;
5. count common-mode and partially shared ancestry conservatively rather than treating cloned models as independent;
6. calculate residual utility intervals only from independently sourced, calibrated evidence at the declared independent-unit level;
7. authorize a recommendation only for a complete declared-scope portfolio with at least one robust-feasible action and a bounded residual envelope.

“Complete for declared scope” never means complete relative to reality.

## Executable contract

The Rust reference provides:

- Four separate axes: ODD ranges/attribute combinations, behavioural competencies, causal mechanisms, and rule/safety obligations.
- Four states: Covered, NotCovered, Unknown and NotApplicable. Unknown never becomes Covered. NotApplicable requires a specific scope rationale.
- Per-hypothesis mechanism class IDs, causal graph and stochastic contract digests, full source lineage, common-mode group, applicability scope and explicit exclusion reasons.
- Effective independent-family counting via connected components over shared source ancestry and common-mode groups. Partial overlap is treated conservatively and transitively.
- A fixed comparison matrix of random, coverage-guided, expert and RL falsification for every hypothesis.
- One common holdout manifest and parent-lineage set, equal actual search budgets, blinded/sealed status and minimum budget obligations.
- Counterexample findings require a replay receipt and an explicit verified-replay flag. Non-discovery is budgeted non-discovery, never proof of absence.
- Exact rational residual utility bounds with per-action evidence, calibration protocol, source lineage and minimum independent-unit counts. The residual scope, estimand and independent-unit definition are explicit. Counterfactual residual utility bounds cannot be justified by observational factual records alone: they require a controlled-intervention assignment protocol or a qualified natural-experiment identification package, and the evidence must not reuse training or sealed challenge lineage.
- Robust feasibility distinct from utility ranking. Unknown or PotentialViolation excludes an action.
- Conservative rectangular regret upper bound: `max(0, max_b U_upper(b) - L(a))`. It is explicitly a rectangular relaxation, not an attainable joint physical world.
- Append-only portfolio revisions: prior challenged hypotheses and coverage records remain byte-identical. New hypotheses use new identities; new coverage evidence appends a successor record bound to the exact prior axis/stratum. Superseded historical rows remain in the digest but no longer count as active obligations. Forked or cross-stratum supersession is rejected.
- A portfolio digest and evaluation digest that bind the manifest, exact challenge receipts, residual bounds and decision disposition.

## Exact conformance targets

- 100% correct state handling on declared coverage fixtures.
- Zero unknown/out-of-scope cells silently counted as covered.
- Zero cloned or partially shared-lineage variants counted as independent corroboration.
- 100% of declared hypotheses have the four equal-budget challenge arms.
- 100% of search arms share one sealed holdout and lineage set.
- 100% replay of claimed counterexamples; zero unreplayed counterexamples accepted as falsification receipts.
- 100% of residual actions carry calibrated evidence references and meet their independent-unit floor.
- Zero observational factual-only residual bounds used as truth for unexecuted alternate actions.
- 100% of natural-experiment residual bounds carry a declared identification-assumptions package; controlled-intervention bounds carry an assignment-protocol reference.
- Zero residual interval inferred from absence of observed failures.
- Zero robust action when residual safety feasibility is unknown or potentially violated.
- 100% evaluation-digest change on any challenge or residual evidence mutation.
- 100% of coverage status changes preserve the prior record and append a single non-forking, same-stratum successor.
- Never emit a completeness-relative-to-reality claim.

These are contract/conformance targets, not measured empirical results. The assignment/identification and calibration digests are declarations bound into the evaluation receipt; this Rust reference does not independently authenticate the external study or re-estimate its causal effects.

## Comparative research baselines

Do not claim this crate outperforms current driving benchmarks. Its role is complementary:

- **Waymo Open Dataset (WOD)** remains a major public benchmark for motion prediction, interaction prediction, sim agents, scenario generation and end-to-end driving. Its 2025 Sim Agents challenge reports separate realism, kinematics, interactivity, map adherence and minADE. A third-party leaderboard mirror lists SMART-R1 at 78.58 realism, 81.10 interactivity, 92.01 map adherence and 1.2885 minADE. Waymo states it is not hosting formal challenges in 2026 while leaving the leaderboards active. This crate adds model lineage, residual-risk and decision-eligibility dimensions; it does not replace WOD task metrics.
- **nuPlan-R (2026)** adds learning-based diffusion reactive agents and interaction-aware selection to the nuPlan closed loop, directly addressing limitations in IDM-style reactive traffic. It is the closest public baseline for realistic reciprocal interaction. The novelty here must be the evidence-qualified portfolio/falsification/unknown-mechanism bridge, not “we have reactive agents.”
- **CARLA Leaderboard 2.0 / Bench2Drive** provide closed-loop driving score, route completion and infraction metrics. Use the exact official split and full infraction breakdown; aggregate driving score alone is not a superiority claim.
- **ASAM interoperability** should target OpenDRIVE 1.9.0, OpenSCENARIO XML 1.4.0 and OpenSCENARIO DSL 2.2.0 as of this research snapshot. XML 1.x and DSL 2.x are parallel tracks, not interchangeable syntax.
- **ISO 34502:2022** is scoped to limited-access highways and certain ADS categories. It does not cover cybersecurity, HMI/misuse, comfort, energy or traffic-flow efficiency. Johannesburg/urban ODD claims therefore need explicit extra coverage.

## What would beat SOTA meaningfully

Do not optimize only for minADE, realism score, route completion or simulated crash count. Require a Pareto comparison across:

1. action-conditioned behavioral fidelity and reciprocal response with independent controlled-intervention anchors;
2. tail-event calibration, rare-event discovery yield and counterexample diversity at equal search budgets;
3. common-mode model ancestry and effective independent family count;
4. residual utility-envelope width, robust feasibility and worst-case regret;
5. decision stability under new mechanisms, held-out geography/time/ODD and alternate dependence/correspondence hypotheses;
6. reproducibility, exact replay, evidence lineage and cost per independently replayed counterexample.

A higher average score is not better if achieved by omitting hard cases, reusing simulator-generated labels, or counting correlated models as independent.

## Roadmap

- **V0.1 (this crate):** exact finite portfolio coverage, common-mode ancestry, equal-budget blinded challenges, replay-bound falsification, independent residual envelope and rational regret bounds.
- **V0.2:** connect the actual #3532 reciprocal producer; consume #3792 full episode artifacts; export one query per branch/hypothesis into SAM; attach each SAM disposition to the portfolio coverage ledger.
- **V0.3:** implement scenario/mechanism discovery with frozen training vs sealed holdout, minimized replayable counterexamples, novelty/lineage checks, and discovery curves versus equal-budget random, coverage-guided, expert and RL methods.
- **V0.4:** calibrate residual bounds on independent controlled-course interventions, hold out geography/time/site/actors, and add a constrained joint residual model to tighten the rectangular relaxation.
- **V0.5:** cross-simulator differential evaluation using ASAM road/scenario inputs and public adapters for WOD/nuPlan-R/CARLA; publish claim ceilings and negative results.

## Run

```sh
cargo fmt --manifest-path research/road-hypothesis-portfolio-001/Cargo.toml -- --check
cargo test --locked --manifest-path research/road-hypothesis-portfolio-001/Cargo.toml --all-targets
cargo clippy --locked --manifest-path research/road-hypothesis-portfolio-001/Cargo.toml --all-targets -- -D warnings
```

The JSON schemas describe the portable manifest and evaluation receipt field contracts. This v0.1 crate is a typed Rust reference; it does not yet include an authoritative JSON parser or signed evidence registry.
