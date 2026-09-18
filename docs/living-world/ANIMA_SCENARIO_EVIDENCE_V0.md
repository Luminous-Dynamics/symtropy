# ANIMA Scenario and Evidence Identity V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Bind ANIMA qualification claims to exact scenario, product, policy, profile, timing, stochasticity, authority, and fidelity lineage rather than descriptive scenario names.

## Core invariant

> A qualification result is meaningful only for the exact semantic inputs and lineage that produced it.

Names such as `horse-wet-bridge` are descriptors, not evidence identity.

## Scenario manifest

A sealed ANIMA scenario binds, where relevant:

- manifest schema version;
- exact product/commit identity;
- parent contract set;
- species/body/receptor/decision profile identities;
- policy/update-rule identities;
- bounded numeric and stochastic policy identities;
- canonical clock/frequency policy;
- world/scenario seed;
- initial canonical state commitment;
- participant stable identities and profile bindings;
- enabled physical/ecological/cognitive processes;
- authority topology when networked;
- fidelity policy;
- horizon/end condition;
- interventions/perturbations;
- claimed invariants and measurements;
- expected artifact schema.

Mutable wall-clock timestamps, CI run numbers, and human-friendly labels are not semantic identity unless the claim explicitly depends on them.

## Canonicalization

The manifest requires one versioned canonical serialization/digest policy. Equivalent semantic manifests produce the same identity regardless of map insertion order, whitespace, or descriptor presentation.

Changing any declared semantic field changes identity.

Non-semantic descriptive metadata must be clearly separated from the hashed semantic body.

## Paired-run qualification

Many liveness claims are causal comparisons rather than standalone outcomes.

A paired-run relation binds:

- baseline manifest digest;
- variant manifest digest;
- declared perturbation family;
- exact changed semantic fields;
- expected first causal divergence class;
- comparison policy.

The pair preserves every undeclared semantic input, including deterministic initialization and keyed stochastic domains, except where the perturbation necessarily changes downstream causal keys.

Examples:

- same present horse state, safe-crossing history vs prior qualified slip -> first divergence should arise in memory/expectation, not raw perception;
- same world event, available signal path vs attenuated/occluded path -> first divergence should arise at receptor/percept availability;
- learned canine cue from partner A vs same cue from unfamiliar B -> first divergence should arise in partner-specific relationship/signal lookup;
- healthy vs degraded robot actuator with same task request -> first divergence should arise in body evidence/self-model capability.

If one authored perturbation necessarily changes derived state, that causal closure is recorded rather than falsely claiming the runs differ in only one byte.

## Evidence bundle

A completed sealed run may emit a compact bundle containing:

- semantic manifest digest;
- exact product/profile/policy lineage;
- initial-state digest;
- ordered canonical ledger digest(s);
- final-state digest and declared observables;
- measurements and claim outcomes;
- artifact hashes/references;
- paired-run first-divergence evidence where applicable;
- toolchain/environment fingerprint only when the claim requires it.

Detailed ledgers remain referenced artifacts rather than being duplicated without bound.

## Lineage rule

Evidence cannot automatically transfer after a semantic change to product code, policy/profile content, RNG algorithm/distribution, canonical scheduling, initial state, backend, intervention, authority topology, or fidelity semantics.

Transfer requires an explicit qualified equivalence theorem.

## Counterfactuals

Observatory counterfactual probes are analysis artifacts with identities distinct from canonical sealed runs.

A counterfactual:

- cannot mutate canonical biography;
- cannot be inserted into the original event chain;
- cannot be reported as evidence the agent actually experienced;
- must identify the snapshot and intervention from which it was derived.

## Human validation

Presentation and owner-playtest evidence may reference the same semantic scenario manifest while adding build/capture/reviewer metadata. Human experience claims remain separate from headless causal qualification.

## Initial reference scenarios

After their underlying product gates exist, useful small manifests include:

- fauna perception oracle/product differential;
- horse wet-bridge history control;
- canine human-cue disambiguation;
- canine scent loss/casting/reacquisition;
- deer non-telepathic alarm propagation;
- synthetic actuator-degradation active sensing;
- ANIMA Cell cross-species information flow.

Do not freeze these as qualified product scenarios before their dependencies exist.

## Qualification direction

At minimum test:

1. same semantic manifest canonicalizes/digests identically;
2. changing any semantic field changes identity;
3. changing non-semantic label metadata does not silently change semantic identity;
4. same manifest produces the same initial canonical state;
5. paired runs preserve undeclared inputs;
6. known paired fixtures expose the expected earliest causal divergence;
7. evidence loaded against incompatible lineage fails closed;
8. paired keyed stochastic domains remain closed under undeclared inputs;
9. save/reload preserves scenario lineage and result where exact equivalence is claimed;
10. networked scenarios bind authority topology/epoch semantics required by the claim;
11. Observatory trace detail cannot alter semantic manifest or result;
12. counterfactual artifacts cannot be accepted as canonical-run evidence.

## Non-goals

This contract does not claim that a manifest proves biological realism or scientific validity. It does not make run number, wall clock, filename, or scenario label a substitute for content identity, and it does not require one giant integration fixture to qualify all ANIMA behavior.
