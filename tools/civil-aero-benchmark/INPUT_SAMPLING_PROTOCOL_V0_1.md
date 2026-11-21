# Correlated Latin-hypercube input design protocol v0.1

## Scope and evidence ceiling

This module generates repeatable input designs for global uncertainty studies. It does not execute a solver, evaluate a response, estimate output uncertainty, or validate a physical model. Each resulting row is an input condition to be evaluated by a separately attested solver. Until those evaluations are attached and audited, the output has no solver or validation evidence.

## Method

1. Each declared input uses a normal marginal with mean, standard uncertainty, and hard physical domain bounds.
2. For each dimension, a seeded randomized Latin hypercube assigns one independent probability to each of N equal-probability strata.
3. An Acklam inverse-normal approximation converts each stratified probability to an independent standard-normal score.
4. A separate independent Gaussian score matrix is transformed by the caller-supplied Cholesky factor to create a latent correlated ordering.
5. Iman-Conover-style rank reordering maps each marginal's sorted LHS values onto sample IDs ordered by their latent correlated scores. Thus every final physical marginal retains exactly one LHS probability in each of the N strata.
6. Each marginal normal score is mapped to a physical input using its mean and standard uncertainty.
7. A sample outside any declared physical domain fails the complete design. The implementation does not clip values, silently truncate distributions, or replace invalid samples.
8. The design reports the target latent Gaussian-score correlation, its empirical finite-sample correlation, and the empirical Pearson correlation of the final physical input values. Neither finite-sample empirical matrix is claimed to equal the target exactly.

This is an Iman-Conover-style correlated Gaussian LHS design using a Cholesky-generated latent normal ordering to reorder stratified marginal values. It preserves the LHS marginal strata exactly and induces approximate dependence. It only supports normal marginals in v0.1. Perfectly correlated / singular matrices are rejected because the Cholesky factor is not positive definite; the tool does not regularize or perturb the caller's matrix. The requested matrix is treated as a latent Gaussian-score correlation target; the achieved physical-input Pearson matrix is separately reported and must be inspected.

## Required downstream evaluation record

A downstream adapter should attach, for every sample:
- sample ID, design seed, study ID and exact input values;
- solver source/build digest, mesh and geometry digests, physics configuration digest;
- solver exit status, convergence histories and conservation defects;
- the scalar and field responses of interest;
- actual CPU/GPU/wall-clock/energy costs and memory consumption;
- raw log/output digests and independent audit result.

Failed and out-of-domain samples must remain visible and retain their incurred cost. A generated design alone is not a Monte Carlo or Latin-hypercube output uncertainty estimate; that estimate requires model evaluations over the samples and appropriate analysis of the resulting response distribution.

## Synthetic validation

The CLI generates 128 samples for three synthetic inputs (Mach, Reynolds scale, static temperature) with a declared correlation matrix. Rust tests verify deterministic output, one final physical sample in every probability stratum per dimension, target latent and empirical physical correlation diagnostics, physical-bound fail-closed behaviour, singular-matrix rejection and metadata retention. An independent Python oracle verifies the CSV schema, stratification, bounds, input membership and the target correlation rows.

The synthetic inputs are not the official DPW8 OAT15A input uncertainties and are not solver evaluations.

## References

- NIST IR 8298, Section 4.2, recommends global input-space methods such as Monte Carlo or Latin-hypercube sampling when local sensitivity propagation is inadequate, and recommends accounting for correlation among inputs: https://nvlpubs.nist.gov/nistpubs/ir/2020/NIST.IR.8298.pdf
- AIAA DPW-8/AePW-4 test conditions: https://www.aiaa-dpw.org/TestCases/testcases.html
