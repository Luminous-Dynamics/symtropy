# EV factory common-random-stream intervention comparisons v0.1

**Related benchmark:** `EV-FACTORY-SIM-001`  
**Status:** variance-reduction improvement to a synthetic simulator, not a calibrated twin  
**Implementation branch:** `research/ev-factory-common-random-streams-v0-1`

## Why this matters

The original simulator used one sequential pseudorandom stream for supplier arrivals, cycle times, quality outcomes and failure draws. A baseline and an extra-server intervention change event ordering, so each run consumes random values in different places. The measured difference then contains two components:

1. the modeled effect of the intervention;
2. extra noise introduced because the two scenarios received different stochastic inputs.

That is avoidable confounding for a paired simulation experiment.

## Change

The report declares the random-stream policy. Supplier inter-arrival variation uses its own dedicated seeded stream, so service/quality draws cannot change the supplier arrival sequence. Service duration, quality outcomes and post-operation failure uniforms are derived from a stable keyed PRNG using:

- the catalog seed,
- a domain label (service-time, quality-outcome or machine-failure),
- station ID,
- pseudonymous simulation job ID,
- station attempt number.

The key is derived by explicit little-endian field encoding and FNV-1a rather than Rust's `DefaultHasher`, avoiding an implicit cross-version hashing contract. Each keyed draw is reproducible and independent of how many draws another job happens to consume. Baseline/intervention scenarios with the same catalog seed now reuse the same service-time and quality random variates for the same job/station/attempt. The same per-job/station failure uniform is reused; if changeover changes service time, the modeled hazard probability can still change.

## What this does—and does not—prove

This is a common-random-numbers (CRN) technique to reduce variance in scenario differences. It does **not** make one replication statistically sufficient, guarantee that every intervention's confidence interval is narrow, or prove a causal production effect. Event ordering, queue interactions, blocking/starvation and which jobs finish within the time horizon still change as station capacity changes.

The underlying synthetic DES approximates equipment failures as a hazard sampled after a job/station operation and a fixed repair interval. Keying that draw to the job/station/attempt improves scenario pairing but is not a physical machine-level reliability model. It does not preserve machine-specific degradation, failure age, failure during a cycle, shared repair teams or common-cause faults.

## Evaluation protocol

1. Freeze one catalog/fixture digest, a set of independently declared seeds, the warm-up, measurement horizon and operating calendar.
2. Run baseline and each intervention for every seed using the same exogenous supplier stream and keyed station/job/attempt variates.
3. Compute per-seed paired good-output differences, WIP, rework/scrap, blocked/starved/down time and resource use. Summarize mean/median, P05/P50/P95, and a paired confidence interval for differences.
4. Check the paired differences across independent seed sets and scenario regimes; don't publish only the best seed.
5. Calibrate cycle/yield/downtime distributions from rights-cleared measured traces, then freeze a time-separated holdout and compare predicted against observed output. Keep intervention-result labels out of fitting.
6. Retain independent site/product safety, quality and regulatory gates. `release_ready` remains false.

## Acceptance criteria

- Same seed, catalog and version produce byte-identical output.
- Keyed streams are repeatable, domain-separated and independent of unrelated random draw consumption.
- Identical stations/jobs/attempts get the same service/quality variates across baseline and one-server counterfactuals.
- Supplier inter-arrival stream stays separate and reproducible.
- All previous input/output-conservation, buffer-capacity, time-accounting, negative-control and artifact tests remain green.
- Published reports say whether CRN is enabled and carry the synthetic claim ceiling.

This is an engineering improvement to the experimental design, not a claim of state-of-the-art simulation fidelity.