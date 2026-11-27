# DIMACS / MCF corpus register v0.1

**Purpose:** identify external minimum-cost-flow benchmark sources and their provenance/licensing before importing any instance into the SAFRB comparison harness.
**Status:** source register only; no external corpus has yet been imported or benchmarked by this branch.

## Source A — DIMACS Network Flows 2.0 core MCF corpus

- Challenge: 13th DIMACS Implementation Challenge: Network Flows 2.0.
- MCF instances and generators: https://coral.ise.lehigh.edu/flow-challenge-2-0/mcf-instances-and-generators/
- MCF format: https://coral.ise.lehigh.edu/flow-challenge-2-0/minimum-cost-flow-input-format/
- Required core families listed by the challenge: NETGEN-8, NETGEN-CR, NETGEN-DEG; GOTO-8, GOTO-SR; GRID-WIDE, GRID-SQUARE (the live page currently prints `GRIP-SQUARE`), GRID-LONG; ROAD-PATH, ROAD-FLOW; VISION-RND, VISION-PROP.
- The challenge requests DNF (did not finish) reporting for instances a solver cannot complete and expects core instances plus additional informative families.
- The challenge explicitly notes that many historical instances/families are too easy for modern algorithms and invites new, harder instances.
- License and per-instance hashes: must be recorded from each individual source artifact before inclusion. Do not infer them from the challenge landing page.

## Source B — University of Bonn large MCF benchmarks

- Dataset: Held, Stephan (2024), *Large Benchmarks for the Minimum-Cost Flow Problem*, Bonn Mathematics Dataverse, version 1.0.
- Dataset DOI: https://doi.org/10.60507/FK2/5IKK7K
- File: `BonnMCFInstances.tar.bz2`, file DOI: https://doi.org/10.60507/FK2/5IKK7K/IFIMGJ
- Publication date: 2024-09-20.
- Published archive size: 311.2 MB.
- Published archive MD5: `f91e67456049615ff003b1ffe7f85159`.
- License/data-use: CC BY-NC 4.0; non-commercial restrictions apply. Check exact terms before reuse in commercial benchmarking, distribution or product deployments.
- Dataset description: VLSI placement legalization (`LEGALIZATION`) and linear relaxations of the discrete time-cost tradeoff problem used for voltage-threshold assignment (`TCT`); the dataset description says `LEGALIZATION` instances are significantly harder to solve than `TCT`.
- The dataset states that instances use DIMACS network format.
- No archive, individual file digest, family manifest or benchmark timing is included in this branch yet. The 311.2 MB archive must be acquired and independently hashed before execution.

## Import and benchmark protocol

1. Preserve the original archive and every extracted input byte-for-byte; hash the archive and each raw instance separately.
2. Record dataset DOI, file DOI, version, license, source revision, archive MD5 and locally computed SHA-256. Do not treat the publisher MD5 as a replacement for SHA-256.
3. Parse with `dimacs_mcf::parse_dimacs_mcf`; record parser version, raw digest, canonical digest, counts, nonzero supplies, signed-bound and negative-cost flags, and resource-budget estimate.
4. Never merge parallel arcs. Any solver-specific conversion must include a reversible mapping to the exact original arc ordinals and a separate mapping verifier.
5. Preserve signed lower/upper bounds and supply/demand; do not silently translate a model class the target solver cannot support. Report UNSUPPORTED/INDETERMINATE or use a correct lower-bound reduction.
6. Compare LEMON CostScaling, LEMON NetworkSimplex and Google OR-Tools SimpleMinCostFlow only after exact instance normalization and objective equivalence. NetworkX remains a secondary small-instance differential oracle, not the primary large-scale comparator.
7. For lexicographic priorities, use exact sequential passes and freeze earlier objective values; do not use an arbitrary Big-M scalarization.
8. Publish per-instance feasibility, objective tuple, DNF/timeout, P50/P95/P99 time, peak RSS, hardware/toolchain, certificate production and independent-verification time. Keep parse and model-build cost separate from solve time.
9. Keep tuning and test corpora disjoint. The challenge asks for fresh and harder instances; do not report performance on only historical or small generated examples as SOTA.

## Claim ceiling

A successfully parsed instance establishes only conformance to the supported input syntax and import contract. A solver result needs an independent primal/dual optimality verification before it can be called optimal. No benchmark superiority or SOTA advantage is claimed by this source register.
