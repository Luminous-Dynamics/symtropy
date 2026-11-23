# Reliability-preserving evidence transaction

**Status:** standalone synthetic composition of reliability-aware planning, consume-once acquisition, payload/receipt binding and time-validity assessment. No physical sensing or navigation state mutation is performed.

## Why this integration exists

The reliability-aware planner (#7165) ranks candidate actions using an externally admitted probability interval for a fresh, valid and independently qualified observation. The transaction (#7163) then consumes the actual attempt and gates the resulting payload.

Without another composition boundary, the planner's probability evidence could be lost between action selection and evidence consumption. This module preserves the selected action's complete reliability admission record next to the exact attempt and payload/receipt/time bindings.

## Composed owners

- #7098: frozen decision-equivalence projection.
- #7145: conditional decision-relevant action assessment.
- #7082: externally computed, domain/protocol-bound acquisition reliability admission.
- #7165: reliability-adjusted Pareto action selection.
- #7138: consume-once attempt identity, outcome and unknown reconciliation.
- #7140: measurement/receipt/processing/decision/target time applicability.
- #7163: payload/receipt/source binding and independent-audit staging.
- #6900 / #6470: downstream fault inventory, posterior/protection-region audit and integrity qualification.
- Mycelix Position #435 / #6054 / #6404: clock, stream and adapter observation type owners.
- #6669: any physical native-controller mutation boundary.

## Transaction semantics

The new reliable-plan entry point:
1. Requires a nonempty reliability-aware plan with a selected action.
2. Finds the exact selected action's admitted reliability record in that plan's Pareto frontier.
3. Projects the selected conditional assessment into the common transaction interface without discarding the full reliability record.
4. Requires the attempt request to match the selected action.
5. Starts and consumes exactly one attempt in the consume-once ledger.
6. Binds the qualified outcome to the exact action ID, attempt ID, result ID, source manifest digest, payload digest and qualification-receipt digest.
7. Requires the time-validity input's result ID and source-manifest digest to match that same evidence binding.
8. Stages only a temporally applicable decision-epoch nowcast for a later independent posterior audit.

The transaction report preserves:
- probability lower and upper bounds;
- confidence-error probability;
- independent campaign-block count;
- valid-success, failed, invalid, non-success and unresolved counts;
- empirical success fraction;
- source-manifest, calibration-cohort and bound-method commitments.

It does not reduce this provenance to one scalar confidence value.

## Result and authority boundary

The result classes from #7163 remain unchanged:
- unknown -> reconciliation required; all new attempts blocked;
- failed or acquired-invalid -> unusable; no time gate;
- qualified but missing/mismatched payload/receipt/source binding -> unusable;
- qualified but stale/unsupported/indeterminate time claim -> unusable;
- historical event-time estimate -> replay only;
- future prediction -> forecast audit only;
- time-valid nowcast -> eligible only for a later independent posterior audit.

The posterior_updated and navigation_authorized flags remain false. The module does not execute the action, update the posterior, update a protection level, grant NORMAL, or invoke a robot controller.

## Synthetic test

An integrated test creates a reliability-aware selected plan, consumes the exact recommended action, and verifies that the probability interval and campaign/source/cohort/method commitments survive intact in the resulting transaction report. It also verifies time-valid nowcast staging and absence of posterior/authority mutation.

## Production limits

The attempt ledger is still in-memory and single-writer. No durable exactly-once physical execution is established. Production use requires:
- durable append-before-side-effect;
- globally unique attempt IDs with cross-writer compare-and-swap and monotonic sequence;
- crash/restart reconciliation before any retry or mutation;
- canonical plan/action/inventory/decision partition digests independently checked by their owners;
- Mycelix-qualified time correlation and stream ordering;
- independent physical source, calibration, common-mode and model-domain verification;
- downstream posterior/integrity audit and native mutation fencing where physical actions are involved.

A passing synthetic workflow establishes only the declared software composition and caller-supplied commitments. It is not physical accuracy, field acquisition reliability, empirical PHMI, safety certification or SOTA superiority.
