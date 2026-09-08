# Animal Memory and Social Continuity V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define bounded future-bearing animal memory and social state without forcing every animal into permanent high-detail identity or pretending coarse populations can remember individual-specific facts they never stored.

## Core invariant

> Memory that changes future behavior is canonical process information and must have exactly one admissible owner across active/coarse/persistent fidelity.

## Candidate memory classes

A species may support a bounded subset such as:

- known water/feeding sites;
- shelter/den/nest sites;
- recent threat locations;
- migration-route landmarks;
- territory/home-range state;
- recent successful/failed foraging patches;
- dangerous encounters;
- mate/offspring/parent associations;
- group membership/dominance relationships;
- learned signal/resource associations.

## Boundedness

Memory has explicit capacity/decay/forgetting policy where applicable. An ordinary animal is not assumed to retain an unbounded event log.

The policy is biological content and versioned when it changes future behavior.

## Memory provenance

A memory derives from a percept/event/action outcome the animal could actually experience.

Do not write `predator at exact world position` into memory if the original percept supplied only a weak uncertain scent direction.

Memory may retain uncertainty/confidence and source modality where those affect future decisions.

## Individual vs aggregate state

Some memory is inherently individual-specific.

If an animal learns that a particular player/location is dangerous and that fact changes future decisions, collapsing the animal into an exchangeable population that cannot retain the association is information loss.

The collapse result is therefore C1/C2/C3 unless a qualified cohort/population representation retains an equivalent future-relevant statistic.

## Social continuity

Social relationships may require stable identity only when future semantics depend on continuity of particular individuals.

Examples likely requiring identity include:

- parent/offspring bonds;
- pair bonds;
- stable dominance relationships;
- individually learned cooperation/conflict;
- tracked/named companions.

Anonymous flocking/herding may often remain group/cohort state without persistent individual identities.

## Group state

Collective movement/social processes may maintain group-level canonical information such as:

- group centroid/home range;
- cohesion/alignment parameters;
- shared alarm state;
- migration objective;
- membership count/structure;
- nursery/offspring cohort;
- territorial boundary.

Group state is not a substitute for individual memory when the process explicitly needs individual relationships.

## Forgetting / decay

Forgetting is a canonical biological transition, not a side effect of unload or memory pressure.

A region unload cannot erase fear, route knowledge, or offspring relationships unless the model explicitly says those memories expired under canonical time/policy.

## Replay / persistence

Save/reload, fidelity transition, and region unload/reload must preserve the future behavior implied by retained memory state.

## Qualification direction

At minimum test:

1. an encounter cannot create memory without a compatible percept/event;
2. save/reload preserves subsequent memory-informed decisions;
3. renderer/entity destruction cannot erase memory;
4. canonical forgetting follows explicit time/policy rather than unload;
5. individual-specific learned fear blocks collapse to an insufficient exchangeable population;
6. group-level flock state can remain aggregate where no individual relationship is required;
7. uncertainty in the original percept can survive into memory and later action;
8. capacity/decay policy is deterministic for a fixed version.

## Non-goals

This contract does not prescribe neural memory models, unlimited episodic simulation, humanlike theory of mind, or universal animal social structure.