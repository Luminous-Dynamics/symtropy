# Distribution-aware manufacturing conformity reference

This standalone Rust reference calculates a conservative conformity-probability enclosure from an explicitly supplied post-measurement posterior represented as a piecewise-uniform histogram.

It does not infer a PDF from expanded uncertainty and it does not authorize product release.

## Why this is separate from the guard-band path

The companion PWR-MFG-CONF-001 implementation uses an uncertainty-expanded interval to apply a conservative guard-band rule.

JCGM 106:2012 distinguishes that coverage-interval summary from the post-measurement PDF of the true measurand. For a coverage interval wholly inside a tolerance interval, the interval can provide a lower bound on conformity probability equal to its coverage probability. If it crosses a specification limit, the interval alone is insufficient to determine the probability without the PDF. The guide then describes guarded acceptance/rejection and consumer/producer risk.

This reference adds the PDF path. It must receive a qualified posterior with explicit lineage. It must not manufacture a Gaussian posterior from U=k*u.

## Posterior model

A posterior consists of contiguous finite bins [a_i,b_i], with probability masses p_i. Each bin is uniform within its boundaries. An explicit unknown tail-mass upper bound tau accounts for posterior probability outside the represented support.

Validation requires:

    sum(p_i) + tau = 1

For tolerance interval C=[LSL,USL], represented in-spec probability is:

    p_model = sum_i p_i * max(0,min(b_i,USL)-max(a_i,LSL))/(b_i-a_i)

Because the unknown tail could be wholly inside or wholly outside the tolerance, the evaluator reports:

    p_c_lower = p_model
    p_c_upper = min(1,p_model+tau)

There is no silent renormalization of bins and no assumption that unknown tail mass is conforming.

## Approved decision policy

An external policy supplies:
- minimum accept conformance probability;
- maximum reject conformance probability;
- owner, revision, effective interval, approval evidence and digest.

The evaluator classifies:
- ACCEPT if p_c_lower >= p_accept.
- REJECT_NONCONFORMING if p_c_upper <= p_reject.
- INDETERMINATE otherwise.
- EVIDENCE_INCOMPLETE for missing, stale, revoked, mismatched or invalid evidence.

For an ACCEPT, conservative conditional consumer-risk upper bound is 1-p_c_lower. For a REJECT, conservative conditional producer-risk upper bound is p_c_upper. These are per-item posterior bounds, not global lot/process risks.

## Frozen fixtures

- Skewed asymmetric posterior: p_c=[0.95,0.95], ACCEPT at p_accept=0.95.
- Multimodal posterior: p_c=[0.50,0.50], INDETERMINATE.
- Out-of-spec posterior: p_c=[0.02,0.02], REJECT at p_reject=0.05.
- Unknown-tail posterior: p_c=[0.98,1.00], INDETERMINATE.
- Uniform distribution over [-2,2] against specification [-1,2]: exact p_c=0.75.

## Build and run

    rustc --edition=2024 -D warnings --test tools/manufacturing-conformity-risk/risk.rs -o /tmp/mfg-risk-tests
    /tmp/mfg-risk-tests

    rustc --edition=2024 -D warnings tools/manufacturing-conformity-risk/risk.rs -o /tmp/mfg-risk
    /tmp/mfg-risk --verify
    /tmp/mfg-risk --csv

## Research ladder

1. Reproduce exact histogram integration with an independent high-precision oracle.
2. Add adapters for calibrated normal, skewed, mixture and empirical posterior representations without changing the risk/decision contract.
3. Audit posterior calibration and coverage by held-out instrument, operator, station, material and process regime.
4. Integrate risk-aware test selection using Symthaea's correlated/non-Gaussian metrology and decision-aware EVSI work.
5. Only then consider lot-level risk and process capability, with independent lot/campaign statistical units.

## Claim and authority boundary

The frozen fixtures verify exact integration and bounded decision semantics for a supplied piecewise-uniform posterior. They do not validate that a posterior is calibrated, establish real production risks, certify a part or lot, select approved thresholds, authorize shipment, or actuate a machine. ACCEPT is a result under the supplied posterior and approved policy, not a release authorization.
