# OLI physical-memory control: independent Preisach hysterons v0

**Status:** deterministic Rust reference and tests added; exact-head Rust validation pending  
**Owner:** Symtropy / `symtropy-lifesim-core`  
**Machine-readable case:** `benchmarks/oli/preisach-hysteresis-memory-v0.json`

## Why this control matters

OLI needs a strict separation between:
- **physical information retention:** a system's present state depends on prior inputs;
- **adaptation:** system dynamics or parameters change in response to environmental conditions;
- **learning:** experience changes future performance on a defined task or improves prediction/control in a way that transfers to held-out conditions;
- **agency:** the system selects and pursues outcomes under operationally defined criteria.

These are not synonyms. Hysteresis is an especially useful counterexample because it is a well-studied physical form of path dependence and memory, but does not by itself implement task learning or agency.

Du, Behera & Vaikuntanathan (2025) review memory and information storage from the perspective of energetics, dynamics and statistical mechanics, including Hopfield models and physical implementations. The 2025 Science Advances paper *Generalizing multiple memories from a single drive: The hysteron latch* demonstrates return-point memory from hysteretic elements and richer nested sequence memory with frustrated interactions. OLI v0 deliberately starts with the simpler independent-hysteron case.

Sources:
- Du et al. (2025), Annual Review of Physical Chemistry, DOI [10.1146/annurev-physchem-083122-010308](https://doi.org/10.1146/annurev-physchem-083122-010308).
- *The hysteron latch* (2025), Science Advances, DOI [10.1126/sciadv.adr5933](https://doi.org/10.1126/sciadv.adr5933).
- Mörée & Leijon (2023), *Review of Hysteresis Models for Magnetic Materials*, DOI [10.3390/en16093908](https://doi.org/10.3390/en16093908).

## Model

The module `symtropy_lifesim_core::hysteresis` implements a finite, deterministic ensemble of independent relay hysterons. Each has a lower threshold, upper threshold, positive weight and binary state.

- A negative hysteron switches positive when the scalar drive reaches/exceeds its upper threshold.
- A positive hysteron switches negative when the drive reaches/falls below its lower threshold.
- Between thresholds, state is retained.

The ensemble reports positive-state weight fraction and normalized magnetization. Its hidden state is explicit and auditable. It has fixed thresholds and weights, no inter-hysteron interactions, no task objective, no adaptive parameter update, no stochasticity, no reproduction and no learning mechanism.

This is a rate-independent Preisach-style relay control. Do not confuse it with a rate-dependent dynamic hysteresis loop caused only by lag or phase delay.

## Frozen cases

Three equal-weight hysterons:
- H0: lower −1.0, upper +1.0
- H1: lower −0.5, upper +0.5
- H2: lower −0.2, upper +0.2

All begin in the negative state.

### Same present input, different histories

- History A: −2 → +1 → 0. All relays end positive, magnetization +1.
- History B: −2 → −1 → 0. All relays remain negative, magnetization −1.

The current input is identical (0); the physical state and output differ because of history. A memoryless scalar map (y=f(h)) cannot represent both outputs.

### Nested return-point memory

Drive −2 → +0.75 → 0 → +0.75. The full hysteron-state vector at the second visit to +0.75 must exactly match the first. This is a state-level criterion, not an aggregate-score coincidence.

### Major-loop erasure

Drive +2 then −2. The first saturates all relays positive; the second returns every relay to negative. This is a controlled, explicit erasure operation.

### Rate independence

Compare the coarse monotone sweep −2 → +1 with a finer sequence that includes intermediate values. The final relay state must be identical. Repeated dwell points do not create additional changes unless a threshold is crossed.

### No learning from repetition

Repeat −2 → +2 → −2 twice. The output trace repeats and thresholds/weights remain unchanged. This is physical memory with no adaptive parameter change or task performance improvement.

## Paradigm shift for OLI

A static image of a structure, a nonzero memory capacity, or a path-dependent response is insufficient to establish learning. OLI should require:
1. a defined task and performance metric;
2. an intervention that changes the information available through experience;
3. measurable improvement or strategy change beyond passive state retention;
4. controls for hysteresis, adaptation, habituation, drift and ordinary system dynamics;
5. held-out conditions demonstrating transfer/generalization;
6. independent replication.

The Preisach case should score strongly for scoped information retention, but must not be promoted to learning, intelligence, agency or life.

## Metrics and baselines

Report:
- same-input history separation;
- exact return-point state error;
- rate-independence state error;
- opposite-saturation erasure error;
- threshold/weight drift;
- output loop area from the recorded input-output trace;
- false-learning promotion count.

Compare:
- memoryless static mapping;
- direction-only control;
- independent Preisach hysterons;
- future interacting-hysteron latch.

All comparisons must use the same input sequences and declared state access. Do not split points from a single trajectory across train/test.

## Qualification targets (proposed, not achieved)

- Same-input history separation: exactly 2 in normalized magnetization for the frozen two-history case.
- Return-point state error: 0.
- Rate-independence state error: 0.
- Erasure error after opposite saturation: 0.
- Threshold/weight drift after repeated cycles: 0.
- False learning promotion from hysteresis-only evidence: 0.

## Claim ceiling

This is a synthetic known-mechanism physical-memory benchmark. It does not demonstrate task learning, agency, intelligence, biological life or consciousness. It is a counterexample that makes OLI's ontology more precise.
