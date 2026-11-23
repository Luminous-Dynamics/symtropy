# Anytime-Valid Episode Miscoverage Audit

## Purpose and scope

This module is an evaluator-side monitor for the true running-average miscoverage of whole-trajectory prediction sets. Each completed mission/trajectory episode is one decision/outcome step. The miss event is whether the complete true trajectory lies outside the bounded tube that was issued for that episode.

The audit implements the stitched empirical-Bernstein confidence-sequence margin described by Chencheng Fang, *Anytime Valid Conformal Prediction on Streaming Time Series*, arXiv:2611.20905 (16 November 2026): https://arxiv.org/abs/2611.20905.

This work is a research adaptation of that preprint. It is not yet an independently reviewed proof of the full navigation stack, a PHMI claim, or a certification.

## Statistical target

At episode t, let Z_t be one if a bounded episode tube was issued and the sealed ground-truth evaluator later found that the complete episode missed. Let nu_t be the conditional/true probability of that miss given the information available when the tube decision was made:

`nu_t = E[Z_t | F_(t-1)]`.

The true running-average miscoverage is

`bar_nu_t = (1/t) sum_(s=1..t) nu_s`.

The observed miss fraction is

`bar_Z_t = (1/t) sum_(s=1..t) Z_s`.

The paper's Lemma 4.1 provides a time-uniform confidence sequence of the form

`P(for all t >= 1: bar_nu_t <= bar_Z_t + omega_t) >= 1 - delta`.

For the stitched empirical-Bernstein margin (paper Equation 29):

`omega_t = [1.7*sqrt((Vhat_t vee 1)*((log(log(2*(Vhat_t vee 1))))_+ + 0.72*log(5.19/delta))) + 3.4*(log(log(2*(Vhat_t vee 1))))_+ + 2.45*log(5.19/delta)] / t`

where

`Vhat_t = sum over bounded episodes s<=t of (Z_s - S_(s-1)/N_(s-1))^2`,

`S_(s-1)` is the prior count of bounded-episode misses, and `N_(s-1)` is the prior number of bounded prediction sets; the prior mean is zero when N=0. The implementation uses `Vhat_t vee 1` in the margin, as specified in the paper.

The monitor reports `min(1, bar_Z_t + omega_t)` as the upper confidence sequence for true running-average miscoverage. It reports the confidence level `1-delta` and the frozen target alpha separately.

The implementation also preserves the paper's abstention rule: before the first bounded set, the scaled margin is zero; an abstention keeps `t*omega_t` unchanged by carrying forward the previous scaled margin.

The confidence sequence can be inspected at any stopping time. It does not require the episode outcomes to be IID, nor does it require a stationary miscoverage probability. The martingale-difference argument relies on the sequential filtration and on decisions being made before the corresponding outcome is revealed.

## Important impossibility boundary

The preprint proves that, under unrestricted unknown distributional shifts, no method that always issues bounded prediction sets can guarantee the true conditional miscoverage of every individual future prediction below alpha at arbitrary stopping times. Without further assumptions, a sufficiently severe shift can make the next prediction miss with probability arbitrarily close to one.

The useful target is a time-uniform bound on the true running-average or rolling-average miscoverage, not a per-episode conditional guarantee.

This module implements only the running-average audit. Rolling-average validity requires the paper's two-sided confidence sequence and explicit feasibility checks for every window that contains the proposed bounded set. It must not be approximated by simply reusing the running-average bound.

## Episode and abstention semantics

For a bounded, qualified tube:
- the sealed evaluator supplies a complete-episode hit/miss outcome;
- a miss increments S_t;
- a hit does not;
- the episode increments the bounded-set count N_t.

For an abstention (no bounded qualified set issued):
- the statistical audit treats the action as an unbounded prediction set, for which Z_t=nu_t=0;
- the episode increments the total monitoring horizon;
- it does not increment N_t;
- it does not count as a hit or a successful localization.

This distinction is essential. A system that abstains on every episode can have zero miscoverage, but it has zero qualified availability. The report therefore exposes bounded-episode availability and the operational availability target separately from the confidence-sequence risk target.

