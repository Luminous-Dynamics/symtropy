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

## Verified industry and research snapshot (checked 2026-11-06)

The field is moving on three fronts at once: more realistic closed-loop agents, reasoning-capable foundation policies, and increasingly explicit ODD/scenario coverage. This portfolio verifier is not a substitute for any of them.

- **Waymo Open Dataset / WOSAC:** the official Motion Dataset currently describes 103,354 twenty-second, 10 Hz segments and 574 hours, mined for interaction-rich cases including unprotected turns, merges, lane changes and intersections. The official 2025 challenge set included scenario generation, Sim Agents and interaction prediction; Waymo says its 2026 formal challenges are not running, while the leaderboards remain active. Benchmark motion and interaction fidelity with WOD's own metrics; report independent scenario/drive counts, not frame count. [Dataset scale and limitations](https://waymo.com/open/about/) · [2026 challenge status](https://waymo.com/open/challenges/) · [WOSAC paper](https://waymo.com/research/the-waymo-open-sim-agents-challenge/).
- **nuPlan-R (IEEE IV 2026):** this is a particularly important closed-loop baseline. It replaces IDM-only traffic with noise-decoupled diffusion reactive agents, adds interaction-aware agent selection and two planning metrics. It directly addresses unrealistic one-sided/rule-based surrounding-agent response. Our comparative test must include nuPlan-R actors, not claim novelty merely because our simulator is reactive. [IEEE IV 2026 paper and abstract](https://doi.org/10.1109/IV66570.2026.11623929) · [arXiv preprint](https://arxiv.org/abs/2511.10403).
- **NVIDIA Alpamayo 2 Super (August 2026):** an open 34B VLA family combines a 32B reasoner and 2B diffusion action expert, with trajectory, causal-reasoning, meta-action and grounded scene outputs. NVIDIA reports both open-loop metrics and closed-loop AlpaSim results on reconstructed scenes. It is a serious reasoning-policy baseline, but the reported results are vendor-authored and not directly comparable to our finite portfolio verifier. Use it as a policy/world-model candidate, then independently test lineage, calibration, interaction and intervention response. [NVIDIA's report](https://developer.nvidia.com/blog/generate-trajectories-reasoning-traces-and-auto-labels-with-nvidia-alpamayo-2-super).
- **Falsification:** AAMAS 2026 includes an RL-based dynamic-driving falsification method comparing counterexample efficiency, diversity and quality. Our four-arm challenge matrix is designed to compare RL with random, coverage-guided and expert search on the same sealed holdout and equal budgets. [AAMAS 2026 publication](https://aiea-lab.github.io/publication/reinforcement-learning-for-falsification-of-dynamic-driving-scenarios-2026/).
- **ODD/behaviour coverage:** a 2026 IEEE Access framework explicitly evaluates attribute-range coverage, ODD and behavioural-competency coverage, out-of-ODD cases and road-rule compliance. The four-axis ledger here deliberately aligns with that direction while adding causal mechanism obligations, model ancestry, blinded falsification and residual decision-risk. [IEEE Access 2026 paper, DOI 10.1109/ACCESS.2026.3665396](https://ieeexplore.ieee.org/document/11407490/).
- **CARLA Leaderboard 2.0 / Bench2Drive:** report driving score, route completion and the infraction penalty components separately. A high aggregate score can conceal unacceptable collision, red-light or other infraction profiles. [Official CARLA evaluation criteria](https://leaderboard.carla.org/evaluation_v2_0/).
- **Standards and scope:** ASAM's current versions on 2026-11-06 are OpenDRIVE 1.9.0 and OpenSCENARIO XML 1.4.0 (both released 19 May 2026), plus OpenSCENARIO DSL 2.2.0 (19 March 2026). XML 1.x and DSL 2.x are separate syntax tracks. ISO 34502:2022 is explicitly limited-access-highway scoped and excludes, among other things, misuse/HMI/cybersecurity and comfort/energy/traffic-flow topics; it is not a universal ODD assurance standard. [OpenDRIVE](https://www.asam.net/standards/detail/opendrive/) · [OpenSCENARIO XML](https://www.asam.net/standards/detail/openscenario-xml/) · [OpenSCENARIO DSL](https://www.asam.net/standards/detail/openscenario-dsl/) · [ISO 34502:2022](https://www.iso.org/standard/78951.html).

## Prospective targets to beat strong baselines

These are goals to pre-register, not achieved results.

| Layer | Baseline / measurement | Target before a superiority claim |
|---|---|---|
| Finite verifier | The 36-case hostile conformance matrix, plus property-based permutations | 100% expected dispositions; zero false authorization under missing/unknown/out-of-scope/unbounded/lineage-leak controls |
| Falsification | Random vs coverage-guided vs expert vs RL; same sealed scenario families, parent lineage, hardware and compute budget | At least 10% more *unique, independently replayed mechanism-level counterexamples per compute-hour* than the strongest non-RL arm, with paired 95% confidence interval above zero and no increase in false discoveries |
| Counterexample integrity | Replay in fresh process/runtime, minimized witness, independent verifier | 100% of credited discoveries replay; zero duplicate/renamed witnesses counted as new mechanisms |
| Coverage | Explicit ODD × condition × behaviour × causal mechanism × rule obligations | 100% declared obligations accounted; zero Unknown/out-of-ODD rows counted as Covered; report active and superseded rows separately |
| Model independence | Shared source, simulator, pretraining and policy ancestry graph | Zero shared-ancestry variants counted as independent corroboration; publish the effective-family count and the graph |
| Residual risk | Independent controlled interventions or qualified natural experiments, action-specific calibration and independent-unit floors | No residual interval without an evidence/identification package; no robust action if feasibility is Unknown or PotentialViolation |
| Decision quality | Exact known-model regret plus residual upper bound; sensitivity across correspondence and dependence hypotheses | Zero invented hypothesis probabilities; report no-decision when the minimax set is non-unique or required support is incomplete |
| External driving baselines | WOD/WOSAC, nuPlan-R, CARLA Leaderboard 2.0 on their official splits | No aggregate-only claim: meet or exceed relevant task metrics while preserving interaction, kinematic, map, infraction, calibration and subgroup breakdowns |
| Generalization | Sealed geography, time, site, weather, road-user and ODD shifts | Report per-stratum confidence intervals and transport degradation; no pooling away a failing stratum |
| Reproducibility/cost | Exact input digests, replay success, p50/p95 runtime, memory and cost per unique replay | 100% replay of credited evidence; publish quality/cost Pareto curves, not only best score |

The 10% discovery target is a deliberately modest *research gate*, not a universal SOTA threshold. It only counts if arms receive equal search budgets and a common sealed holdout, unique counterexamples are grouped by mechanism rather than file/seed, and the paired confidence interval accounts for independent scenario families. If the target fails, publish that result and investigate why.

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
