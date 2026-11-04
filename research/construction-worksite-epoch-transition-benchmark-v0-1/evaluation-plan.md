# Dynamic worksite epoch benchmark — statistical evaluation plan v0.1

Issue/PR: #3542. This is a preregistration template, not a result.

## 1. Separate deterministic conformance from physical reliability

The frozen transition corpus is a software-contract test. Its target is exact: zero false-safe transitions, zero stale/conflict command leakage and zero implicit restart. Report corpus version, scenario count, test count, oracle version and exact source SHA. The current oracle includes four versioned scenarios, strict Draft 2020-12 fixture validation, and a bounded invariant sweep across 14,641 event sequences (16,105 prefixes). Keep this deterministic sequence corpus separate from generated/fuzzed tests and from physical encounter denominators.

A physical encounter campaign estimates performance only within its declared task, machine configuration, ODD and sampling frame. Passing the frozen corpus is not evidence of a field probability of failure.

## 2. Define the independent experimental unit

Before collection, define the independent unit for each endpoint:
- **Human/obstacle detection:** one independently staged target encounter with independently recorded target ground truth; repeated frames from one encounter are not separate encounters.
- **Epoch invalidation:** one independently randomized context-change event; message retries, duplicated packets and repeated controller cycles from one event do not increase n.
- **Stopping distance:** one physical stop under a predeclared payload/speed/surface/slope/brake/mode combination; repeated sensor frames inside one stop are not trials.
- **Site replication:** a site means a separately managed construction site with independent physical layout and work coordination, not another room or day at the same site.
- **Configuration replication:** a declared change in relevant robot, payload, tool, sensor, brake, controller or software configuration, not a software label change with identical as-built hardware.

Record cluster identifiers for site, shift, robot, configuration, operator role, task and encounter. Analyse clustered observations with a preregistered hierarchical or cluster-robust method; do not count correlated observations as independent Bernoulli trials.

## 3. False-negative / false-safe bounds

For a binary endpoint with zero observed failures in n genuinely independent Bernoulli trials, the exact one-sided (1-alpha) upper confidence bound on the unknown failure probability is:

`p_upper = 1 - alpha^(1/n)`

At 95% one-sided confidence (alpha=0.05), this is approximately the rule-of-three bound 3/n for sufficiently large n. Illustrative planning values:
- n=30, zero observed failures: upper bound is about 9.5%;
- n=100: about 3.0%;
- n=299: about 1.0%;
- n=2,995: about 0.1%.

These figures are not acceptable safety thresholds. They show why zero observed failures in a small pilot does not establish a tiny failure probability. A risk assessment and safety case must set the required bound and confidence level before the experiment. Where one claim spans M separately asserted strata/endpoints, control multiplicity or report separate intervals; do not silently pool strata to make a weak stratum disappear.

If failures occur, report exact one-sided/two-sided binomial intervals and the full numerator/denominator. Never report only a percentage or an aggregate pass rate. State the sampling frame and any exclusions.

## 4. ODD stratification and challenge sampling

Pre-register safety-relevant strata before looking at outcomes: geometry/zone-change type, access state, trade/plant overlap, surface/slope, payload/speed/mode, sensor modality/health, lighting/weather/dust/occlusion, comms/clock condition and software/configuration. Report measured, unmeasured and excluded cells explicitly.

Use two separate datasets:
1. **Challenge set:** oversamples rare/adversarial transitions to test contract robustness. It is not prevalence-weighted.
2. **Operational sample:** sampled from a declared worksite/ODD distribution to estimate real-world rates. Preserve inclusion probabilities so weighted prevalence estimates are possible.

Do not combine the challenge set and operational sample into one rate. Hold out entire sites/configurations for generalization tests. Use preregistered split ownership and blind custody for acceptance-critical challenges.

## 5. Timeliness and stopping-envelope endpoints

Capture independent monotonic timestamps and the active timebase/boot identity for:
- physical/context event onset;
- sensor observation;
- context update creation and receipt;
- decision/inhibit output;
- safety output actuation;
- physical stop completion.

Record clock synchronization uncertainty, monotonic timebase identity, instrument latency and timebase changes/restarts. A qualification lease expires at the preregistered task-policy deadline; a clock-domain reset or boot/timebase identity change invalidates old leases. Report p50/p95/p99/max plus sample count and confidence/tolerance bounds. A sample p99 alone is not a guaranteed worst-case bound. For safety-critical deadlines, define a one-sided tolerance-bound method and test instrumentation uncertainty before collection.

For every physical trial, calculate the remaining clearance margin using the measured stopping envelope for the exact payload, speed, surface, slope, brake state and mode. Include U95 and an explicitly justified reserve. Report every violation, not just mean margin.

## 6. Baselines and comparisons

Vendor and manual comparators must run under matched task definition, design revision, ODD, floor/surface, control network, calibration regime, robot configuration, environmental condition and acceptance criteria. Use the same functional unit and count only accepted output. Randomize or counterbalance order where practical; record exclusions and pauses. Vendor-published accuracy or productivity is a comparator for planning, not independent safety evidence.

Report separate scorecards:
- safety: false-safe, misses, stale command leakage, unsafe restart;
- measurement: reference uncertainty, calibration, timebase, traceability;
- response: inhibit/stop latency distribution and margin;
- operations: accepted output/crew-hour, downtime, requalification time, false holds;
- transfer: held-out site/configuration performance.

No weighted composite score may trade a safety failure against productivity.

## 7. Stop rules and requalification

Pause/abort physical testing on any false-safe event, stale/conflicting epoch command, unauthorized restart, loss of independent ground truth, expired critical calibration, corrupted custody, unexplained clock discontinuity or stopping-envelope breach. An independent review determines scope of invalidation, corrective action and whether the test series must restart. Never delete a failed run from the denominator.

## 8. Reporting package

Every campaign release should include:
- preregistration and risk-derived thresholds;
- exact robot/as-built/software/payload/sensor/calibration configuration;
- ODD profile and task/design/acceptance baseline;
- raw event data, independent reference data and timebase;
- sample-unit ledger, exclusions, cluster IDs and challenge custody;
- analysis source, dependency lock and environment receipt;
- per-stratum numerators/denominators, intervals, uncertainty and all failure narratives;
- exact commit, artifacts, hashes and independent adjudication reference.

No output from this plan authorizes operation or claims certification.