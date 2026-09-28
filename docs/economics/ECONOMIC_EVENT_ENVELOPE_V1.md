# Symtropy EconomicEventEnvelopeV1

Status: engineering reference contract.

The envelope is the bridge between deterministic simulation truth and Mycelix Economic Fabric semantics.

## Required fields

- envelope_id
- world_id
- episode_id
- simulation_build_id
- deterministic_replay_id
- tick
- source_event_id
- source_event_type
- actor_refs
- resource_refs
- quantities_and_units
- causal_parent_refs
- observed_at_simulation_time
- emitted_at_wall_time
- origin
- evidence_refs
- validity
- correction_lineage
- schema_version

## Semantic rule

A simulation event asserts only a fact about the declared simulation world, episode, build and tick.

It does not assert physical-world occurrence, legal ownership, external-currency value, or real-world economic entitlement.

## Event classes

- ResourceProduced
- ResourceConsumed
- ServicePerformed
- AssetTransferred
- OfferCreated
- OfferAccepted
- DeliveryCompleted
- MarketCleared
- RepairPerformed
- OutcomeObserved

Market events must preserve the causal source event chain.

## Replay integrity

A consumer may accept a deterministic event as simulation evidence only when world_id, episode_id, simulation_build_id, deterministic_replay_id, tick and schema_version bind consistently.

A replay match is not independent external verification.

## External recognition

Recognition by Mycelix, another Symtropy world, Valueflows, Integral, TEND, or an accounting adapter must create an explicit recognition record. Recognition does not rewrite origin.

## Cross-domain claim ceiling

Simulation evidence can support simulation-level economic claims under the declared model. It cannot alone establish physical-world production, legal title, external-currency value, financial performance, or social/economic policy desirability.
