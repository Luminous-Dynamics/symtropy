# Anytime Rolling-Window Episode Miscoverage Control

## Purpose

This module extends the evaluator-side episode coverage audit from running-average to rolling-window true miscoverage. A lifetime running average can conceal a recent burst of misses after an abrupt environment or hardware change. Rolling windows provide a separate guarantee over recent episode history.

The implementation follows Fang (2026), *Anytime Valid Conformal Prediction on Streaming Time Series*, arXiv:2611.20905, Section 4.2, Equations (36)-(41), Algorithm 2 and Theorem 4.2:
https://arxiv.org/abs/2611.20905

This is a research implementation of the rolling decision and certified-sum logic. It is not a predictor/set constructor, a per-episode conditional-risk guarantee, PHMI certification or actuator authorization.

## Window rule

For episode t, a predeclared rule produces a window length k_t and start:

`s_t = t - k_t`.

The module supports:
- **Fixed:** (k_t=min(t,K)), a correctness/reference track; its asymptotic efficiency loss does not vanish.
- **Proportional:** (k_t=lceil 	heta tceil), (0<	hetaleq1).
- **Polynomial:** (k_t=lceil t^etaceil), (1/2<eta<1).

For fixed windows (	au_s=s+K) when s>0 (and (	au_0=1)). For proportional windows with (	heta=n/d<1), (s_t=lfloor t(d-n)/dfloor) and the first time a window starts at s is (	au_s=lceil sd/(d-n)ceil); when (	heta=1), only start zero occurs. For polynomial windows, the implementation uses a deterministic lower-bound search over the predeclared schedule to find the first t such that (s_t=s).

The crucial detail is that (	au_s) can be in the **future** relative to the current episode. It is the first time that the predeclared window rule will use start s, not merely the first time that start has already been observed.

## Statistical confidence sequence

Let (Z_t) be the whole-episode miss indicator for a bounded prediction set, and let (
u_t) be its conditional/true miscoverage probability given the past. At abstentions, (Z_t=
u_t=0); the confidence sequence is updated only on bounded predictions.

The rolling target is:

`P(for all t: nu_bar_t^(k) <= alpha) >= 1-delta`.

The construction uses a **two-sided** anytime confidence sequence, because the rolling cumulative true-miscoverage sum is the difference of a prefix at t and a prefix at s_t. For Fang's stitched empirical-Bernstein margin, the two-sided confidence sequence uses the one-sided Equation 29 margin at error level (delta/2).

