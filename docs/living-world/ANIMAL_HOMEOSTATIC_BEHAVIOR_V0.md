# Animal Homeostatic Behavior V0

Status: normative Living World design contract. Documentation only.

## Purpose

Define animal behavior as bounded action selection over physiology, percepts, memory, ecological affordances, and species policy rather than omniscient scripted state switching.

## Core invariant

> Behavior responds to what the animal needs, perceives, remembers, and can do—not to unrestricted global truth or animation state.

## Inputs

Candidate canonical inputs include:

- energy/hunger;
- hydration/thirst;
- thermal stress;
- injury/pain/locomotor capability;
- infection/illness;
- fatigue/rest pressure;
- reproductive/parental state;
- current percepts with uncertainty;
- bounded spatial/episodic/social memory;
- local ecological affordances;
- species/developmental temperament priors.

## Behavioral hierarchy

A practical layered model may distinguish:

- reflex / immediate avoidance;
- motor intent;
- homeostatic action selection;
- spatial/episodic memory use;
- social/parental/territorial policy;
- richer cognition only for species/agents that justify it.

Not every animal needs a general-purpose cognitive architecture.

## Action intents

Behavior emits canonical intents such as:

- move toward/away;
- forage/graze/hunt;
- drink;
- rest;
- seek shelter;
- investigate/orient;
- flee/hide;
- defend;
- court/mate;
- care for offspring;
- vocalize/signal;
- interact with substrate/nest/den.

The intent is not the locomotion animation and does not guarantee success.

## Affordance-based ecology

Avoid hardcoded pairwise rules such as `WolfEatsDeer` where possible.

Action feasibility should compose from consumer capability, resource/prey/shelter affordance, need, accessibility, risk, and perception.

Species-specific specialization remains possible through traits/policy.

## Bounded rationality

Animals may choose locally reasonable actions without global optimization.

Canonical policy may include uncertainty, incomplete maps, limited memory, habit, exploration, risk sensitivity, social learning, and species priors.

This is preferable to perfect pathfinding toward resources the animal has never detected.

## Deterministic choice

For equivalent authoritative state, choice is deterministic or uses explicit keyed stochastic policy semantics. Sequential runtime RNG and container/thread ordering cannot change canonical choices.

Tie-breaking/fairness policy is versioned where it changes long-run behavior.

## Persistence / collapse

Any internal state that changes future decisions is future-bearing information.

Coarse collapse may summarize it only when enabled processes remain insensitive to the discarded detail. Individual-specific learned fear, den location, offspring bond, or territory may require coarse enrichment or persistent Level-I identity.

## Presentation split

Animation state follows canonical motor/action state. Animation transitions, montage completion, blend-tree state, facial expressions, or renderer frame timing cannot choose canonical behavior.

## Qualification direction

At minimum test:

1. severe thirst can dominate lower-priority exploration under an authored policy;
2. inaccessible/unperceived resources cannot be selected through omniscient lookup;
3. identical state -> identical action choice for a fixed policy version;
4. reordering candidate affordances cannot change outcome unless policy explicitly depends on canonical ordering;
5. injury can restrict feasible locomotor actions;
6. save/reload preserves future decision sequence for retained state;
7. animation/FPS changes cannot alter canonical action choice;
8. coarse collapse refuses to erase memory that an enabled policy still requires.

## Non-goals

This contract does not prescribe behavior trees vs utility systems vs active inference, universal animal psychology, or high-level humanlike cognition.