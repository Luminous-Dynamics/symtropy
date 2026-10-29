# QSIM-002 — Calibrated geometry-to-likelihood bridge

**Issue:** [Symtropy #2912](https://github.com/Luminous-Dynamics/symtropy/issues/2912)  
**Parent:** [QSIM-001 #2866 / PR #2867](https://github.com/Luminous-Dynamics/symtropy/pull/2867)  
**Optical forward model:** [VIS-OPTICS-001 #1807 / PR #1808](https://github.com/Luminous-Dynamics/symtropy/pull/1808)  
**Downstream adequacy:** [Symthaea QARCH-007 / PR #9023](https://github.com/Luminous-Dynamics/symthaea/pull/9023)  
**Downstream decision benchmark:** [Symthaea QARCH-009 / PR #9061](https://github.com/Luminous-Dynamics/symthaea/pull/9061)

## Contract thesis

A geometry twin is not a photonic likelihood model. QSIM-001's analytic fixture moves a detector support by 1 mm and checks the Euclidean separation; that geometric result alone does not determine transmission probability, phase, coupling, detector yield, or gate-success probability.

QSIM-002 adds a narrow, versioned boundary: a qualified optical model and calibration record may provide categorical likelihoods to Symthaea only when the model/configuration identity, valid operating domain, and evidence split are all explicit. Missing, stale, or out-of-domain prerequisites produce a structured failure status and **must omit `outcome_likelihoods` altogether**. A fake numerical estimate is more dangerous than a clear refusal.

## Files

- Schema: `docs/research/quantum/schemas/QSIM-002-LIKELIHOOD-BRIDGE.schema.json`
- Frozen adapter fixtures: `docs/research/quantum/fixtures/QSIM-002-likelihood-bridge.json`
- Predictive-adequacy known answer: `docs/research/quantum/fixtures/QSIM-002-predictive-adequacy-example.json`

The schema structurally requires the calibration artifacts, source provenance, optical model scope, explicit operating domain and split identifiers before a result can claim `QUALIFIED_LIKELIHOOD`. The verifier must still enforce semantic invariants JSON Schema cannot express by itself: probability rows sum to one within a declared tolerance; candidate, hypothesis and outcome labels/order agree with the frozen workload; calibration and evaluation split identifiers are disjoint; configuration fingerprints match; artifact digests resolve; and the requested operating point lies inside the model's validated domain.

## Frozen cases

### 1. Geometry-only input is refused

`QSIM-002-GEOMETRY-ONLY-REJECT-001` reuses the current QSIM-001 analytic geometry fixture (source blob SHA-1 `99f4bee9002dc7d5e31dbcec7ffb734071c84a57`). The expected output is `NOT_CALIBRATED`, with no likelihood matrix. The independent geometry oracle remains Euclidean distance:

- nominal separation: 0.2 m
- perturbed separation: 0.20000249998437522 m
- change: 0.0000024999843752093742 m

These are mathematical geometry values only.

### 2. Stale calibration is refused

`QSIM-002-STALE-CALIBRATION-REJECT-001` is a synthetic negative control. The model requests configuration v2 while the only calibration artifact applies to v1. The expected status is `STALE_CALIBRATION`; any output containing likelihood values violates the fixture's claim ceiling.

### 3. Predictive adequacy uses a separate held-out interface

`QSIM-002-SYNTHETIC-HOLDOUT-ADEQUACY-001` reproduces a known-answer categorical example with prediction [0.5, 0.5] and counts [50, 50]. Under the explicitly declared IID-categorical fixture assumption, expected values are TV = 0, Brier = 0.5, mean log loss = 0.6931471805599453 nats, and the 95% finite-sample TV upper bound is 0.13581015157406193. It is a synthetic checker example, is marked `deployment_allowed = false`, and cannot qualify an optical model. The production predictive-adequacy implementation remains the owner of this score rather than duplicating its code in Symtropy.

## Evidence and calibration rules

1. **Analytic geometry:** validates coordinate arithmetic only.
2. **Uncalibrated simulation:** may explore scenarios, but does not produce qualified likelihoods.
3. **Calibration-derived estimate:** binds a qualified optical model, versioned configuration, calibration samples, uncertainty and validity domain.
4. **Held-out predictive adequacy:** binds a distinct evaluation artifact and tests predicted outcome distributions before they drive decisions.
5. **Measured physical:** additionally requires traceable instrument/session/device identity and raw evidence.

Never split repeated measurements randomly at the row level when they share a session, instrument setup or configuration. Split by the sources of dependence relevant to the claimed generalization (for example device/configuration, session or operating regime). If IID categorical assumptions are not justified, report scores but do not present IID concentration bounds as valid coverage.

## Acceptance gates

- Geometry-only input and stale-calibration controls return their expected statuses and contain no likelihood matrix.
- Wrong coordinate units/frames, changed source digest, mismatched hardware/configuration fingerprint, out-of-domain wavelength, missing hypothesis/outcome identity, malformed probability rows and non-finite values fail closed.
- Any qualified result has resolved model/calibration/raw-evidence digests, distinct calibration and evaluation partitions, interval-bearing estimates and explicit validity bounds.
- Held-out QARCH-007 status and evidence class remain separate from Symtropy simulation status.
- QARCH-009 policy runs report regret, no-measurement utility, unsupported outcome mass and measurement cost separately; synthetic fixtures never masquerade as measured hardware.
- Exact-head format/tests and the focused workflow must pass before implementation correctness is claimed.

## Industry comparison and proposed metrics

Compare on common tasks and metrics, not on a private “quantum score.” QOBLIB's 2026 model-independent benchmark library emphasizes reproducible instances and strong classical solver comparisons, while Open QBench spans hybrid, high-level and compiled stacks and multiple quantum paradigms. Those suites benchmark optimization/system performance; QSIM-002 instead qualifies the *physical-model-to-likelihood boundary* required to interpret an experimental result. This is complementary, not a replacement.

Proposed gates (not achieved results): 100% agreement on analytic known-answer fixtures; zero false accepts on stale/mismatched calibration and invalid probability contracts; held-out predictive scores reported with uncertainty and data provenance; and no claim of physical probability quality until independent calibrated traces are available. A future hardware comparison must report time, energy/resource use, accepted-output quality, calibration residuals, and comparison to the strongest applicable classical baseline on the same workload.

## Current status

These are schemas, fixtures and research contracts. The known-answer and negative-control artifacts are synthetic/analytic. They are not an implementation of optical physics, not a physical calibration data set, and not evidence of quantum advantage.