For a prefix ending at t, the scaled two-sided margin is (tomega'_t), computed from the full three-term stitched empirical-Bernstein expression:

`t*omega'_t = 1.7*sqrt((Vhat_t vee 1)*((log(log(2*(Vhat_t vee 1))))_+ + 0.72*log(5.19/(delta/2)))) + 3.4*(log(log(2*(Vhat_t vee 1))))_+ + 2.45*log(5.19/(delta/2))`.

The predictable variation (widehat V_t) is the sum of squared deviations ( (Z_i-S_{i-1}/N_{i-1})^2 ) over bounded-set episodes only, with the prior bounded-set miss fraction treated as zero when no bounded set has yet been issued.

At an abstention, the algorithm carries forward the scaled margin unchanged:

`t*omega'_t = (t-1)*omega'_(t-1)`.

A rolling-window prefix snapshot stores (N_t), (S_t), (widehat V_t) and (tomega'_t). Prefix state is retained for every completed episode.

## Certified rolling miss sum

For a window from s+1 through t, Fang (2026), Equation (39), defines:

`C(s,t) = min{ N_t - N_s, S_t - S_s + t*omega'_t + s*omega'_s }`.

The first term is the trivial upper bound: each bounded set has conditional miscoverage at most one, while abstentions contribute zero. The second term follows from the two-sided confidence sequence.

The next miscoverage level is Equation (40):

`eta_(t+1) = alpha + gamma*(alpha*k_t - C(s_t,t) - B)`.

Here B>=0 is an efficiency buffer. Unlike the running-average algorithm, there is no fixed buffer that replaces the rolling feasibility check.

## Prospective feasibility check

A bounded set at episode t is issued only if eta_t>0 and, for **both** possible outcomes z in {0,1}, every future window that could contain episode t remains within its allowance even if later episodes abstain.

For each relevant window start (sin{s_t,ldots,t-1}) with finite first occurrence (	au_s), compute:

`C^(z)(s,t) = min{N_(t-1)+1-N_s, S_(t-1)+z-S_s + t*omega'_(t,z) + s*omega'_s }`

where (omega'_{t,z}) is the two-sided margin as if a bounded set were issued at t and its miss indicator were z.

The smallest possible future window length containing episode t is:

`m_t(s) = max(t,tau_s) - s`.

The bounded set is feasible only when:

`C^(z)(s,t) <= alpha*m_t(s)`

for every candidate z and every relevant s.

The implementation emits a witness per (window start, candidate outcome), including first occurrence, certified sum, allowance and pass/fail. If any witness fails, the decision is abstention. If the predeclared compute budget would be exceeded, the decision is also abstention; the implementation never truncates the feasibility loop and then issues a bounded set.

This check is prospective: the current miss/hit is unknown when deciding, so both z=0 and z=1 must pass.

## Decision/outcome ordering

For episode t:
1. Read only completed history through t-1.
2. Calculate k_t, s_t and eta_t.
3. Compute future first-occurrence (	au_s) for every relevant start.
4. Compute both hypothetical outcomes and all prospective feasibility witnesses.
5. Request a bounded set at exactly eta_t only if eta_t>0 and every witness passes; otherwise abstain.
6. Seal the decision receipt and source event stream.
7. After the whole trajectory and truth are sealed, submit the complete-episode miss/hit and outcome receipt.
8. Update N_t, S_t, Vhat_t and the two-sided margin; compute C(s_t,t); store the prefix snapshot.

Ground truth must not alter the prediction decision, sensor inclusion, observation order or controller-visible event stream.

## Abstention and availability

An abstention has no bounded-set outcome receipt, does not increment N_t or S_t, and keeps t*omega'_t unchanged. It does increment the total episode horizon and reduces bounded-set availability.

The reported rolling empirical miscoverage uses ((S_t-S_s)/k_t). The bounded-set availability is ((N_t-N_s)/k_t). The certified rolling upper bound is (C(s,t)/k_t). These are reported separately: an always-abstain policy is not a high-availability success.

## Efficiency and window choices

Fang (2026), Proposition 4.2, derives a deterministic confidence-margin excess of order (h(t)=O(sqrt{tlog t})). Consequently:
- proportional windows (k_t=lceil	heta tceil) yield a vanishing normalized margin excess (O(sqrt{log t/t}));
- polynomial windows (k_t=lceil t^etaceil), (etain(1/2,1)), yield (O(t^{1/2-eta}sqrt{log t}));
- fixed windows and windows (O(sqrt t)) have a non-vanishing asymptotic efficiency loss.

The benchmark should therefore compare proportional and polynomial policies for long-running deployments. Fixed windows are useful as a finite-horizon correctness reference, not the expected long-run default.

## Current implementation

Implemented in `anytime_rolling_coverage.rs`:
- fixed, proportional and polynomial predeclared window rules;
- future first-occurrence lookup (	au_s), including not-yet-observed starts;
- two-sided stitched empirical-Bernstein margin at delta/2 using the full three-term Equation 29;
- prefix snapshots of bounded count, miss count, predictable variation and scaled margin;
- certified rolling sum C(s,t);
- next eta from Equation 40;
- prospective checks for both hypothetical outcomes and all future-window starts that can contain the next episode;
- per-window, per-outcome feasibility witnesses;
- bounded feasibility-work budget that forces abstention rather than truncation;
- sealed decision/outcome sequence and receipt checks;
- atomic update on outcome completion;
- separate rolling miss, certified upper bound and bounded-set availability.

Not implemented or demonstrated:
- a downstream conformal quantile/set builder that accepts dynamic eta and has exact level semantics;
- signed/hash-authenticated decision and outcome receipts;
- external proof review of this Rust adaptation;
- real UWB source episodes, truth qualification or physical deployment;
- formal verification of the combined running/rolling/mode-controller stack;
- calibrated PHMI, conditional per-episode guarantees or safety qualification.

Keep this research-only until exact-head CI, independent equation review, source-complete episode data and held-out rolling coverage results exist.
