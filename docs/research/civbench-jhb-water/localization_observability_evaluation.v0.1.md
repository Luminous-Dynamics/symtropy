# CivBench localization observability evaluation v0.1

**Status:** proposed executable evaluation contract; no achieved field-performance claim  
**Evaluation owner:** Symtropy / CivBench  
**Hydraulic observability and sensor-placement owner:** Sol-Atlas  
**Decision closure and service-outcome owner:** Symtropy  
**Authority, appeal, privacy, and institutional-governance owner:** Mycelix

## Research basis and why the field needs a shift

Leak detection and pipe localization are different inference problems. Recent studies demonstrate that the performance gap can be physical, not just algorithmic:

- El Hachoumi et al. (AQUA, 2026) evaluated a calibrated pressure-driven Moroccan district network using 10 pressure sensors, 5 flow sensors and 648 scenarios. Detection AUPRC was 0.901, zone performance 0.700, nearest-sensor accuracy 0.423 (0.185 system-level), and Top-5 pipe accuracy 0.122 across 171 pipes. A zone oracle improved Top-5 pipe accuracy only to 0.170, with sensing layout identified as a binding limitation. DOI: https://doi.org/10.2166/aqua.2026.300
- Tarawneh (Water Practice & Technology, 30 October 2026) reviews 244 records, identifying 71 pure deep-learning studies. The review reports that 75% of studies relied on simulated data, 59% did not share code or data, and only 11.3% reported inference time. Its benchmark reports Time-Transformer detection accuracy of 96.1% / ROC-AUC 0.94 and GGNN localization success of 88.5% within 300 metres, but those metrics are not directly interchangeable with pipe Top-K accuracy on another network. DOI: https://doi.org/10.2166/wpt.2026.485
- Farah & Shahrour (Water, September 2026) screened 90 studies and found substantial variation in scenario generation, validation practice, and metric definitions. They specifically recommend evaluating scenario leakage from random splits, jointly optimizing sensor placement and detection, and improving uncertainty and deployment reporting. DOI: https://doi.org/10.3390/w18182267
- Ranganathan et al. (Journal of Water Process Engineering, May 2026) report >95% detection accuracy across 47,000+ EPANET/WNTR Net3 scenarios. This is a useful synthetic detection comparator, not a prospective field guarantee. DOI: https://doi.org/10.1016/j.jwpe.2026.110114

The paradigm shift is to treat *physical identifiability as an explicit benchmark input*. Before asking whether a GNN, Transformer, tree model or detector chose the right pipe, ask whether the registered observations and sensor layout can distinguish that pipe at all.

## Executable contract

The Rust `localization_eval` module consumes two independent evidence streams:

1. **Oracle observability evidence** from Sol-Atlas or another hydraulic/physical owner: the pipe ambiguity set consistent with the frozen observations, network model, sensor layout and declared tolerance; resolution class; source receipt digest.
2. **Model prediction evidence:** ranked pipe IDs, explicit abstention, and a probability (parts per million) that its stated Top-K list contains the true pipe.

The evaluator validates:
- the true pipe belongs to the declared network;
- every oracle candidate and every model-ranked pipe belongs to the same frozen network/version universe;
- unique-pipe resolution has exactly one candidate, ambiguous/zone-only resolutions have at least two, and non-spatial/unmeasured/unsupported statuses carry no invented pipe set;
- the independently supplied truth belongs to trusted oracle candidate sets;
- prediction IDs are unique; abstention cannot carry a hidden ranking or confidence; and issued predictions require an in-range confidence;
- oracle and predicted evidence are referenced by digests, with the explicit limit that digest syntax is checked but cryptographic authentication is outside this module.

Trusted oracle resolutions are `unique_pipe`, `ambiguous_pipe_set` and `zone_only`. `event_only`, `model_uncertain`, `not_measured` and `unsupported` remain distinct unscorable classes. Unknown is never converted to zero.

## Metrics that must be reported together

For a declared K:

1. **Oracle Top-K feasibility:** share of trusted-oracle cases whose complete physical candidate set contains no more than K pipes. This is a guaranteed feasibility ceiling, not an expected model accuracy.
2. **Conditional model Top-K accuracy:** hits divided by issued predictions on trusted-oracle cases.
3. **System Top-K accuracy:** hits divided by all trusted-oracle cases; abstentions on otherwise scorable cases count as misses.
4. **Scorable prediction coverage:** issued predictions divided by trusted-oracle cases.
5. **Safe vs unnecessary abstention:** abstentions where the oracle is unscorable or candidate-set cardinality exceeds K are distinguished from abstentions where the full trusted candidate set fits within K.
6. **Unqualified pipe claims:** issued pipe rankings when the oracle is event-only, model-uncertain, unmeasured or unsupported.
7. **Risk–coverage:** Top-K hit rate and risk at preregistered confidence thresholds. The confidence is the predicted probability that the Top-K list contains truth, not merely a softmax score for the top pipe.

Do not collapse these into one weighted score. Always publish the denominators and the number of independent networks, events, sensor layouts and topology families.

## Recommended benchmark ladder

| Lane | Frozen physical/evidence boundary | Baselines |
|---|---|---|
| Full observability reference | Dense/reference sensor layout; trusted model and tolerance | Oracle candidate set, nearest sensor, hydraulic residual |
| Fixed sparse layout | Same network and fixed sensor positions; held-out event/template groups | Threshold, Random Forest/boosting, GNN, temporal/Transformer |
| Sensor placement comparison | Same sensor count and cost; random vs hydraulic sensitivity/observability-driven placement | Random placement, standard hydraulic placement, sensitivity-ranked placement |
| Cross-network | Unseen network IDs; event/session/scenario-template holdouts | Target-only, source-only, no-adaptation, same-architecture random-init |
| Cross-topology | Unseen topology family and scenario template | All above under identical label and tuning budget |
| Degraded sensing | Missing sensors, noise, drift, calibration error, demand regimes | Fixed-layout baselines plus controlled missingness/noise ablations |
| Prospective utility | Locked time/site holdout, independently adjudicated events and field confirmation | Existing crew process, acoustic survey, satellite screening where available |

