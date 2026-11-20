# External restoration case contract v0.1

**Programme:** [POWER-FABRIC-EXT-COMP-001 #6320](https://github.com/Luminous-Dynamics/symtropy/issues/6320)  
**Purpose:** safe, data-only interchange between independently built restoration planners and validators.

## Why this exists

The LARA paper is a required comparator, but its public code snapshot needs provenance, rights and model-fidelity qualification before any fair reproduction. The source profile pins the public repository to commit `0e7d2d79241ce20b2c39a5b0d7fd0f5f60fc2016` (26 March 2026). That initial commit is unsigned, and no LICENSE file appears in its 16-file commit inventory. The five IEEE-13 scenario files are Python pickle binaries; their Git blob SHA-1 IDs and byte sizes are recorded, but their raw bytes and SHA-256 values have **not** been obtained in this pass. Rights/reuse review is pending. No artifact is redistributed.

Python's official documentation warns that unpickling malicious or tampered payloads can execute arbitrary code. The LARA repository's environment loads scenario pickles with `pickle.load`; its scenario-creation helper parses archive text using regex and `eval()`. Our adapter must not unpickle or execute that parser on untrusted source text. The target interchange is an explicit, schema-validated JSON data model. Sources: [Python pickle security warning](https://docs.python.org/3.13/library/pickle.html), [LARA source](https://github.com/jishnuwarrier/LARA-Power-Restoration).

## Important source-code fidelity observation

LARA's README describes physics-aware simulation enforcing radiality, power flow and resource constraints. In the audited `dsr_engine/environment.py`, the observed implementation uses NetworkX graph reachability, radiality cycle checks, and aggregate mobile-source active P, reactive Q and apparent S checks against Pmax/Smax. The audited file contains no voltage, impedance, AC power-flow, OpenDSS or pandapower implementation references. `main.py` also performs aggregate P/Q/S validation for MPS assignments. This is a bounded source audit, not a claim about all unpublished paper experiments or any external simulator.

Consequently, our reproduction needs an independent electrical model/validator. A schedule passing a graph/radiality/aggregate MPS screen is not equivalent to a per-step AC operating point, voltage/thermal verification or transient stability qualification.

## Unit ambiguity to resolve

The source `components.py` comments identify load P/Q as kW/kVAr, while prompts and `main.py` diagnostics use W/VAR labels. The source profile marks active/reactive unit bindings as ambiguous. Do not compare absolute power, energy, restoration reward or service deficits until a source-specific unit binding and conversion receipt is reviewed.

The paper describes a five-minute restoration timestep. A benchmark case must bind the timestep, horizon, time-origin and conversion explicitly; do not silently infer duration from an integer timestep.

## Data-only interchange

The schema separates:
- power-network edges from travel-network edges;
- loads and priority weights from physical and objective units;
- damaged locations/repair demand from crew capacity/efficiency;
- mobile-power active/apparent limits from its energy/fuel inventory;
- graph/radiality constraints from independent AC, voltage, thermal and frequency validation;
- upstream source artifact provenance from normalized values and transformation receipts.

Unknown electrical parameters remain null/referenced, never invented. The example fixture is deliberately synthetic and not extracted from LARA.

## Reproduction protocol

1. Resolve licence/reuse permission and review source commit provenance.
2. Acquire exact binary scenario bytes by a trusted workflow; compute SHA-256 without executing them.
3. If conversion is approved, perform a constrained offline export in an isolated environment and compare source object inventories against a manually transcribed data-only JSON.
4. Bind every unit, timebase, topology/road mapping and objective transformation to an explicit receipt.
5. Run LARA, MILP/CP-SAT and our planner against the exact same instance and frozen independent simulator/validator.
6. Record LLM model/provider revision, prompt hash, temperature, context limit, retries, API latency/cost, simulator hash, planner wall clock and invalid action proposals.
7. Keep the paper's cumulative weighted-load objective for the replication tier. Evaluate hard beneficiary floors and independently accepted service in a separate service-assurance tier.

## Synthetic fixture

`synthetic-restoration-case-v0.1.json` is an illustrative four-node graph, one repair crew, one mobile source, one fault and one normally-open switch. It exercises the data shape only. It is not a LARA test case, IEEE-certified model, or physically validated electrical network.

## Claim boundary

Source inventory and data-only schema only. No LARA run, binary import, licence clearance, objective reproduction, electrical simulation, operational feasibility, SOTA comparison or field restoration result is claimed.

## Expanded comparator set (17–20 November 2026)

The comparison set now includes four distinct problem families beyond LARA:

1. **Explainable Agentic AI for Distribution System Load Restoration** (Chowdhury & Li, arXiv:2611.22016, 17 Nov 2026). Five agent roles; constrained feasible-action lists; receding-horizon plan generation; deterministic pandapower AC checks; IEEE 33/69/118; 250 scenarios per topology split 200/25/25; DQN, Actor-Critic, MCTS and SingleLLMAgent baselines. The preprint reports ~25.8% average weighted-load improvement over its single-agent ablation, alongside 94.64k vs 25.38k tokens and 26.06 vs 21.25 min per scenario. Treat as a mandatory paper comparator, not a reproduced result or general topology-transfer proof.
2. **Multi-stage unbalanced M-SOP restoration** (SEGAN 2026, DOI 10.1016/j.segan.2026.102576). Unbalanced three-phase, pre-event/fault-isolation/service-restoration phases, asymmetric VSC ports and soft-open-point support. This motivates per-phase evidence rather than a balanced-only AC certificate.
3. **Hybrid AC/DC restoration with VSC FIDs and storage** (Journal of Energy Storage 2026, DOI 10.1016/j.est.2026.123636). 207-node hybrid AC/DC system with black-start ESS, multiple VSC modes, cold-load pickup and rolling-horizon MILP. This is a separate network class.
4. **Frequency-dynamics-aware restoration** (PSCC 2026 / DOI 10.1016/j.epsr.2026.113573). IEEE 9-bus work demonstrates static-optimal switching can violate dynamic frequency constraints. Static AC is not a dynamic-stability certificate.

These are not one leaderboard. Match the network class, action set, physical model, objective, uncertainty, resource constraints and verification boundary before comparison.
