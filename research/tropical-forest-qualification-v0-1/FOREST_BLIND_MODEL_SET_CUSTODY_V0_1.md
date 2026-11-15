# Sealed Forest Model-Set Adequacy Challenge v0.1

**Tracking:** [FOREST-BLIND-ADEQUACY-001 #5328](https://github.com/Luminous-Dynamics/symtropy/issues/5328)  
**Canonical authority:** [Symtropy #2334](https://github.com/Luminous-Dynamics/symtropy/issues/2334)  
**Forest development owner:** [#5198](https://github.com/Luminous-Dynamics/symtropy/issues/5198)  
**Status:** public protocol and schemas only; no independent custodian appointed and no blind challenge run

## 1. Why the public corpus is insufficient

The existing 30-variant public model-set corpus is useful for development and conformance. It cannot demonstrate blind omitted-mechanism detection because its case families and expected dispositions are public. The current small Rust kernel is a deterministic known-truth diagnostic over cohort states, not a general scientific model-set adequacy detector.

A public hash does not make a public generator blind. A secret file in the same candidate repository is not independent custody. The sealed test truth and generator must remain outside the candidate's code, permissions, runtime and review path.

## 2. Objective and threat model

The challenge should estimate how well a submitted system detects *preregistered classes* of model-set inadequacy and avoids false alarms. It does not prove universal model completeness.

Threats include:
- hidden truth or generator leakage through source, filenames, case order, seeds, metadata, issue text or output timing;
- candidate memorization of public synthetic fixtures;
- post-hoc thresholds and selective reporting of successful runs;
- evaluator/candidate modifying the challenge after submission;
- shared credentials that let one actor alter both hidden truth and the final receipt;
- uneven compute, model calls, observation access or experiment budgets;
- failed and timed-out cases omitted from the score;
- rights-restricted observations copied into public artifacts;
- repeated benchmark rounds that allow adaptation to the reserve holdout.

## 3. Roles and separation of duties

| Role | Authority | Must not control |
|---|---|---|
| Benchmark owner | Public protocol, schema, metrics, challenge interface and resource rules | Secret truth or custodian signing key |
| Independent custodian | Hidden generator, seed, truth vault, commitment opening and family rotation | Candidate source or score thresholds after freeze |
| Candidate team | Frozen candidate model registry and immutable responses | Hidden generator, truth, evaluator internals |
| Independent evaluator | Executes candidate against sealed cases and signs execution receipt | Secret opening material unless needed for authorized adjudication |
| Auditor | Verifies custody, receipts, commitment openings and non-compensatory gates | Candidate modifications after submission |

A named independent custodian and evaluator are prerequisites. Neither has been appointed in this repository state. Do not represent an internally generated public fixture as sealed.

## 4. Commitment and reveal

Use a domain-separated, salted SHA-256 commitment over a canonical manifest. Canonicalize JSON using RFC 8785 (or another frozen canonical encoding explicitly versioned by the protocol). Keep the random 256-bit nonce and sealed bundle in custodian-controlled storage.

Commitment preimage fields:
- challenge round and protocol version;
- generator bundle digest and generator revision;
- secret seed commitment;
- case manifest digest;
- truth bundle digest;
- scoring/evaluator configuration digest;
- rights/access policy reference;
- secret nonce.

The published commitment should be:

SHA256("LUMINOUS-FOREST-BLIND-CHALLENGE-V1" || 0x00 || nonce || canonical_manifest_bytes)

The nonce must be generated from a cryptographically secure random source, kept secret until opening, and not reused across rounds. The custodian should independently sign the commitment and timestamp it before accepting candidate submissions. A plain hash without a nonce may permit guessing of low-entropy hidden scenarios.

At round close, the custodian reveals the nonce and canonical manifest, or provides an independent auditor a verifiable opening and publishes a redacted attestation if source rights or sensitive ecological data prevent public disclosure. A commitment proves that a committed byte sequence was not changed after commitment; it does not prove that the generator is scientifically valid or that custody was independent.

## 5. Challenge lifecycle

1. Freeze protocol, candidate interface, output schema, metric implementation, training-data cutoff, resource budget, stopping rule and evaluator build. Publish their digests.
2. Custodian generates hidden cases from preregistered families using secret parameters/seeds. Store generator, seeds and truth outside this repository and candidate execution environment.
3. Custodian creates a salted commitment and signs it before issuing the round's challenge inputs.
4. Candidate receives only permitted historical observations, rights-qualified metadata, opaque case tokens, target definitions and allowed experiment catalogue. No hidden family labels, future truth, secret seeds or generator features.
5. Candidate produces one immutable submission containing source/model/data digests, per-model distributions, adequacy status, abstention reason and proposed measurements/actions.
6. Independent evaluator verifies digest, signature, schema, data access, time and resource parity before running every assigned case. No missing run is silently omitted.
7. Evaluator records success, abstention, error, timeout and resource exhaustion for every opaque case token and signs the receipt.
8. Custodian/auditor verifies the commitment opening after the round. Publish family-level scores and permissible evidence; retain a separately sealed reserve holdout.
9. Rotate seeds and case construction details for later rounds. Do not expose the reserve set or adapt thresholds after seeing a candidate submission.

## 6. Challenge families

The public protocol names families, but the sealed case-to-family mapping and exact generator details remain hidden:
- common-mode omitted process with aggregate historical-fit aliasing;
- delayed/hysteretic recovery and mortality;
- seed/propagule limitation despite adequate canopy light;
- spatial canopy-gap topology and support mismatch;
- unchanged latent ecology with shifted observation/census process;
- correlated EO products with common upstream ancestry;
- disturbance × drought or climate interaction absent from all candidates;
- common sensor/campaign ancestry and calibration drift;
- parameter uncertainty or within-model stochasticity that should not trigger structural alarms;
- genuinely adequate model sets under noise, sparse support and missingness;
- low-cost and high-cost experiments with different expected decision value.

The hidden generator should include counterfactual pairs: same latent ecological trajectory with a changed observation process; and same observation process with a changed latent ecological mechanism. This tests whether the candidate attributes the change to the correct layer.

## 7. Submission, scoring and non-compensatory gates

Candidate response schema: schemas/forest-blind-candidate-submission-v0.1.schema.json  
Evaluator receipt schema: schemas/forest-blind-evaluation-receipt-v0.1.schema.json

Report per family and regime:
- omitted-mechanism recall and false-adequacy rate;
- false alarm on adequate-in-set cases;
- abstention precision/recall and selective risk–coverage;
- CRPS, WIS, Brier and coverage where appropriate;
- calibrated interval width and local/worst-stratum coverage;
- model disagreement preserved by state/horizon/site;
- decision regret and experiment discrimination per cost;
- runtime, peak memory, energy, network/data access and wall-clock budget;
- rights violations, digest mismatch, hidden-truth leakage and protocol deviations.

No single weighted score may compensate for a critical false-adequacy event, truth leakage, rights violation, omitted failed run or resource-budget mismatch. The clairvoyant mechanism-aware policy is an upper-bound diagnostic, never a leaderboard contestant.

## 8. Research and benchmark positioning

The public model-set corpus remains the development suite. The sealed challenge measures generalization to unseen members of declared mechanism families. Report both; never merge public and sealed scores.

The benchmark should compare candidate systems against:
- each registered process model;
- ensemble mean/median and model averaging;
- robust/minimax action under the registered model set;
- residual/predictive adequacy diagnostics;
- an abstention-capable model-set challenger;
- random, information-gain and decision-value measurement selection;
- a clairvoyant upper bound for interpretation only.

## 9. Acceptance gates and claim ceiling

- [ ] Independent custodian and evaluator named, with access controls documented.
- [ ] Hidden generator/truth are outside candidate repository, CI artifacts and candidate runtime.
- [ ] Protocol/evaluator/data/resource digests are frozen before submissions.
- [ ] Salted commitment is signed and timestamped before the round starts.
- [ ] Candidate submission is immutable and digest-bound.
- [ ] All assigned cases and failed runs are receipted.
- [ ] At least one reserve holdout remains sealed after the first public round.
- [ ] Rights-constrained data is evaluated without unauthorized copying or derivative release.
- [ ] Public development fixtures and sealed benchmark results remain separately labelled.

A successful round measures performance on its sealed, preregistered mechanism families and custody process. It does not prove universal model adequacy or eliminate unknown unknowns. No blind challenge is currently operational.
