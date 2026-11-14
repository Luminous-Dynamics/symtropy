# First-passage risk and value-of-information inspection oracle

This is a standalone synthetic exact-enumeration reference for MACH-DECISION-001 (#2977). It is a decision-math test fixture, not a wear predictor, prognostic model, production controller or maintenance authority.

## Paradigm

Instead of converting one point wear/RUL estimate into a fixed threshold, carry a discrete posterior over latent wear states and calculate the probability of reaching the end-of-life state within a declared future cutting horizon. Under a cost model, compare continuing and replacing. Then calculate whether a noisy inspection is worth its cost by enumerating every possible inspection reading, updating the posterior with Bayes' rule, and choosing the downstream action.

The reference includes:
- monotonic five-state wear transitions under NORMAL and HIGH_LOAD regimes;
- a stochastic five-bucket inspection likelihood model;
- exact four-cycle first-passage probability from every initial wear state;
- no-inspection Bayes decision;
- mean-wear threshold baseline;
- always-inspect expected cost;
- net EVSI (expected reduction in decision loss after paying inspection cost);
- probability that inspection changes the selected action;
- clairvoyant full-state lower bound and decision regret.

## Synthetic model assumptions

Wear states 0..4 are ordinal synthetic bins; state 4 is absorbing end-of-life. Transition matrices are illustrative, not calibrated to a real tool, alloy, cutting speed, feed, depth of cut, or machine. Inspection readings have five ordered categories and a frozen noise matrix. Costs are abstract units:

- ordinary next-horizon run cost: 2;
- failure / quality escape loss: 250;
- replacement cost: 65;
- inspection cost: 8.

These values deliberately create cases in which an inspection is worthwhile, not worthwhile, or too late to change the action. They are not real factory economics or safety limits.

## Metrics

- first_passage_probability: posterior-weighted probability of reaching EOL in four transitions.
- risk_policy_action: optimal continue/replace decision using the posterior and cost model.
- mean_threshold_action: simpler fixed threshold on expected wear state (2.5).
- always_inspect_cost: inspection cost plus expected Bayes-optimal downstream action cost.
- net_evsi: no-inspection optimal cost minus always-inspect expected cost; positive means information is worth buying.
- probability_inspection_changes_action: probability that a possible inspection reading changes the no-inspection Bayes action.
- evsi_policy_action: inspect only when net EVSI is strictly positive.
- clairvoyant_lower_bound: expected cost if the true discrete state were known before acting; non-deployable lower bound.

## Build and run

    rustc --edition=2024 -D warnings --test tools/machining-decision/inspection_voi.rs -o /tmp/inspection-voi-tests
    /tmp/inspection-voi-tests

    rustc --edition=2024 -D warnings tools/machining-decision/inspection_voi.rs -o /tmp/inspection-voi
    /tmp/inspection-voi --verify
    /tmp/inspection-voi --csv

## Research ladder

1. Replace the synthetic wear-transition matrices with source-pinned, tool-held-out calibrated transition estimates from admitted datasets.
2. Treat wear measurements as noisy / interval-censored labels with a calibration model.
3. Extend the single-horizon first-passage calculation to a sequential partially observed Markov decision process.
4. Compare fixed interval, posterior mean, quantile-risk, first-passage and EVSI policies at matched quality-escape risk.
5. Evaluate on whole-tool and whole-operating-condition holdouts, then a prospective human-supervised shadow study.

## Claim ceiling

The synthetic exact-enumeration tests validate the mathematics of Bayesian inspection value and first-passage risk only. They do not establish prediction accuracy, real tool-life distributions, factory cost savings, quality safety, machine control or production benefit.
