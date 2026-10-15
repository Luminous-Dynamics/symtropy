# Maritime Resilience Benchmark Ladder v0.1

**Purpose:** define what “better” means before optimizing or claiming that the system is resilient.  
**Status:** proposed project targets, not an established maritime standard and not measured results.  
**Applies to:** the adaptive-resilience contract, the deterministic trajectory reducer and future admitted simulation models.

## Principle: separate software correctness from model truth

A program can calculate a toy service-loss curve perfectly and still be a poor model of a real port. Benchmarks must therefore be reported in separate tiers. A pass at one tier does not imply a pass at the next.

This is consistent with UNCTAD's resilience dimensions—robustness, redundancy, visibility, flexibility, collaboration, agility, information sharing, response and recovery—and its guidance that resilience includes preserving acceptable service through disruption, recovering, and adapting. High-consequence model credibility also needs verification, validation and uncertainty quantification proportionate to the decision consequence, rather than a single accuracy score. Sources: [UNCTAD resilience-related metrics](https://resilientmaritimelogistics.unctad.org/guidebook/510-resilience-related-metrics), [UNCTAD definition of port resilience](https://resilientmaritimelogistics.unctad.org/guidebook/21-defining-port-resilience), [NIST AMS 100-85, published 5 October 2026](https://doi.org/10.6028/NIST.AMS.100-85).

## Tier 0 — contract, arithmetic and adversarial correctness

These are hard release gates for the pure trajectory reducer.

| Benchmark | Target | Why it matters |
|---|---:|---|
| Independent-oracle parity on frozen integer fixture metrics | 100% exact match | Prevent arithmetic drift and off-by-one recovery errors |
| Required authority-escalation negative controls rejected | 100% | A malformed report must never imply operational authority |
| Unknown/stale/conflicting/missing evidence controls preserve uncertainty | 100% | No unknown-to-zero, unknown-to-available or unknown-to-feasible substitution |
| Hard-constraint and non-compensability mutants rejected | 100% | No score can offset a hard violation |
| Deterministic replay on identical pinned inputs/configuration | Byte-identical canonical report | Reproducibility and exact audit comparison |
| Duplicate JSON keys, unknown fields/states, zero demand, invalid intervals and integer overflow | All frozen negative controls rejected | Eliminate ambiguous or undefined contract inputs |
| Recovery boundary semantics | Zero off-by-one mismatches in boundary corpus | Recovery timestamp must use the declared end-of-interval convention |

The current increment implements a first slice of this tier: it validates the synthetic contract, recomputes four trajectory examples, binds the raw fixture bytes with SHA-256 and includes an independent test oracle and negative controls. It does **not** yet cover the full table. The rest must be implemented and shown passing by exact-head tests before calling Tier 0 complete.

**No false greens:** define the suite so any case labelled feasible when a required hard constraint is violated is a test failure. Report false-green count alongside total cases; never hide it inside average accuracy.

## Tier 1 — computational scale and reproducibility

Benchmark the reducer separately from the full scenario engine. Pin the CPU/runner class, operating system image, Rust toolchain, optimization profile, input generator seed and benchmark binary digest. Collect at least 30 warm runs per point and publish p50/p95 wall time, peak RSS, input bytes, intervals/sec and output bytes. CI timing is noisy; establish the reference environment before converting latency targets into release gates.

| Dimension | Benchmark points | Target hypothesis to test |
|---|---|---|
| Service trajectory length | 1k, 10k, 100k, 1M interval points | Approximately linear work as points increase |
| Dependency graph shape | 100, 1k, 10k nodes; sparse and dense edge sets | Graph validation scales with nodes + edges; no hidden all-pairs expansion in the contract reducer |
| Trace multiplicity | 1, 10, 100 service classes across each interval scale | Clearly document whether work scales as trace count × interval count |
| Serialization/replay | 1k, 100k, 1M result records | Exact digest comparison remains deterministic |
| Memory | Same scales with fixed input dimensions | No unbounded retention of duplicate full trajectories |

Candidate engineering targets **to test, not achieved results**: on a pinned reference runner, p95 ≤2 seconds for 100k simple interval points and ≤15 seconds for 1M points; peak RSS ≤512 MiB at 1M points; for a 10× workload increase, runtime ≤15× and peak memory ≤12×. First collect a baseline and verify test fixtures actually represent those sizes. If the initial model requires more resources, record the measured scaling and bottleneck rather than weakening correctness checks or claiming performance not measured.

Keep benchmark runs distinct from PR correctness jobs if shared GitHub runners are congested. A queued job is neither a pass nor a performance datapoint.

## Tier 2 — uncertainty, model credibility and historical validation

No universal predictive-accuracy target is defensible until a target port/service owner defines the tolerated error and the reference observations are qualified. Use these as staged evidence requirements.

### Dataset and evaluation design