In a physical robot or safety process, UNKNOWN/UNAVAILABLE means no normal action may be authorized. The statistical convention that an abstention has zero prediction-set miscoverage must never be interpreted as permission to continue operating.

## Filtration and source contract

The caller must ensure:
1. The bounded/abstain decision is made using only information available before the episode's truth outcome.
2. The decision receipt is sealed before evaluation truth is inspected.
3. The complete trajectory is scored once, using the frozen episode score definition.
4. The outcome receipt commits to the full episode, truth source, tube identity, evaluator version and hit/miss result.
5. Episode identity IDs, episode sequence IDs, decision-receipt sequence IDs and bounded-episode outcome-receipt sequence IDs are strictly increasing. An abstention has no truth outcome or outcome receipt sequence; it cannot advance the outcome watermark.
6. Receipt digests are canonical and bind episode identity, sequence, frozen policy and decision/outcome data.
7. Ground-truth arrival or evaluation must never change historical controller-visible event inclusion, order or input data.

The module checks monotonic episode identity, episode/decision/outcome sequence progression, nonzero receipt digests and immediate receipt replay. The episode and outcome identifiers are source-ledger sequence identities, not arbitrary UUIDs. It does not compute SHA-256, authenticate the receipt contents, verify signatures, validate ground-truth independence or enforce the caller's filtration. Those are separate source/evaluator assurance obligations.

A bounded outcome with missing truth or a missing outcome receipt is not recorded as a hit. It remains pending/indeterminate and cannot advance the audit.

## Metrics to report

Every audit snapshot contains:

- total completed episode horizon;
- number of bounded qualified prediction sets;
- number of abstentions;
- whole-episode miss count;
- observed running miscoverage over all episodes;
- empirical miss fraction among bounded episodes;
- qualified availability;
- anytime confidence-sequence margin;
- upper confidence sequence for true running-average miscoverage;
- confidence level and target miscoverage;
- certified upper miss sum S_t + t*omega_t;
- separate availability-target status and combined operational-target status.

Do not publish the risk bound without its confidence level, target alpha, monitoring horizon, number of bounded predictions and qualified availability.

## Benchmark protocol

Compare on the same frozen controller and complete-episode event stream:

1. fixed split-conformal whole-episode tube from PR #6940;
2. observed running miss fraction alone (diagnostic only, not a time-uniform bound);
3. the stitched empirical-Bernstein anytime audit implemented here;
4. the paper's adaptive running-average set-width update only after a separate implementation/review;
5. rolling-window variants only after implementing the paper's feasibility check.

Include stable periods, abrupt shifts, slow drift, common-cause faults, stale calibration, source loss, anchor/clock/firmware faults and recovery events. Preserve the actual chronology; do not randomly shuffle episodes.

Report separate Pareto axes:
- true running-average miscoverage upper bound;
- observed miss fraction;
- bounded-set availability and abstention fraction;
- P50/P95/P99 tube radius and width;
- false-safe/fault-specific outcomes;
- recovery time, CPU, RAM and energy.

An always-abstain baseline is mandatory. It should show low miscoverage but zero bounded-set availability, demonstrating why risk and availability cannot be collapsed into one score.

## Current implementation status

Implemented in `anytime_episode_coverage.rs`:

- frozen alpha, delta and minimum qualified-availability policy;
- one completed episode as one audit step;
- whole-episode hit/miss input;
- abstention handling that contributes zero risk but not a hit;
- stitched empirical-Bernstein running margin;
- true running-average miscoverage upper confidence sequence;
- separate bounded-set, abstention and total-horizon counts;
- source decision/outcome receipt and monotonic episode, decision and truth-outcome sequence checks;
- fail-closed handling of missing truth and replay;
- tests for hits, misses, abstentions, always-abstain availability, all-miss behavior, replay and formula values.

Not implemented or demonstrated:
- adaptive radius/set-width update from the certified miss sum and buffer;
- rolling-window confidence sequence and feasibility check;
- source receipt cryptographic calculation or verification;
- real UWB trajectory truth adjudication;
- physical availability, integrity or safety results;
- independent proof review of the preprint-to-episode adaptation.

Keep this evaluator-side and research-only until source/evaluator receipts and the sequential information contract are independently qualified.
