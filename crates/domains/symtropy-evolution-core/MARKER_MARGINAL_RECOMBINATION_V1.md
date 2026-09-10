# Marker-Marginal Poisson Recombination V1

Status: implemented/source-reviewed only. Not executable-qualified.

## Purpose

This contract executes `PoissonCrossoversNoInterferenceV1` at the resolution the current chromosome model actually observes: ordered modeled loci in genetic-map space.

It derives exact modeled-locus gamete content under that process while deliberately **not** realizing or claiming exact hidden crossover coordinates.

## Core distinction

`exact modeled-locus inheritance != exact hidden breakpoint history`.

For two adjacent modeled loci separated by distance `d`, the hidden gametic crossover count obeys the selected C3A Poisson/no-interference process. The source homolog changes between the two loci iff that hidden count is odd.

C3B2A2 therefore samples only the sufficient statistic needed by the current model: odd/even parity.

## Initial homolog selection

The initial source homolog uses the exact C3B1 V1 opportunity grammar:

- domain: `symtropy:evolution:linked-gamete:no-crossovers-independent-assortment:v1\0`;
- reproduction event ID;
- parent-role tag;
- chromosome ID.

No chromosome iteration ordinal is included.

If both complete source haplotypes are identical, the local source coordinate is canonicalized to slot 0 because C2 provides no ancestry authority capable of distinguishing the two homolog copies.

## Adjacent-interval probability

For every adjacent locus pair, V1 obtains the odd-parity threshold from `HALDANE_PARITY_ORACLE_V1`:

`P(odd) = (1 - exp(-2d)) / 2`.

The returned threshold is integer ppm in `0..=500_000`.

## Adjacent-interval opportunity draw

For each adjacent interval, V1 derives an unbiased integer in `[0, 1_000_000)` from SHA-256 using domain:

`symtropy:evolution:linked-gamete:marker-parity-poisson-no-interference:v1\0`

followed by canonical encodings of:

- reproduction event ID;
- parent role;
- chromosome ID;
- left locus ID;
- right locus ID;
- rejection-attempt counter.

The first 64 digest bits are interpreted little-endian. A rejection zone whose accepted size is divisible by `1_000_000` removes modulo bias. The retry counter changes only if a digest lands in the rejected tail.

Genetic-map distance is deliberately absent from this stochastic key. Distance changes the deterministic threshold and exact map/profile authority, not the underlying opportunity field. This permits common-random-number map-distance counterfactuals.

## State transition

For each chromosome:

1. choose the initial local source homolog using the C3B1 opportunity field;
2. emit the first modeled locus allele from that homolog;
3. walk adjacent mapped loci in exact C1 biological order;
4. compute exact map distance;
5. compute Haldane odd-parity threshold;
6. derive the unbiased interval opportunity draw;
7. classify `draw < threshold` as `Odd`, otherwise `Even`;
8. toggle local source slot on `Odd` when the two complete source haplotypes differ;
9. emit the next modeled allele from the resulting source slot;
10. record exact interval parity evidence.

For allele-identical complete homologs, parity is still sampled under the process, but the unobservable local homolog coordinate is kept canonical at slot 0.

## Evidence

Each adjacent interval records:

- chromosome ID;
- left and right locus IDs;
- exact genetic-map distance in micromorgans;
- exact Haldane odd-parity threshold in ppm;
- exact semantic opportunity draw in ppm;
- sampled parity;
- local source-haplotype slot after the interval.

The full provenance additionally binds exact schema, chromosome map, recombination profile, source phased state, reproduction event, parent role, and resulting `LinkedGamete` digests.

Restored evidence must deterministically re-execute against externally expected event/role and current authorities before regaining derivation authority.

## What `Odd` means

`Odd` means:

> Under the selected Poisson/no-interference process, the sampled hidden crossover count in this genetic-map interval has odd parity.

It does not mean:

- exactly one crossover;
- a crossover at the midpoint;
- a known breakpoint coordinate;
- a physical base-pair location.

Likewise, `Even` includes hidden counts `0, 2, 4, ...` and therefore does not mean that no crossover occurred.

## Regression boundaries

A realization in which all adjacent intervals are `Even` must produce the same modeled-locus gamete as C3B1 zero-crossover inheritance for the same source/event/role/chromosome initial opportunity field.

Adding unrelated chromosomes must not shift existing chromosome-local opportunities. Changing map distance between the same locus IDs must not reroll the interval opportunity draw. Changing exact map/profile authority still stales provenance.

## Scientific limitations

V1 assumes the explicit C3A Haldane/Poisson no-interference model. It does not model crossover interference, hotspots beyond what future process profiles may express, gene conversion, structural variants, mutation, physical DNA coordinates, polyploid meiosis, selection, or persistent ancestry-node identity.

## Successor

A narrow validated-linked-gamete contribution interface should allow the existing child assembler to consume either C3B1 zero-crossover or C3B2A2 marker-marginal derivation evidence without accepting raw gametes.

A later higher-fidelity lane may realize hidden crossover counts and positions only when downstream ancestry/sequence authority genuinely needs that information.
