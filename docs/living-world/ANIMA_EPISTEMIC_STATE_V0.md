# ANIMA Epistemic State V0

Status: normative Living World integration contract. Documentation only.

## Purpose

Refine the animal perception and memory contracts into a reusable embodied-agent boundary for biological and synthetic agents without creating an omniscient world-state cache.

## Parent contracts

This contract is subordinate to the Living World fauna contracts. It does not replace `ANIMAL_SENSORY_PERCEPTION_V0.md` or `ANIMAL_MEMORY_SOCIAL_V0.md`.

## Core invariant

> World truth, emitted evidence, raw percept, interpretation, remembered experience, current belief, and expected outcome are distinct authorities.

No downstream layer may gain exact semantic world knowledge merely because an upstream simulator component knows it.

## Evidence chain

Conceptually:

`world event -> emission -> propagation/path -> receptor -> raw percept -> interpretation -> experience/memory -> belief/expectation`

Each future-bearing transition retains enough provenance to explain which qualified evidence supported it.

## Carrier vs meaning

A receptor senses a supported carrier or operational channel. Meaning is generally inferred downstream.

A future reusable signal seam should distinguish physical/operational carrier from source/event semantics. Existing Basin `SignalKind` remains valid for its current contract and qualification line; this document does not retroactively redefine frozen perception subjects.

One event may produce several carriers. One carrier may have several possible causes.

## Experience sources

A retained experience may derive from:

- direct percept;
- bodily/action outcome;
- communication received through a qualified channel;
- social observation;
- explicit inference from retained evidence.

There is no admissible `WorldTruth` memory source.

Inference provenance must remain distinguishable from firsthand observation.

## Bounded memory

Species/platform policy may support bounded subsets of:

- working/active memory;
- episodic experience;
- associative cue/outcome memory;
- spatial/place memory;
- relationship memory;
- learned habit/policy bias.

An ordinary animal is not required to retain an unbounded event log.

## Belief and uncertainty

Beliefs may be uncertain, stale, contradictory, incomplete, or wrong.

A belief representation may retain:

- subject/hypothesis identity;
- confidence/precision semantics;
- supporting evidence references;
- contradicting evidence references;
- last update/freshness;
- alternatives where ambiguity matters.

Weak acoustic or olfactory evidence must not silently become exact target identity/coordinates.

## Revision

A versioned deterministic policy may strengthen, weaken, supersede, retain alternatives, mark stale, generalize, consolidate, or forget beliefs/memories.

Sequential runtime RNG, map iteration order, unload, renderer teardown, and memory pressure are not canonical forgetting policy.

## Consolidation

Repeated routine episodes may consolidate into bounded associations when the transformation preserves all future-relevant semantics required by enabled processes.

Canonical forgetting/consolidation is a cognitive transition. It is distinct from representation/fidelity collapse.

## Relationships

Partner-specific history may create expectations such as familiarity, safety, predictability, cooperation, handling sensitivity, conflict history, or learned communication conventions.

Relationship state does not reveal a partner's private internal state and should not collapse into one universal friendship scalar when pair identity matters.

## Backend independence

The contract does not require neural memory, HDC, vector databases, or one storage algorithm.

A deterministic baseline backend should exist before alternate backends claim equivalence or advantage. Any HDC/vector-symbolic backend is optional and requires differential evidence against a frozen reference corpus.

No capability claim should rely on HDC merely because Symthaea uses HDC elsewhere.

## Fidelity

Any memory/belief/relationship detail that changes future canonical behavior is future-bearing process information and participates in Living World information-sufficiency/fidelity authority.

Offscreen transition cannot erase it merely for performance.

## Qualification direction

At minimum test:

1. no compatible evidence path -> no source-attributable memory/belief;
2. same evidence/history/policy -> replay-identical retained state where exact semantics are claimed;
3. same current world/body state + different qualified history -> attributable later behavioral delta;
4. ambiguous raw percept cannot become exact semantic truth without additional evidence;
5. contradictory evidence remains provenance-visible under deterministic revision;
6. save/reload preserves future-bearing epistemic state;
7. unload/fidelity transition cannot cause forgetting;
8. consolidation preserves declared future decisions within its qualified equivalence class;
9. partner-specific expectations require partner-specific history;
10. alternate memory backend claims require differential qualification.

## Non-goals

This contract does not define human autobiographical memory, consciousness, unlimited episodic simulation, universal animal psychology, mandatory HDC, or unrestricted runtime reinforcement learning.
