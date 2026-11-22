# AV Action-Space EVSI Qualification v0

**Parent issue:** Symtropy #6750  
**Existing single-round action-value benchmark:** #6546 / PR #6549  
**Event-by-event execution:** #6555 / PR #6564  
**Candidate export:** Symthaea #13179  
**Observation/replay and eligibility:** #6201 / #6253  
**Modality necessity:** #6305 / PR #6307  
**Evidence memory:** #6357 / PR #6359

## Objective

Test whether a candidate understands the value of the *available choices*, not merely whether its estimate for the action it happened to request was plausible.

At each decision turn, the candidate forecasts gross expected value for every action in the candidate-visible catalogue and for explicit stop/no-acquisition, then commits to one selection. The evaluator checks the complete forecast set, independently scores unselected alternatives, derives net value under a published cost conversion, and reports whether the choice missed the best feasible option.

This v0 is a small, single-turn evaluator extension. It intentionally does not duplicate #6564's event-sourced executor. A subsequent integration binds forecasts and selected actions to #6564's append-only transcript and repeats the forecast after every delivered outcome.

## Why this closes a measurement gap

The existing #6549 evaluator reports EVSI estimate error for actions the policy actually requested. That is useful, but it cannot tell whether a policy assigned a terrible value to the better action it ignored. A policy can therefore appear calibrated on its selected actions while systematically overlooking better choices.

This benchmark makes every admitted alternative part of the forecast denominator, including the stop action. Missing alternatives, duplicate action IDs and stale transcript heads fail closed.

## Protocol

For each episode:

1. Candidate receives the current evidence-state digest, current tick, deadline, declared budget, action catalogue, action costs, arrival ticks and any public diagnostics.
2. Before any action outcome is revealed, candidate commits one forecast for every action ID plus the reserved `__stop__` ID. Each forecast carries a point estimate and lower/upper interval for gross EVSI.
3. The forecast is bound to the exact candidate-visible transcript digest and current forecast tick.
4. Candidate commits its selected action or stop decision in the same turn.
5. Evaluator checks all forecast IDs, interval consistency, timing and selection feasibility. It privately scores each alternative against evaluator-only potential-outcome labels.
6. The scorecard reports value calibration, pairwise net-value ranking, oracle-best feasible choice and decision regret separately.

In v0, every catalogue entry is an admitted alternative to evaluate. An alternative can still be deadline-infeasible or unaffordable; its infeasibility is computable from the candidate-visible arrival/cost profile and it cannot be selected. The forecast is made before outcome disclosure.

## v0 scenario controls

Three deliberately small synthetic episodes isolate:

- **High EIG / zero task value:** a visually complex detail has no decision value; another correlated re-listen is also low-value. Stop wins.
- **Low EIG / high task value:** a short non-speech alarm cue has low information-gain proxy but resolves a costly decision. It beats a visually busy dashboard.
- **Correlated versus late:** repeating a source already represented has low conditional value; an independent visual check would have value but arrives after the deadline. Stop wins.

These are hand-authored finite contract fixtures, not an empirical model comparison, stochastic simulator validation or SOTA score.

## Scorecard

No single weighted leaderboard number is produced. Report:

- action-space coverage (must be complete in v0);
- gross EVSI MAE and signed bias, including unselected alternatives;
- net EVSI MAE and signed bias under the published scenario cost weight;
- forecast interval coverage against evaluator-known expected gross values;
- pairwise ordering accuracy across non-tied alternatives;
- missed-best-action count;
- selected-action regret and mean regret;
- oracle-best feasible action selection accuracy.

The oracle is evaluator-only and is an upper-bound scoring instrument, not a candidate capability.

The current v0 cost conversion is an explicit synthetic scalar `net EVSI = gross EVSI - cost_weight × cost_units`. It is intentionally simple so the evaluator mechanics can be tested. Real deployments must report separate latency, energy, bandwidth, privacy and interruption dimensions, and preregister any conversion to task loss. Synthetic cost units are not physical energy or wall-clock time.

## Local execution

From the repository root:

```sh
python3 tools/av-modality-necessity-evaluator/action_space_evsi.py \
  --workload research/av-action-space-evsi-v0/fixtures/agent/workload-v1.json \
  --truth research/av-action-space-evsi-v0/fixtures/evaluator-only/potential-outcomes-v1.json \
  --submission research/av-action-space-evsi-v0/fixtures/agent/reference-submission-v1.json \
  --output /tmp/av-action-space-evsi-scorecard.json

python3 -m unittest discover \
  -s tools/av-modality-necessity-evaluator \
  -p "test_action_space_evsi.py" -v
```

The tool uses only the Python standard library. JSON Schema validation is part of the dedicated workflow.

## Limits and blind evaluation

The evaluator-only potential-outcome fixture is committed publicly for deterministic known-answer testing. It is not secret, blind or secure. The candidate must never be given evaluator potential outcomes in an actual run. A genuinely blind challenge requires evaluator-controlled storage/process isolation, independent custody, sealed seeds and truth, immutable candidate/model digests and a reproducible reveal/audit protocol. Public hashes alongside public truth do not create a blind benchmark.

The synthetic potential outcomes are deterministic point-value labels. They do not establish real EVSI calibration under stochastic worlds. The next evaluator tranche should use repeated paired worlds/common random numbers or a validated simulator oracle, with uncertainty around potential-outcome expectations. Unchosen observations are counterfactual labels, not actual sensor deliveries.

## Integration plan

1. Keep this scorer model-independent and deterministic.
2. Extend Symthaea #13179 to export all-action forecasts and the selected action before an outcome is revealed.
3. Bind each forecast to #6564's current transcript digest and action catalogue.
4. After #6564 is qualified, repeat this protocol at every multi-round turn and evaluate how forecasts update after partial, failed, late and delivered observations.
5. Run A0/A1/A2/A3/A4/A5 under matched workload, hardware, model revision and resource budgets.
6. Reproduce official active-perception / long-video protocols before making industry-superiority claims.

## Claim ceiling

This v0 qualifies evaluator semantics for complete action-space forecasts, simple synthetic cost-adjusted ranking and regret. It does not show that Symthaea or any other model is calibrated, optimal, causally reasoning internally, or better than SOTA. Those claims require real candidate runs, independent hidden evaluation, multiple held-out seeds, protocol-matched baselines and measured end-to-end resource use.
