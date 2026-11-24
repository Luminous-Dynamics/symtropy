# Joint persistent model + common-cause policy tree v0.1

**Status:** isolated offline exact Bayesian-mixture reference. Composes the common-cause track (#7272 / #6708) and persistent model-mixture track (#7273 / #7251). No production sensing or navigation authority is exposed.

## Paradigm shift

Do not treat sensor modality, sensor model, or failure domain as independent labels. The same observation can be affected by the true navigation/service condition, a shared common-cause regime, and a persistent uncertain observation/acquisition model.

The joint hidden state is Z=(X,C,M): navigation condition X, common-cause regime C, and model identity M. Each is binary. The eight state order is model 0/1 × nominal/shared-fault × inside/outside. Each action supplies a complete three-outcome likelihood law for all eight states. The same model identity and common-cause regime persist across the horizon. Outcomes factorize only conditional on the complete joint state; marginally they can be dependent.

## Implemented contract

The Rust module navigation_joint_latent_policy_tree.rs provides:
- exact eight-element PPB joint prior;
- complete action-specific likelihood row for each hidden state;
- exact posterior for navigation state, model identity and common-cause regime;
- explicit state/model/fault-dependent no-observation;
- one shared action per observable branch, never one per hidden-state value;
- exact non-anticipative horizon-two planning and matched greedy-first comparator;
- asymmetric false-safe/false-degraded loss;
- hard latency, energy and bandwidth ceilings;
- deterministic ordering, checked u128 arithmetic, no action reuse, abstention and non-authority marker.

This is a Bayesian mixture over one frozen joint prior, not a robust mission-global worst-case policy. Model and fault regime are latent and persistent; observations update their posterior.

## Synthetic regression fixtures

The tests include posterior updates for model and fault regimes and a persistent model-dependence fixture. With two equally likely persistent models and per-sensor outcome-1 probabilities 0.1 and 0.9, the joint probability of two outcome-1 reports is 0.41 while the product-of-marginals approximation is 0.25. Another test shows no-observation can update both model and common-cause posteriors.

These are exact constructed probabilities, not calibrated physical failure rates.

## Tests authored

Ten tests cover: model/fault posterior updates; joint no-observation updates; induced dependence; exact-vs-greedy loss; candidate-order invariance; resource budgets and non-reuse; malformed input; asymmetric loss; abstention; and one policy per observable history. Exact-head CI is the qualification gate.

## Limitations

The v0.1 model has two states each for navigation condition, model identity and common-cause regime. It assumes one persistent model and one persistent common-cause regime, and conditional independence given the full joint state. It does not calibrate priors/likelihoods, discover real dependency groups, model residual dependence after conditioning, allow model/regime transitions, handle continuous pose or delayed observations, establish integrity/PHMI or physical safety, or implement global minimax robustness.

The global robust objective is separate:
min over observable policy trees pi, max over fixed mission model m, E_m[L | pi].
It must evaluate one shared observable tree under each fixed model and take the maximum after whole-tree evaluation. A branchwise maximum is a different rectangular adversary.

## Benchmark comparators

Freeze the same joint prior, outcome tables, action inventory, resource ceilings and terminal losses. Compare:
- independence approximation that discards common-cause and model latents;
- common-cause-only policy (#7272);
- model-mixture-only policy (#7273);
- combined joint-latent policy (this module);
- one-step greedy;
- exact horizon-two oracle;
- stop/no sensing.

Report held-out joint scenario log/Brier score, false-safe/false-degraded exposure, decision regret, model/fault posterior calibration, resource/energy/airtime, abstention and ranking sensitivity.

## Research and empirical anchors

- Li, Kuhn & Sutter (2026), non-rectangular robust MDPs: https://doi.org/10.1137/24M1631250
- Kumar et al. (NeurIPS 2025), normed uncertainty sets: https://papers.nips.cc/paper_files/paper/2025/hash/372cb7805eaccb2b7eed641271a30eec-Abstract-Conference.html
- UWB critical survey (23 Nov 2026): https://doi.org/10.1186/s13638-026-02703-9
- MILUV dynamic UAV dataset: https://pmc.ncbi.nlm.nih.gov/articles/PMC13552925/
- HYMN static reference-point benchmark: https://doi.org/10.1109/ieeedata.2026.3691044
- TWR-CLOUD-Bern v6: https://zenodo.org/records/23434390
- ISO/IEC 21134:2026 benchmark framework: https://www.iso.org/standard/86651.html
