# HoloFrac Held-Out Compositional Generalization Protocol v0

**Status:** deterministic typed-interface / grammar fixture  
**Implementation:** research/holofrac-retrieval/src/composition.rs  
**Executable:** research/holofrac-retrieval/src/bin/composition_generalization_sweep.rs  
**Workflow output:** composition-generalization.csv  
**Related:** [#6039 — held-out compositional generalization](https://github.com/Luminous-Dynamics/symtropy/issues/6039), [#12519 — Symthaea structural grammar handoff](https://github.com/Luminous-Dynamics/symthaea/issues/12519), [#12737 — HLB world-model handoff](https://github.com/Luminous-Dynamics/symthaea/issues/12737)

## 1. Research question

Can a system compose a valid spatial system from individually familiar modules when the complete multi-module configuration was absent from its training examples?

This is distinct from:
- retrieving a similar whole building;
- predicting a known edge type;
- generating many geometric variants of a single module;
- checking each port independently without validating the composed graph.

The first finite fixture deliberately holds out a four-module configuration that joins structural support, air supply and water supply into one habitable-space module. Each pairwise family/port-kind rule appears in one of the three training examples, but the full four-module signature does not.

## 2. Exact module and port contract

Each module declares:
- stable module ID;
- module family;
- spatial scale;
- typed ports.

Each port declares:
- stable local port ID;
- port kind;
- input/output/bidirectional direction;
- unit;
- coordinate frame;
- maximum capacity;
- alignment tolerance;
- whether the port is required.

Each connection declares exact source/target module and port IDs, demand and alignment error.

The current exact oracle checks:
- unique module and local port identities;
- endpoint existence;
- no self-edges or duplicate connections;
- matching port kind, unit and coordinate frame;
- compatible source/target direction;
- finite positive demand and aggregate port capacity;
- finite alignment error within both port tolerances;
- every required port connected.

Units and coordinate frames must match exactly in v0. Unit conversion and coordinate transforms must be represented as explicit, separately validated adapters in a later version; silent conversion is not permitted.

## 3. Held-out split

Training contains three independent two-module examples:
1. structural frame to habitable-space structural support;
2. air handler to habitable-space air supply;
3. water riser to habitable-space water supply.

The held-out case composes all four modules and all three pairwise connections together. It is therefore a new complete composition signature, even though each pairwise interface rule has been observed in the training set.

This is a finite grammar-oracle fixture, not a trained HLB result. It verifies that the exact rules are sufficiently composable and that the split is genuinely held out at the whole-composition level.

## 4. Falsification controls

The test suite mutates the held-out composition to include:
- unit mismatch;
- coordinate-frame mismatch;
- reversed/invalid port direction;
- demand above port capacity;
- alignment error above declared tolerance;
- disconnected required port;
- unknown module endpoint.

Each invalid composition must produce at least one exact validation violation. A proposal system must not be able to override or suppress these failures.

## 5. Required future comparisons

The next stage should add candidate proposal systems over the same sealed split:
- exact grammar search / constraint satisfaction;
- canonical typed-graph composition;
- nearest-neighbour retrieval from training compositions;
- HRR, FHRR, MAP, BSC, TPR and separately specified GHRR encoders;
- HLB-assisted proposal;
- hybrid HLB proposal followed by exact Symtropy verification.

Use identical module vocabulary, training examples, candidate budget, verifier, compute budget and downstream physics-call budget. Split on whole combinations and topology families, not random rows or edges.

Report proposal quality separately from verifier quality:
- held-out exact validity precision/recall/F1;
- train-to-held-out generalization gap;
- valid novel compositions per 1,000 proposals;
- invalid-interface escape rate after exact admission;
- proposal diversity and Pareto coverage;
- abstention calibration for unsupported module families;
- time and high-fidelity calls to first feasible composition;
- evidence closure after module, port, version or source mutations.

## 6. Prospective targets

These are research targets, not achieved results:
- 100% agreement with an independent oracle on the finite port/grammar contracts;
- zero invalid ports, units, frames, directions, capacity or tolerance cases admitted;
- at least 20% lower held-out generalization gap than retrieval-only at matched candidate budget;
- at least 2× valid novel compositions per 1,000 proposals over nearest-neighbour retrieval, without reducing hard-feasibility precision;
- zero false-current evidence after module/port/version mutation.

A passing finite grammar oracle does not establish that HLB has learned to generalize. It establishes only that the exact validation contract and held-out composition fixture are coherent.

## 7. Claim ceiling

This fixture tests typed composition only. It does not generate or validate final geometry, load paths, airflow, water performance, thermal comfort, building-code compliance or life safety. Real engineering claims require geometry realization, applicable qualified solvers, code/standard checks, source-bound receipts and independent professional review.
