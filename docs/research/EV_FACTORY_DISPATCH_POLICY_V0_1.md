# EV factory bounded changeover-aware dispatch policy v0.1

**Related implementation experiment:** [EV-FACTORY-DESIGN-002 #2417](https://github.com/Luminous-Dynamics/symtropy/issues/2417)  
**Sequence instrumentation:** [PR #2414](https://github.com/Luminous-Dynamics/symtropy/pull/2414)  
**Status:** synthetic policy comparison; not a calibrated production scheduler or plant recommendation

## Policy contract

The catalog now has an explicit `dispatch_policy` field. Omitting it remains backward-compatible and selects `fifo`. Supported policies:

- **`fifo`**: remove the oldest job from the station's input queue.
- **`minimize_changeover`**: when an idle server has a prior variant, select the oldest waiting job of that variant if one exists; otherwise select the oldest waiting job. The policy uses that server's recorded last variant, does not look ahead at future events, and applies separately to the source buffer and each station input buffer.

The variant-aware policy preserves FIFO within a variant by selecting the first matching queue position. Each job carries a consecutive-skip counter while waiting in one queue. Every time a later queued job is selected ahead of it, the counter increments; queue transfer resets the counter. Once the oldest job reaches `maximum_consecutive_skips`, it is forced through before a later matching job can be selected.

It also applies an **elapsed-age priority override**: when the oldest queued job's time in that queue reaches `maximum_queue_age_seconds`, it is selected before a later matching variant. The fixture makes this threshold explicit at 900 seconds; omission in older catalogs defaults to 900 for backward compatibility. This is a synthetic benchmark knob, not a plant-derived service-level target. The threshold is checked only when an idle server has a dispatch opportunity: it is not a guaranteed maximum waiting time, because server occupancy, failures, blocking, and downstream congestion may continue to delay the job. The age override and consecutive-skip rule are separate guards.

This is a bounded local queue-selection rule, not a global optimizer, due-date scheduler, sequence planner, or safe plant controller. Job ID inversion remains a release-order proxy rather than a complete customer-order or service-level model.

## Fair comparison

The workflow runs the same frozen fixture and identical 20-seed replication list under both policies. The existing scenario catalog retains:
- the baseline line;
- one-extra-server interventions at each of the five stations.

The generated comparison report pairs the same scenario IDs and seed streams, and reports throughput means/deltas alongside changeover distributions and per-variant waiting-time statistics. It does not declare a winner. A policy must not alter supplier inputs, released job mix, buffer capacities, quality/rework rules, or release gates.

The simulator uses keyed per-job service/quality/failure random streams and a dedicated supplier-arrival stream. Comparing policies on the same seed list controls exogenous draws as far as the reference model's random-stream contract permits.

## Queue-wait measurements

A queue-wait observation is recorded when a job starts a station operation after warm-up. It measures seconds since that job entered the *current station input queue*. The source buffer timer starts when a supplier-backlogged job enters that buffer; supplier-backlog waiting remains separately reported. For each variant and scenario, the output includes sample count, mean wait, p95 wait and maximum wait for that replication, and across-replication distributions of those statistics.

A finite maximum consecutive-skip rule bounds how many later jobs can bypass an older job **while the job remains in that queue**, subject to the simulator's station/buffer progression. It is not a general guarantee of a due-date or delivery-time bound; blocked output, machine failures, upstream starvation and downstream congestion can still delay work.

## Required controls and interpretation

- **Legacy control:** omission of `dispatch_policy` defaults to FIFO, omission of `maximum_consecutive_skips` defaults to 3, and omission of `maximum_queue_age_seconds` defaults to 900.
- **Age-priority control:** an aged queue-front job must override variant matching at the configured threshold; a zero threshold is rejected. The threshold is not asserted as a hard wait-time guarantee.
- **Queue unit tests:** deterministic FIFO tie-break, oldest matching variant, FIFO within variant, fallback when no matching variant exists, and forced service after the maximum skip count.
- **Homogeneous/no-setup control:** one product variant and zero changeover durations must produce the same system behavior under FIFO and `minimize_changeover`.
- **Shared streams:** FIFO and variant-aware runs use the same ordered replication seeds, job release sequence, quality/failure keys and supplier stream.
- **Conservation and release guards:** output cannot exceed released input; buffers stay within capacity; station-time accounting stays within horizon; `release_ready` remains false for synthetic inputs.
- **Comparative metrics:** good units/hour, scrap/rework, changeover counts, release-order inversions, buffer WIP, per-variant input-queue wait, p95 and maximum wait. No weighted summary may conceal worsened waiting or loss of throughput.

A changeover-aware policy may reduce setups and still worsen output, waiting-time fairness or starvation under some mixes. The candidate is allowed to lose. Results must be reported without selecting a favorable seed or turning a synthetic improvement into a physical production claim.

## Prespecified age-threshold sensitivity

The benchmark sweep is fixed in advance at **300, 900, 1,800 and 3,600 seconds**. Each changeover-aware arm is paired against the identical FIFO replication seeds and scenario keys. The generated report retains seed-level deltas for good units/hour, total good units, scrap, rework, per-variant mean/p95/maximum queue wait, and station changeovers, transfers and output-order inversions. The 900-second arm reuses the current comparison report; the other arms change only the queue-age threshold.

The 20-seed paired 95% t intervals remain **unadjusted descriptive magnitude intervals**. The report now separates a predeclared primary family of 24 throughput contrasts (four thresholds × six scenario types) from secondary metrics. Each primary contrast uses a two-sided exact paired sign test; exact-zero deltas are counted as ties and excluded from the sign-test denominator. The 24 raw p-values receive Holm step-down family-wise error correction at alpha 0.05. This tests whether the median same-seed throughput difference is zero; it does not estimate the effect magnitude, test queue-wait fairness, or convert the fixture into a plant-calibrated model. Per-variant waiting, scrap/rework, changeovers, transfers and order inversions remain exploratory guardrails, and their displayed t intervals are not multiplicity-adjusted. The threshold grid remains prespecified; don't choose a best setting merely because one exploratory interval looks favorable. More seeds, an independent holdout scenario family, calibration and prospective site evidence remain necessary before deployment-oriented claims. All results remain synthetic and cannot establish a real factory waiting-time target.

## Statistical decision contract v0.2

For a fixed hypothesis family, family-wise false-positive control is part of the benchmark—not an after-the-fact report decoration. The confirmatory family is intentionally narrow: exactly 24 median-throughput sign tests, one for each combination of four prespecified age thresholds and six scenario types. Holm-adjusted p-values and raw per-seed sign counts are serialized so an independent consumer can recompute the result. The output retains the 95% paired t intervals for effect magnitude, labelled unadjusted, and makes no confirmatory claims about the many secondary waiting, quality or sequence metrics.

The exact sign test is chosen because the paired delta samples are small and may contain exact zeros; it avoids assuming normality of throughput differences. Its inference is about the median sign/effect, not a guarantee about tail waits or practical significance. An adjusted p-value below 0.05 is not a production-release gate. The report remains a sensitivity screen on 20 synthetic replications.

## Claim ceiling

This is deterministic, stochastic-replication software testing over synthetic assumptions. It does not validate a real plant's cycle times, changeover rules, workforce, quality, HV/battery safety, regulatory compliance or production readiness. The policy is not authorized for dispatch to a MES/PLC and is not an investment recommendation.