A cross-network score is not a cross-topology score. A sensor-layout transfer score is not a real-world transfer score. A random sample split is not a generalization protocol. Use the split-integrity presets before reading model results.

## Proposed CivBench targets (not achieved results)

- **Observability:** under equal sensor count, installation/maintenance cost and network coverage, improve the share of trusted cases whose oracle candidate set fits within Top-5 by at least 20% relative to random placement over held-out scenarios, with uncertainty intervals.
- **Model skill:** at the same oracle class and coverage, improve Top-5 accuracy by at least 20% relative to the strongest matched baseline. Report UniquePipe, AmbiguousPipeSet and ZoneOnly strata separately.
- **Selective quality:** reduce Top-K risk by at least 25% relative at matched coverage, or increase coverage by 15 percentage points at matched risk, on held-out network/topology groups.
- **Uncertainty:** achieve calibration error (e.g. expected calibration error) <=0.05 on the frozen test corpus, reported by network, sensor layout, demand regime and leak magnitude. The current v0.1 evaluator supplies risk–coverage points; a separate preregistered calibration report should be added before making this claim.
- **Abstention:** zero unqualified pipe claims on EventOnly, ModelUncertain, NotMeasured and Unsupported cases. A high-confidence pipe assertion when the physical evidence is not trusted is a safety failure, not a useful prediction.
- **Operational value:** any localization gain must improve feasible survey/dispatch selection under the same crew-hours, parts, travel, shift and acoustic-confirmation budget; measure verified leaks per crew-day, false investigations, time-to-confirmation and repair closure.
- **Service:** keep water recovery, pressure/supply continuity, essential-service floors and worst-served area as downstream outcomes. Pipe-localization gains are not themselves service improvements.

The 20% and 25% targets are research gates to test, not claims of superiority today.

## Industry comparison without false equivalence

SUEZ / ASTERRA / Affinity Water report screening 9,045 km, prioritising 763 locations, confirming 902 leaks, recovering an estimated 1.26 billion litres annually, walking less than 4% of the network, and identifying 7.2 leaks per crew-day; 59% of confirmed leaks were non-surfacing. This is a provider case study, not an independently randomized controlled comparison. Satellite anomaly prioritisation and acoustic field confirmation are distinct stages.

A second SUEZ / ASTERRA case at SES Water reports over 1,000 km analysed, 211 likely leak locations and 219 confirmed leaks over 59 days, or 3.7 leaks per crew-day and 2.2 leaks per kilometre inspected. These are operational productivity measures, not directly comparable to sample accuracy, event recall or Top-K pipe metrics.

Johannesburg Water's 29 October 2026 update describes reservoir/tower recovery constrained by high demand and draw-off, low pressure, intermittent supply and prolonged outages; the proposed Level 1 restrictions target approximately 15% demand reduction and include safeguards for basic supply and essential services. Its 23 July 2026 technology update states 53 pressure controllers in service by April 2025, 306 additional controllers procured and being installed, and 225 noise loggers installed with 125 planned by October 2026. Those target dates have passed; final installed counts must be verified before using them as a current infrastructure inventory.

For Johannesburg evaluation, low pressure and outage duration must be stratified by elevation, reservoir/tower supply area, essential-service status and outage history. A leak survey that prioritises a pipe but worsens reservoir recovery or basic service is not a successful policy.

## Ownership and integration boundaries

- **Sol-Atlas:** hydraulic sensitivity/Jacobian, pressure-driven model validity, sensor layout, network topology, oracle ambiguity sets, localization and sensor placement.
- **Symtropy / CivBench:** consume owner-supplied oracle receipts; validate candidate universes, separate observability ceiling from model ranking, score Top-K/coverage/risk/abstention; pair survey policies under common resource envelopes; connect verified interventions to service outcomes.
- **Mycelix:** authority, privacy, contestability, audit access and institutional feedback.

No second EPANET/WNTR solver, sensor placement optimizer, value-of-information engine or autonomous PLC controller is introduced here.

## Source references

- El Hachoumi et al. (2026), AQUA 75(10), 1014–1036, https://doi.org/10.2166/aqua.2026.300
- Tarawneh (2026), Water Practice & Technology, https://doi.org/10.2166/wpt.2026.485
- Farah & Shahrour (2026), Water 18(18), 2267, https://doi.org/10.3390/w18182267
- Ranganathan et al. (2026), Journal of Water Process Engineering 87, 110114, https://doi.org/10.1016/j.jwpe.2026.110114
- Villacres, Putri & Moazeni (2026), Water Research 304, 126295, https://doi.org/10.1016/j.watres.2026.126295
- SUEZ / ASTERRA / Affinity Water, https://www.suez.com/en/uk/case-studies/satellite-leak-detection-delivers-results-affinity-water
- SUEZ / ASTERRA / SES Water, https://www.suez.com/en/uk/case-studies/suez-satellite-leak-detection-transforms-water-management-for-ses-water
- Johannesburg Water, 29 October 2026, https://www.johannesburgwater.co.za/blog/2026/10/29/johannesburg-water-implements-urgent-interventions-to-stabilise-severely-constrained-systems/
- Johannesburg Water, 23 July 2026, https://www.johannesburgwater.co.za/blog/2026/07/23/johannesburg-water-accelerates-smart-technology-rollout-to-reduce-water-losses/
