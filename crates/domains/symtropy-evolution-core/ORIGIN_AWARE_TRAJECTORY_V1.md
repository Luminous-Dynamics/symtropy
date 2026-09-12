# Origin-Aware Trajectory + Mutation-Origin Fate Delta V1

Status: source contract for POPGEN-05D / #715. This is not executable qualification.

## Purpose

POPGEN-05D gives POPGEN-05C a richer causal trajectory coordinate and an exact descriptive fate delta for current mutation-origin classes.

It introduces no new stochastic process. It wraps the existing POPGEN-05C transition.

## Origin-aware trajectory point

`OriginAwarePopulationTrajectoryPoint` binds:

- exact hereditary schema digest;
- exact `OriginAwarePopulationStateDigest`;
- exact underlying `PopulationTrajectoryPointDigest`;
- the underlying ordinary trajectory point itself.

Equal aggregate allele counts are therefore insufficient for point identity when origin partitions differ.

A restored point is data only until `validate_current(...)` proves it against the exact current origin-aware state.

## Fate delta

`OriginFateDelta` is deterministically derived from one validated source and destination origin-aware state.

For every active mutation origin present in either endpoint it records:

- locus ID;
- allele ID;
- mutation-origin digest;
- source origin count;
- destination origin count;
- source allele count;
- destination allele count;
- destination locus total.

It also records modeled-baseline source/destination counts by locus + allele.

Lost origins remain present in the delta with destination count zero.

## Descriptive predicates

`MutationOriginFateDelta` exposes count-derived predicates:

- `lost`: source count > 0 and destination count == 0;
- `persisting`: destination count > 0;
- `fixed_within_allele`: destination origin count equals the destination allele count;
- `fixed_at_locus`: destination origin count equals every sampled copy at the destination locus.

These predicates are **descriptive states only**.

They do not establish:

- beneficial/deleterious/neutral effect;
- fitness;
- selection coefficient;
- adaptation;
- selective sweep;
- ecological advantage;
- causal reason for increase/loss/fixation.

All of these states can arise under neutral drift.

## Continuation

`continue_origin_aware_trajectory(...)`:

1. revalidates the exact source origin-aware trajectory point;
2. delegates the biological transition to POPGEN-05C unchanged;
3. binds the destination origin-aware state to the ordinary destination trajectory point;
4. derives the exact neutral fate delta;
5. returns one replayable transition object.

## Origin context invariant

One `MutationOriginDigest` may not change locus or allele identity across the source/destination states.

A later mutation that changes the allele is represented by a new active origin upstream in MUT-05C; POPGEN-05D never relabels an old origin onto a different allele.

## Baseline interpretation

Modeled baseline means only that no currently carried modeled mutation origin explains those copies. It is not proof of mutation-free biological history.

## Qualification requirements

Promotion requires exact-head execution demonstrating at minimum:

- reference start declaration/revalidation;
- equal aggregate allele states with different origin partitions yield different trajectory-point identities;
- destination point binds the exact POPGEN-05C destination state;
- lost origins remain visible in fate deltas;
- recurrent origins under one fixed allele remain distinct;
- origin loss can occur while the allele stays fixed;
- origin fixation predicates are exact count statements;
- baseline deltas conserve source/destination allele partitions;
- Serde restore requires exact replay;
- wrong state/point/transition/profile fails closed;
- no fitness or selection fields are introduced.
