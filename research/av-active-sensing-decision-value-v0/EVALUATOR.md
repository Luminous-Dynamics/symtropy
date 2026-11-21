# Active Sensing Decision-Value Evaluator

The deterministic evaluator is shared with the AV benchmark tooling:

tools/av-modality-necessity-evaluator/active_sensing_evaluate.py

Run from the repository root:

    python3 tools/av-modality-necessity-evaluator/active_sensing_evaluate.py \
      --workload research/av-active-sensing-decision-value-v0/fixtures/agent/workload-v1.json \
      --truth research/av-active-sensing-decision-value-v0/fixtures/evaluator-only/action-truth-v1.json \
      --submission research/av-active-sensing-decision-value-v0/fixtures/agent/reference-submission-v1.json \
      --output /tmp/av-active-sensing-reference-scorecard.json

## Scoring contract

The evaluator checks:
- exact workload/truth/submission episode alignment;
- unique request IDs, known action IDs, request timing and action max-use limits;
- scenario action outcome arrival = request tick + catalogue latency;
- scenario action catalogue budget for audio bytes, video frames and energy-cost units;
- candidate observation citations against initial observations and requested action outcomes only;
- future, post-cutoff, post-deadline and fabricated/unrequested evidence;
- evaluator-declared required observation refs for decision-changing action cases;
- common-cause dependency claims;
- exact action sequence versus the evaluator's optimal sequence;
- answer correctness, safe-decision correctness, deadline success, regret, wasted/late acquisition and realized EVSI.

The candidate's EIG, EVSI and cost estimates are validated for finite values and legal sign, but are not trusted as realized value. Realized EVSI uses evaluator-only scenario labels and is credited only for a delivered, on-time, independent observation from a within-limit request. Over-limit duplicates cannot multiply value. Late observations preserve identity but are not admissible for a decision before/after their deadline; partial/unavailable/failed outcomes contain no observation ref.

## Finite synthetic reference result

The hand-authored synthetic reference policy selects only three requests:
- inspect-hazard-tone for low-EIG / high-EVSI;
- capture-machine-audio for the audio-necessary machine-state decision;
- capture-cell-b-alarm for the scarce-budget cell-selection decision.

It stops on high-EIG / zero-EVSI, correlated common-cause views, late-only evidence, partial/unavailable sensor input and already-sufficient evidence.

The expected known-answer scorecard is:
- 8/8 answer accuracy and 8/8 acquisition/stop-policy correctness;
- zero decision regret;
- 3 acquisitions, with zero wasted or late actions;
- zero budget violations and zero evidence violations;
- mean realized EVSI 1.875.

The total 16,384 audio bytes and 1.3 energy-cost units are sums of the synthetic action catalogue's configured costs. They are not measured physical energy, wall-clock latency, production sensor bandwidth or model resource telemetry.

## Claim limits

This validates finite scenario and evaluator mechanics only. It does not establish that a model estimates EVSI well, that its internal evidence use is causal, that a physical microphone/camera can complete the actions, or that the policy beats an external baseline.

The public repository includes evaluator truth/outcomes and is not a blind test set. A blind evaluation requires hidden rights-qualified scenarios in an evaluator-controlled process/storage boundary, with the candidate receiving only the state and observations actually available at each step.

The next qualification tranche should add multiple acquisition rounds per episode, partial observations that change the follow-up action, repeated seeds, calibrated predicted-EVSI versus realized-EVSI reliability, explicit stop-decision metrics, and measured wall-clock/input/energy resource envelopes.
