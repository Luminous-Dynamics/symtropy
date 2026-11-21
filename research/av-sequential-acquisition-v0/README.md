# AV Sequential Acquisition Benchmark v0

**Issue:** #6675  
**Decision-valued observation owner:** Symtropy #6222 / PR #6225  
**Risk-constrained EVSI:** Symtropy #6045  
**Probability-free measurement selection:** Symtropy #5774  
**Replay and temporal eligibility:** Symtropy #6201 / #6254  
**Counterfactual modality value:** Symtropy #6305 / PR #6307  
**Long-horizon evidence:** Symtropy #6357 / PR #6359

## Objective

Compare finite audiovisual acquisition plans by their effect on the eventual decision, including acquisition cost, delay, permission and resource constraints. v0 uses small fixed serial plans and an exact non-adaptive action-set oracle; it does not yet support branching policy trees that choose the next sensor conditionally on prior outcomes.

## Why this matters

Recent work shows adaptive access can be useful, but its ablations also contain important negative controls:
- Video Active Perception reports up to 5.6x frame efficiency on long-form video QA: https://arxiv.org/abs/2605.01662
- SEW-Bench reports 75–94% episodic retrieval under a 4,096-token budget versus 3–19% for equal-budget query-independent sampling, but its full architecture does not beat a verbatim-only baseline and two preregistered falsification criteria fire: https://arxiv.org/abs/2608.22975
- TRACE reports 1,240 records / 517 videos and argues for explicit evidence timing, response timing, false alarms, workload and completion: https://arxiv.org/abs/2609.30670
- SPOT-Bench evaluates every stream prediction and has a posted 1Hz Ctrl-SPOT T-F1@5 baseline of 20.9: https://dibschat.github.io/SPOT-Bench/
- DIVE reports sub-second average latency alongside average response-window rates no higher than 53.77%: https://arxiv.org/abs/2610.28405

These scores are not directly comparable. The lesson is to measure the decision value of each acquired observation and its resource/deadline cost, and to preregister controls that can falsify an adaptive approach.

## v0 scenario set

1. **EIG trap:** the high-information subtype observation does not change the decision; a lower-EIG audio hazard cue does.
2. **Sequential complementarity:** neither sensor alone resolves an XOR-like decision, but both together do.
3. **Deadline inversion:** a higher-quality visual action arrives after the decision deadline; a fast audio cue is usable.
4. **Permission fallback:** a high-value camera action is not authorized; the policy must use an authorized audio alternative.

All media are represented by synthetic outcome likelihoods and placeholder resource units. There are no raw PCM/RGB assets, physical sensors or measured wall-clock/energy readings.

## Protocol

The candidate receives only the public workload, including priors, decision-loss tables, action outcome likelihoods, permissions, deadlines and resource budgets. The hidden truth file contains one realized hidden state and realized outcome for each possible action; it must never be mounted into the candidate process.

A candidate submits a serial action trace. Each request begins when the previous acquisition arrives, and each declared arrival must equal request tick plus the action duration. The evaluator verifies the outcome against hidden realization, applies Bayes' rule, and requires the final decision to minimize posterior expected loss.

The exact oracle enumerates all feasible non-adaptive action subsets from the finite action catalogue. It minimizes expected terminal decision loss plus acquisition cost under hard time, permission and resource constraints. This is an exact v0 comparator, not a branching sequential policy oracle.

## Scorecard

Report expected information gain (bits), gross EVSI, net EVSI, expected objective loss, regret against the exact finite action-set oracle, realized decision loss, action mix and resource use independently. A higher EIG score is not automatically a better policy. Hard permission and budget constraints cannot be traded against answer quality.

## Proposed targets (research goals, not achieved)

- Reproduce all finite known answers and exact oracle action sets.
- Never acquire a denied sensor or exceed a time/resource cap.
- Avoid an EIG-only action when it has zero decision value and positive cost.
- Select complementary observations when their joint net EVSI is positive.
- Prefer a lower-fidelity measurement when the higher-quality sensor arrives too late.
- At matched realized decision loss, target at least 20% fewer measured acquisition resources than acquire-all.
- Next, extend to branching acquisition policies and compare greedy EIG, greedy EVSI, risk-constrained EVSI and non-myopic finite-horizon planning under multiple seeds.

## Claim ceiling

The finite synthetic evaluator does not prove production active sensing, real-world SOTA, physical sensor value, or end-to-end latency/energy improvements. The initial branch qualifies outcome likelihoods, finite-plan arithmetic, action timing and permission/budget gates only.