# Tournament Protocol: Decision-Sufficient Audiovisual Observation

**Version:** 0.1  
**Status:** preregistration draft  
**Issue:** #6222  
**Implementation PR:** #6225 (draft)

## Evaluation tracks

Never pool different evidence regimes into one score.

1. **Calibrated probabilistic track:** priors and observation likelihoods come from a frozen, calibrated source model / data cohort. EIG and EVSI may be calculated.
2. **Probability-free track:** only a finite closed hypothesis set and deterministic outcome partitions are admitted. Score worst-case residual set size, diameter, and pair separability. No priors are synthesized.
3. **Open-world/model-adequacy track:** the true event may be absent from the declared hypothesis family. EIG/EVSI are unquantified unless a validated open-world model exists; score detection of model inadequacy, abstention and request for independent evidence.

## Policies to compare

Use the same source streams, initial evidence, candidate request catalogue, action losses, deadlines and budgets.

- P0: current decision / no request
- P1: fixed-rate observation
- P2: seeded random request
- P3: recency-only retrieval
- P4: entropy/surprise trigger
- P5: EIG-per-cost (#7946)
- P6: risk-constrained EVSI (#6045)
- P7: probability-free minimax discrimination (#5774)
- P8: progressive stop/escalate retrieval (StreamScout-like)
- P9: hierarchical event memory (OASIS-like) and hierarchical KV cache (HERMES-like), only where implementations and budgets can be matched
- P10: active hybrid that composes the preceding owners
- Oracle: evaluator-only clairvoyant upper bound; never a competitive result

The best practical modular policy is the primary baseline. An ablation that removes a feature may help attribution, but cannot replace a strong baseline.

## Episode construction

Each episode binds:
- exact scenario ID/version and source/seed ancestry;
- task-request lifecycle events: create, modify, cancel, supersede, expiry and acknowledgement;
- source stream and observation artifact digests;
- world/camera/microphone profile and calibration status;
- clock domains and reset generations;
- hidden event identity, actual outcomes and dependency truth in evaluator-only artifacts;
- agent-visible evidence and request catalogue;
- terminal decisions and finite loss matrix;
- response deadline and request budgets;
- exact policy/model/runtime revisions and resource measurement method.

Train, calibration and confirmatory episodes must be disjoint by scenario family, source episode, seed ancestry, environment, speaker/session, texture/render parent and nuisance-factor family, according to the generalization claim. Never split adjacent frames/windows from one generated episode across partitions.

## Continuous interaction / dead-time extensions

The static fixtures are only the first rung. The next episode family must borrow the important interaction structure from SPOT-Bench and IPIBench:

- **Dead-time:** long stretches where no response is expected; score correct silence and unnecessary resource use.
- **Task modification:** user changes the target, scope or response condition while the stream is active.
- **Cancellation:** a pending observation request is cancelled; a late arrival must not revive the cancelled task or trigger an obsolete response.
- **Reactive/proactive interleaving:** a user asks an immediate question while a standing proactive monitor is active; evaluate floor priority, response delay and task continuity separately.
- **Multi-task contention:** several standing requests compete for sensor, compute and interruption budgets; a high-priority explicit query must not be starved by a low-value proactive request.
- **Duplicate suppression:** one underlying event observed through several derived/correlated paths must not create repeated alerts.

A request is a versioned intent object. Responses and sensor acquisitions bind to the exact request revision that authorized them. Cancellation or modification creates a new revision; it never rewrites the old audit record. A result arriving for a superseded request remains retrievable as historical evidence but cannot automatically satisfy the new request.

## Hard gates before scoring

Any one of these is a benchmark-integrity failure:
- future frame, future audio chunk, reference transcript or hidden truth is visible to the agent;
- request is outside the declared permission, capability, privacy, safety or consent envelope;
- request exceeds a hard latency/energy/bandwidth budget but is still scored as eligible;
- two correlated/derived views are counted as independent without qualified evidence;
- incomplete outcomes or unknown priors are silently imputed to calculate EIG/EVSI;
- a late observation is credited for a decision whose deadline has already passed;
- a score is reported without exact protocol, split, model/profile identity and resource accounting.

## Metrics

### Decision and evidence
- current expected loss and post-observation expected loss;
- EIG (nats), EVSI and net EVSI separately;
- realized decision regret and decision-change rate;
- strict required evidence recall and minimal sufficient evidence rate;
- false support, unsupported claims, evidence-source precision;
- probability-free worst-case residual hypothesis count/diameter;
- open-world/OOD detection, abstention and model-adequacy error.

### Interaction timing
- response-window compliance;
- premature intervention / false alarm;
- missed intervention / false negative;
- duplicate-response rate;
- correct silence during no-trigger/dead-time intervals;
- request-to-arrival and first-sufficient-evidence-to-decision latency.

### Cost and resource
- request count, bytes, audio duration, frame count, tokens;
- p50/p95/p99 wall latency, queue age, throughput, missed deadlines;
- CPU/GPU utilization, peak RAM/VRAM;
- energy per stream-minute only where instrumented with a calibrated meter; otherwise label synthetic/unmeasured.

### Epistemic calibration
- Brier/NLL/ECE only for calibrated probability outputs;
- risk-coverage and selective risk;
- interval coverage for event timing/localization;
- stale, contradictory, superseded and correlated evidence use rates.

No weighted aggregate can compensate for a hard-gate failure. Report a score vector and Pareto frontier.

## Statistical comparison

- Pair candidate and baseline runs on identical episode seeds.
- Unit of analysis is the independent episode / source session, not each 1 Hz tick or every frame.
- Pre-register primary strata, primary metrics, minimum practical effect and non-inferiority margins.
- Use paired confidence intervals and multiple-comparison correction for confirmatory task families.
- Report all failures, unsupported cases, timeouts and abstentions; do not convert missing runs into zero successes or silently drop them.
- A win must be reproducible on a source-family-disjoint holdout and not trade away privacy, reliability or silence behavior.

## Promotion sequence

G0: fixture JSON parses and episode IDs align.  
G1: finite known-answer EIG/EVSI calculations match independent arithmetic oracle.  
G2: deterministic negative controls pass.  
G3: hidden truth process boundary demonstrated.  
G4: matched baseline tournament on frozen synthetic episodes.  
G5: source-family-disjoint holdout and external benchmark reproduction.  
G6: physical microphone/camera qualification with calibrated device clocks, acquisition losses, geometry and resource meters.

Passing G0–G2 does not qualify a model or renderer. Passing synthetic G3–G4 does not establish physical sensor accuracy.