- Pilot corpus: at least 10 independently defined disruption episodes spanning at least 3 disruption/dependency classes and, where legitimate data access permits, more than one port.
- Before making broader predictive claims: aim for at least 30 episodes, with a temporally held-out test set and a leave-one-port-out evaluation when the data supports it.
- Keep all observations from one incident/time window in one split. Avoid leakage from post-incident data, future schedule revisions or revised reports into the model inputs for an earlier decision time.
- Record unavailable observations, data revisions, changed operational definitions and censored recovery times. Do not treat a port reporting outage as zero physical throughput.
- Compare to at least two simple baselines: persistence/no-new-information and a transparent static capacity-loss/recovery baseline. A complex model must earn its complexity.

### Metrics to publish per service class and event family

1. Normalized MAE for interval service rates and integrated unmet service, with the denominator and zero-baseline handling declared.
2. Absolute error in time-to-recovery for observed recoveries; score right-censored / not-recovered-within-horizon episodes separately instead of inventing recovery times.
3. Peak-loss magnitude and onset/duration error.
4. Directional correctness of candidate-plan ranking, only where an independently adjudicated comparison exists and all hard constraints remain separate.
5. Performance by hazard family, port/system type, data-quality state and forecast lead—not only pooled aggregate.
6. Calibration and coverage of uncertainty intervals, if the model makes probabilistic claims. Report nominal versus observed coverage and proper scoring rules; do not report a confidence label without the sample size and uncertainty behind it.

### Candidate predictive goals

These are research targets for discussion with domain stakeholders—not industry standards:
- On a qualified held-out corpus, target at least a 15% relative reduction in normalized service-trajectory MAE against the strongest simple baseline, without worsening any hard-constraint result.
- For episodes with an observable recovery, target median absolute TTR error no worse than one simulation interval and relative error ≤20%; report both because either alone can obscure long/short events.
- Once there is enough independent held-out evidence to evaluate calibration meaningfully, for nominal 80% predictive intervals aim for empirical coverage of 75–85%, including stratified results and a coverage uncertainty interval.
- If a target is missed, preserve and publish the failure analysis and scope restriction. Do not tune on the holdout and continue calling it a holdout.

These thresholds should be changed only through an explicit, versioned acceptance decision based on operational tolerance and data quality, not to make a result look green.

## Tier 3 — policy and decision usefulness

Evaluate adaptive policies against a static-plan baseline on the exact same versioned case family.

| Benchmark | Target |
|---|---|
| Required hard constraints satisfied in a claimed robustly-feasible set | 100%, with each case/constraint traceable |
| Adding an adverse required scenario | Must never expand the robustly-feasible set |
| Removing, staling or conflicting required evidence | Must never improve a dependent disposition |
| Unconfirmed alternate capacity | Never becomes a confirmed allocation, booking or reservation |
| Evidence request utility | Every gap maps to the exact result/policy branch it could change, required fields and resolution scope |
| Policy execution | 100% bounded simulation-only actions; no external side effects |
| Decision usefulness | Compare decision changes and unresolved dependencies against expert adjudication; do not use a single score to cover disagreement |

Initially report **decision relevance**, not numeric expected value of information. A value-of-information calculation is only admissible when the probability model, consequences, costs, timing and dependencies are justified and validated.

## Tier 4 — integration, interoperability and auditability

Every scored run should be replayable from pinned scenario, model/capability profile, policy, evidence revisions, exact input bytes, time semantics, configuration, event schedule and canonical output. Each external adapter should record issuer identity, product/dataset/revision distinction, retrieval time, validity window, terms/use limitations and exact-byte digest. A checksum establishes byte identity; it does not establish issuer authenticity, scientific validity or licensing.

Map each interface against the applicable maritime data standards and versions, but never assume that a standard's publication implies live data access or correctness. The IHO S-100 ecosystem began entering operational use with Phase 1 product specifications in January 2026; specific product version, distribution, permissions and support still need to be checked for each deployment. See [IHO's S-100 framework update](https://iho.int/en/the-s-100-framework-is-now-operational).

Before any operational pilot, require an independent reviewer and domain owner to sign off on: model scope, input quality, uncertainty treatment, validation evidence, failure modes, usage restrictions and the explicit boundary between decision support and operational authority. A simulation should remain read-only unless a separately assessed and authorized control system is designed.

## Recommended sequence

1. Finish Tier 0 arithmetic/schema/authority negative controls and exact-head verification.
2. Establish reproducible Tier 1 baseline measurements; optimize only the measured hot path.
3. Inspect the actual deterministic engine capability profile before implementing propagation or physical calculations.
4. Capture and quality-check a small historical reference corpus before making accuracy claims.
5. Compare with simple baselines on temporally held-out incidents; publish all failures and censored cases.
6. Admit a limited, explicitly versioned model only after independent validation and uncertainty review.
7. Integrate read-only outputs across Symtropy, Symthaea, Mycelix and Sol Atlas with source lineage and authority separation.

**Completion rule:** a benchmark is “passed” only with a published command, corpus/version/digest, reference environment, raw result, verifier version and exact commit SHA. Targets in this document are proposals, not measurements. Neither a green software test nor a stable digest alone establishes physical truth, safety, or real-port performance.
