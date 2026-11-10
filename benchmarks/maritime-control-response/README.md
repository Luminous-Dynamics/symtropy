# LiU-ICE command-response benchmark lane v0.1

**Parent owner:** Symtropy issue #4628, MARITIME-CONTROL-AWARE-DIAGNOSIS-001  
**External benchmark:** [LiU-ICE Industrial Fault Diagnosis Benchmark 2026](https://vehsys.gitlab-pages.liu.se/dx_benchmarks/liuice/liuice_index), DX37  
**Status:** independent Rust evaluator primitives; dataset payload not yet audited and no benchmark score claimed  
**Claim ceiling:** typed semantics, synthetic contract behavior, and exact score-formula implementation only

## External benchmark fit

The public LiU-ICE page describes an engine test-bench dataset at 20 Hz with two requested commands and eight measured signals. The commands are \`umf\` (requested injected fuel mass) and \`uwg\` (requested wastegate actuator position). The signal list includes intercooler pressure and temperature, intake manifold pressure, air-filter mass flow, throttle actuator position, engine speed, ambient pressure and ambient temperature.

The evaluator deliberately maps \`umf\` and \`uwg\` to \`CONTROL_INTENT\`. It maps \`yxpos\` only to \`CANDIDATE_ACTUATOR_FEEDBACK\`, pending review of the instrument/I-O dictionary. It is not a wastegate-position signal and is not promoted to verified feedback. The other listed measurements are classified as physical response or disturbance context according to their public descriptions.

The benchmark lists four known fault classes: intake-manifold leakage and multiplicative faults affecting \`ypic\`, \`ypim\` and \`ywaf\`. Its official isolation interface has a fifth slot for unknown/unclassified faults. The public training set is reported as 26 recordings, with only 2–7 recordings per known fault family; evaluation may include driving cycles other than WLTP. See the [official benchmark description](https://vehsys.gitlab-pages.liu.se/dx_benchmarks/liuice/liuice_index).

## Implemented evaluator primitives

- Typed signal-role registry, including the distinction between requested commands and measured plant response.
- Exact public competition score calculation: \`((1 - FAR) + TDR + FIA) / 3\`, with five isolation probabilities in the published order.
- FIA uses the mean probability assigned to the true diagnosis among correctly detected fault samples; it is not top-1 accuracy. Undefined FIA denominators are explicitly reported.
- Exact row-key matching for labels and predictions; probabilities must satisfy the official detection-dependent sum contract.
- Whole-recording split manifests and optional held-out-driving-cycle split separation.
- Monotonic recording-row checks and missing-channel preservation without zero or nominal-command imputation.
- Command-conditioned response residual evaluation with distinct command-availability, prediction and physical-observation timestamps. It requires command availability at or before prediction, response strictly after prediction, a bounded response horizon, and command freshness measured at prediction time. Missing, stale, future, unsupported and uncalibrated states are not normal. A residual mismatch never independently claims an actuator fault.
- Split-conformal calibration using one maximum standardized residual per independent recording/episode, not 20 Hz samples or overlapping windows.
- Calibration receipts retain their sorted episode identities and maximum residuals. Their digests and finite-sample order statistic are recomputed; the threshold must match the frozen model, regime, training manifest and split manifest.

## Statistical constraint

For the finite-sample split-conformal order statistic, the minimum number of independent calibration episodes is 9 for 90% marginal coverage, 19 for 95%, and 99 for 99%. A regime with fewer independent recordings is not calibrated to that target and must remain not-evaluated. Exchangeability of independent episode maxima is still an assumption; this library does not prove it across different cycles, fault modes or regimes.

The small 26-recording training corpus cannot substantiate 99% calibration per fault and per driving cycle. More data from independent runs/engines and an explicit instrument dictionary are required.

## Required benchmark tournament

Evaluate on common frozen recordings/splits, same channel availability, equivalent model capacity and disclosed compute:

1. Official Fault Diagnosis Toolbox analytical/structural model, where its public configuration and outputs can be reproduced.
2. State-only residual / statistical process-control baselines.
3. Command-only baseline.
4. Naive command + state concatenation.
5. Command-conditioned response model with regime and disturbance context.
6. Gated/control-aware fusion and multi-class regime discrimination.
7. Calibrated selective / unknown-fault rejection baseline.

The official page describes a MATLAB or Python competition interface. This Rust crate is an independent evaluator and evidence-verification component, not a submission-ready competition interface. A separate adapter or method reproduction is necessary for direct entry.

## Report beyond the leaderboard

Always report FAR, TDR, FIA and official total score, but do not stop there. Add event recall, false alarms per recording-hour, detection latency, per-fault/per-cycle performance, unknown-fault false-known rate, ambiguity/abstention, risk-coverage, interval coverage and sharpness, worst-regime performance, model size, inference latency and resource use. Group uncertainty by independent recording/engine campaign, never by rows.

A test bench with one engine supports within-engine drive-cycle/magnitude generalization. It does not establish transfer to another engine, vessel or fleet.

## Claim boundaries and next steps

- The official training ZIP has not been downloaded or audited in this workstream. No data SHA, trained model score or competition score is claimed.
- The public landing page does not establish a redistribution licence for the archive. Do not republish the data until licensing is confirmed.
- Inspect the archive, variable dictionary, sampling/time semantics, data completeness and fault-label rules before freezing feature and split manifests.
- Compare published baselines only under a recovered, matched protocol. Mark unreproduced paper scores as literature references.
- Physical validation requires permissioned synchronized controller intent, independently measured actuator feedback and physical plant response from an engine testbed, with an independently reviewed I/O map.

This benchmark lane does not write controllers, actuate equipment, authorize maintenance, establish safe-life limits, or clear operating restrictions.


### Prediction-time causality control

The response evaluator records three distinct times: when the command became available to the diagnostic process, when the frozen model issued its expected response, and when the independent physical response was observed. It rejects a command that arrived after prediction, a response that predates or coincides with prediction, a response beyond the declared horizon, and a command that was already stale at prediction time. This is stricter than merely checking that the command precedes the measurement; it blocks an after-the-fact command from being used to explain an earlier prediction.