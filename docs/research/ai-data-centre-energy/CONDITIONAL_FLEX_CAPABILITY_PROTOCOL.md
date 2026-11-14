# Conditional Flexibility Capability Protocol

**Protocol ID:** GRID-FLEX-CALIBRATION-001  
**Version:** 0.1  
**Status:** research protocol draft  
**Repository issue:** #3730  
**Claim ceiling:** statistically calibrated capability for a pinned site, PCC, product, operating domain and evidence period; not ELCC, market accreditation or a universal capacity rating.

## 1. Purpose

This protocol defines how Symtropy should convert repeated operational observations into a conservative, condition-specific flexibility capability envelope.

A dispatch event is not an independent coin flip merely because it has a unique identifier. Events may share weather, equipment, workload, grid-stress, controller, communications and recovery conditions. Treating correlated events as IID can make uncertainty look materially smaller than it is.

The protocol therefore separates:

1. physical and service feasibility of each full trajectory;
2. empirical response performance over observed opportunities;
3. uncertainty under temporal, spatial and cross-component dependence;
4. applicability of the evidence to a proposed operating condition;
5. system adequacy value, which remains a separate downstream calculation.

The output is a capability surface, not a single flexibility percentage.

## 2. Normative terminology

- **Opportunity:** one scheduled or contractually expected dispatch request, including requests that are unavailable, rejected, missed or not delivered.
- **Episode:** a group of opportunities sharing a common stress or causal state, such as one heatwave, grid-scarcity episode, controller outage or contiguous dispatch/recovery sequence.
- **Operating stratum:** a version-pinned domain of conditions within which evidence is intended to apply.
- **Full-trajectory success:** every hard service, physical, response, measurement and recovery gate passes from pre-event baseline through terminal-state reconciliation.
- **Capability envelope:** the set of product quantities and response/recovery trajectories that satisfy declared hard constraints and a declared conservative delivery-confidence requirement.
- **Out-of-domain (OOD):** a proposed condition outside the support or validated bounds of the calibration evidence.
- **Evidence expiry:** the time or condition change after which a capability receipt is no longer eligible for offer or adequacy use without requalification.

## 3. System boundary and evidence chain

The declared measurement boundary is the point of common coupling (PCC) unless a different boundary is explicitly required by the product contract. IT, server, rack, UPS, battery and cooling meters are nested observations; they must not be summed as independent reductions when the PCC already measures their net effect.

The evidence chain is:

1. workload verifier: same useful-work obligations and service-level objectives;
2. physical-state verifier: feasible battery, thermal, cooling and site-power trajectories;
3. response verifier: command-to-response latency, ramp, sustained power and PCC delivery;
4. recovery verifier: rebound, recharge, thermal recovery and terminal-state closure;
5. calibration layer: conditional delivery probability and uncertainty over episodes;
6. owner policy: whether the site is authorized to offer the capability;
7. adequacy engine: whether the qualified trajectory changes system reliability under a frozen grid model.

A pass at one layer does not imply a pass at the next. Calibration must not repair or override a failed physical or service gate.

## 4. Conditional capability definition

Let:

- (x) be the operating-condition vector;
- (a) be a candidate product action/envelope;
- (Y(a)in\{0,1\}) indicate whether the complete trajectory passes every hard gate;
- (p_\star) be the owner/operator-required delivery probability;
- (\alpha) be the confidence level used for the lower confidence bound.

The candidate action is statistically eligible only when:

\[
\operatorname{LCB}_{1-\alpha}\left[
  \Pr\{Y(a)=1\mid X\in\mathcal{S}(x)\}
\right] \ge p_\star
\]

where \(\mathcal{S}(x)\) is an explicitly defined operating stratum or validated conditional neighborhood.

The eligible capability set is:

\[
\mathcal{C}(x)=
\left\{a:
\text{all deterministic constraints pass and the conditional delivery gate passes}
\right\}.
\]

Do not assume the capability set is a scalar or that its dimensions are independent. The envelope may jointly constrain upward/downward kW, latency, ramp, sustain duration, energy, rebound, recovery time and service quality. Its geometry must be preserved in the receipt.

This probability is conditional on the pinned data, stratum, site configuration and method. It is not a probability of system adequacy and must not be labelled ELCC.

## 5. Full-trajectory outcome

An opportunity is successful only if all contractually applicable conditions hold:

- request authenticity, ordering, expiry and local owner authorization;
- valid and sufficiently fresh telemetry with adequate clock quality;
- response latency and ramp limits;
- minimum and maximum PCC response envelope;
- sustained delivery for the full required duration;
- no workload loss, deadline violation or hard service-level objective breach;
- no battery reserve, charge/discharge, thermal, humidity, cooling or facility-limit violation;
- complete accounting of gross reduction and negative reduction/rebound;
- recovery completed within the declared window;
- required terminal battery and thermal states reconciled;
- no unexplained missing samples, substituted opportunity IDs or post-hoc exclusion.

Unavailable requests, controller faults, communications failures, under-delivery, overshoot, late response, hard service failures and insufficient evidence remain failed or indeterminate outcomes. They may not be silently dropped from the denominator.

## 6. Operating-condition vector

At minimum, the pinned condition vector should include:

| Group | Required fields |
|---|---|
| Identity | site, PCC, meter-chain revision, product, controller, scheduler, firmware and configuration revisions |
| Weather | ambient temperature, wet-bulb temperature, humidity and weather episode identity |
| Grid | system-stress state, local congestion/constraint state, voltage/frequency state where relevant |
| Workload | workload class mix, arrival intensity, queue/backlog, accelerator SKU, request mix and SLO class |
| Electrical | initial SoC, reserve floor, battery health/degradation state, UPS mode and available on-site generation |
| Thermal | zone/loop state, cooling mode, thermal headroom, water constraints and equipment availability |
| Controls | telemetry quality, clock uncertainty, communications path, controller availability and local policy version |
| History | recent dispatch count, inter-event gap, recovery completion, accumulated thermal/storage debt and maintenance state |

The vector must preserve units, sample times, missingness and evidence provenance. Unknown fields are not equivalent to nominal conditions.

## 7. Episode identity and data partitioning

Every opportunity must carry both an opportunity ID and one or more causal episode IDs. At minimum, distinguish:

- weather episode;
- grid-stress episode;
- site equipment/controller episode;
- contiguous dispatch/recovery episode;
- cross-site common-cause episode where relevant.

Train, validation and test partitions must be made at the episode level. Individual time steps or events from the same heatwave, outage, controller incident or recovery sequence must not be split across partitions where that would leak shared information.

Required evaluation splits:

1. **Chronological forward holdout:** train on earlier episodes; test on later episodes.
2. **Leave-one-stress-episode-out:** test generalization to a held-out heatwave or scarcity episode.
3. **Leave-one-site/configuration-out:** only where cross-site transfer is being claimed.
4. **Regime-shift holdout:** test an intentionally changed workload, controller, thermal or grid regime.
5. **OOD challenge:** include conditions outside the calibrated support and verify abstention or downgrade.

Random-row cross-validation alone is not acceptable evidence of generalization.

## 8. Dependence-aware uncertainty methods

Implement a comparison harness; do not select a method by its most favorable confidence interval.

| Method | Permitted role | Key limitation to test |
|---|---|---|
| Exact binomial / Hoeffding | IID reference case and simple baseline | Invalid as a general guarantee when trials are dependent or non-stationary |
| Cluster bootstrap | Resample independent episodes or clusters | Requires enough representative clusters; few clusters make tail inference weak |
| Moving/stationary block bootstrap | Preserve temporal dependence within a stationary sequence | Sensitive to block construction and stationarity assumptions |
| Hierarchical beta-binomial / regression | Partial pooling across sites or strata | Pooling may hide poor conditional coverage; model misspecification matters |
| State-space / survival / fatigue model | Time-varying availability and recovery dynamics | Requires identifiability, calibration and held-out regime tests |
| Importance sampling / rare-event simulation | Rare stress exploration and tail estimation | Must preserve likelihood weights and be checked against known-truth cases |

Block length and cluster definitions must be preregistered or selected using a declared rule, followed by sensitivity analysis. Report uncertainty over the episode population, not only over the number of raw samples.

A method must not claim 99% or 99.9% delivery reliability from a small number of correlated stress episodes. Where the evidence does not support the requested confidence, the result is **insufficient evidence**, not extrapolated certainty.

## 9. Calibration and coverage metrics

Report, at minimum:

- full-trajectory empirical success rate and lower confidence bound;
- episode count, raw opportunity count and effective independent episode count;
- conditional coverage by each declared stress stratum;
- false-qualification rate on adversarial known-truth fixtures;
- abstention rate and OOD detection recall;
- envelope sharpness (delivered kW/duration/recovery limits) subject to achieved coverage;
- latency, ramp, sustain, rebound and recovery distributions;
- service completion, deadline/SLO and physical-state gate outcomes;
- uncertainty interval method, block/cluster definition and sensitivity;
- chronological holdout performance and drift/expiry state.

