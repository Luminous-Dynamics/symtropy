# Time-aware navigation value of information v0.1

**Issue:** [NAV-TEMPORAL-VOI-001 #6730](https://github.com/Luminous-Dynamics/symtropy/issues/6730)  
**Parent:** [NAV-DEPENDENCY-GRAPH-VOI-001 #6720](https://github.com/Luminous-Dynamics/symtropy/issues/6720)

## Core change

A sensor observation is generated at sample time, but the decision may be made several ticks later. The sample-time posterior must be propagated through the action's declared latency before the next controller decision.

The temporal oracle uses a binary Markov service state with transitions conditioned on the static failure-domain pattern. Each sensing action:
1. samples the current latent service state and cause pattern;
2. produces a success/outcome or no-observation branch;
3. updates the posterior at sample time;
4. propagates that posterior through acquisition/transport/processing latency;
5. makes the next decision using the age-adjusted posterior.

Failure-domain variables remain fixed over this short horizon. Only the service state transitions.

## Model

For each cause pattern c:
- p01(c) is P(StateOne at t+1 | StateZero at t, c);
- p10(c) is P(StateZero at t+1 | StateOne at t, c).

Multi-tick propagation repeatedly applies the same transition matrix for the relevant cause pattern. The sample-time likelihood is applied before latency propagation. The no-observation branch is propagated too.

## Known-answer policy reversal

A static model selects the more accurate sensor even though its result arrives after four ticks. Under the temporal model, service drift makes a faster, lower-accuracy sensor better at the decision time.

Synthetic transition probabilities: StateZero-to-StateOne 2% per tick and StateOne-to-StateZero 15% per tick. The demo uses a fast sensor with 80% outcome accuracy and one-tick latency, and a slow sensor with 95% accuracy and four-tick latency.

The expected choice reverses from the slow high-accuracy sensor in the static planner to the fast lower-accuracy sensor in the temporal planner. These constructed probabilities demonstrate the principle; they are not estimated from actual vehicle dynamics.

## Limits

Static failure-domain bits, one binary service state, known per-tick transition probabilities, finite actions and horizon two. The model does not infer hazard rates from route data, represent continuous pose or time-varying cause states, or certify alert limits. Hazard rates must be estimated on independent calibration data and evaluated on held-out routes before any safety claim.