Aggregate calibration error or an overall reliability percentage is not sufficient: it can conceal under-coverage in exactly the hot-weather or grid-scarcity strata that drive capacity value.

## 10. Synthetic known-truth corpus

The benchmark corpus must include reproducible generators with explicit seeds and ground-truth parameters:

1. IID Bernoulli opportunities where binomial/Hoeffding methods are valid.
2. Autocorrelated success/failure blocks where IID intervals are overconfident.
3. Heatwave episodes jointly affecting cooling derate, workload demand, thermal headroom and battery availability.
4. Common-mode controller or communications failures across multiple sites.
5. Consecutive calls with state-dependent recovery, fatigue and unavailable opportunities.
6. Abrupt and gradual changes in response performance.
7. Rare high-severity under-delivery and rebound.
8. Missing-not-at-random telemetry, clock drift and measurement dropout.
9. Site/configuration heterogeneity where partial pooling may or may not be valid.
10. OOD conditions that must cause a capability downgrade or abstention.

Each fixture should declare true conditional success probability, dependence structure, episode membership, true capability boundary and known coverage target. Include a negative control in which IID analysis yields a falsely narrow interval while cluster-aware analysis remains appropriately uncertain.

## 11. Reproducibility receipt

A capability receipt must bind:

- schema and protocol version;
- site/PCC/product and meter-boundary identity;
- source dataset IDs, permissions, provenance and content digests;
- measurement quality, timebase and units;
- operating-stratum definition and episode assignment;
- train/validation/test partition manifest;
- method, parameters, seeds, block/cluster rules and code revision;
- opportunity denominators, including rejected/unavailable/failed trials;
- capability envelope and all uncertainty/coverage diagnostics;
- OOD/drift status, issue time and expiry conditions;
- upstream workload, physical-state, response and recovery receipts.

A caller-supplied digest is a label until independently recomputed from source bytes. The consumer must verify receipt structure, digest references, schema version and exact contract binding before using the result.

## 12. Acceptance gates

These are proposed engineering goals, not regulatory requirements or achieved results.

- 100% exact agreement on deterministic known-truth fixtures.
- 100% fail-closed behavior for invalid, stale, missing, unsupported or OOD evidence.
- Pre-registered 90% and 95% coverage targets evaluated on chronological episode holdouts, with uncertainty intervals.
- No false qualification in fixtures that deliberately violate an estimator's assumptions.
- No IID confidence claim when the data-generating process contains dependence that invalidates it.
- Zero credited trajectory with a hard workload, physical, response, rebound or recovery failure.
- Exact replay from frozen inputs, method parameters, seeds and digests.
- Explicit insufficiency/abstention when episode counts or regime coverage cannot support the requested confidence.
- No cross-site or cross-regime transfer claim without held-out evidence for that transfer.

## 13. Integration contract

- **#3652:** event-level response evidence; retain its declared statistical assumptions.
- **#3673:** useful-work/service equivalence.
- **#3675:** physical-state and terminal-state equivalence.
- **#3694:** owner-controlled offer authorization and reservation.
- **#3667:** chronological metric evaluator; consume only frozen full-horizon trajectories.
- **#3687:** PRAS v0.9 interoperability and probabilistic multi-region adequacy comparison.
- **#3730:** this protocol's statistical calibration and OOD/expiry work.

The capability receipt is an input to the adequacy model. It does not substitute for the power-system scenario model, transmission deliverability, ELCC calculation, regulatory accreditation or operator approval.

## 14. Research anchors

- LBNL, *A Guide for Improved Resource Adequacy Assessments in Evolving Power Systems* (2023): chronological simulation, multi-year weather and weather dependency.
- Liu, Wang, Botterud and Anjos, GERAD G-2026-36 (2026): stochastic capacity accreditation under correlated weather uncertainty and temperature-dependent thermal derating.
- ESIG Large Loads Task Force, *Resource Adequacy with Large Loads* (April 2026): incorporate large-load flexibility into adequacy planning from the outset.
- NIST TN 2387 (September 2026): local transactive services and owner-controlled flexibility participation.
- Symtropy #3652: repeated-event reliability and its independence/stationarity limitation.

## 15. Status and non-claims

This document freezes a research protocol only. It is not an implemented estimator, a validated statistical method, a field result, a system-level capacity assessment or a market qualification. No Rust compilation or test execution is claimed by this document.
