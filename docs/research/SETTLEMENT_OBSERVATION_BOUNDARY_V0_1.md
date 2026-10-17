# Settlement Observation Boundary v0.1

**Status:** Research contract, tracked by [SETTLEMENT-OBS-001 #1753](https://github.com/Luminous-Dynamics/symtropy/issues/1753). This is a prerequisite for policy evaluation, not an implementation claim about current v1 behavior.

## Why this is a separate layer

A settlement-policy benchmark must distinguish the simulated world from what the decision-maker knows about it. An evaluator may need synthetic latent state to calculate the outcome of a scenario; a policy must only receive the observation records that would have arrived by its decision time.

The current v1 kernel uses `ComponentState::Unknown` as a conservative state-estimate contract. This prevents an unknown supply from being treated as healthy or zero-loss, but it is not yet a complete physical truth + sensor + policy-knowledge model. We must preserve that limitation and version the stronger semantics explicitly.

## Three independent state spaces

### 1. Latent scenario state (simulator-owned)

The artificial state stipulated by a frozen scenario, including exogenous faults and recovery. Used to calculate synthetic outcomes and to determine when an injected event actually happened. It is not passed into the policy information API. It must never be presented as measured evidence about the real world.

### 2. Observation stream (sensor/report-owned)

A sequence of immutable source observations with target identity/generation, original sample time, received/available tick, units, provenance, validity/currentness and uncertainty. Observation records can be delayed, missing, stale, conflicting, unvalidated or revoked.

A telemetry fault changes the observation stream; it does not automatically change the latent component's physical state. A physical fault changes the latent state and may or may not be observed immediately.

### 3. Policy information state (decision-owned)

The policy gets only records available at its current decision tick and any memory it derived from previously available records and its own past actions. Every derived memory update retains its input lineage. Hidden latent state, future events, final report values and oracle outputs are not policy inputs.

## Causal evaluation flow

```text
versioned latent initial state + exogenous events
                  |
                  v
       latent scenario evolution
                  |
             +----+----+
             |         |
             v         v
       synthetic      observation
       outcomes       generator + delay/fault model
                         |
                         v
               policy-visible frame at tick t
                         |
                         v
                   policy decision
                         |
                         v
          intervention validation / resource cost / latency
                         |
                         v
         accepted action changes future latent evolution
```

This separates *what happened*, *what became knowable*, *what was decided*, and *what the intervention actually changed*. A future event must not affect a policy decision until that event has changed the available observation history.

## Proposed versioned contracts

- **`LatentComponentStateV1`**: component ID/generation, synthetic operational state, event time, validity interval, exogenous cause reference and explicit synthetic claim ceiling.
- **`ObservationRecordV1`**: sensor/report ID/generation, target ID/generation, original sample time, available/received tick, observed value or explicit unavailable/conflicting status, units, currentness, source provenance, transformation lineage and uncertainty.
- **`ObservationScheduleV1`**: per-source latency, dropout/staleness/conflict injection, validation status, expiry rules and deterministic seed.
- **`PolicyInformationFrameV1`**: decision tick, only observations available so far, source currentness and policy-local memory lineage.
- **`OutcomeFrameV1`**: simulator-side latent state and realized service/resource/floor trajectories, never directly injected into the policy frame.
- **`DecisionReceiptV1`**: policy version, decision tick, exact information-frame digest, action request, trigger/justification, constraints evaluated and result (accepted/rejected/deferred).
- **`InterventionReceiptV1`**: action execution start/completion/abort, costs/resources/crew, latency, resulting event references and future-state causal lineage.

Unknown and unavailable states must be typed; never encode missingness as zero, success or the last known value without an explicit staleness marker.

## Information-boundary negative controls

1. **No-lookahead:** create two scenarios with the same observation prefix through tick (t), but different later latent events. The policy's decisions through (t) must be byte-identical.
2. **Stale sample:** keep the latent state fixed but delay delivery of an older measurement. The observation's sample time remains old and the policy input exposes its age/currentness.
3. **Missing sample:** omit a scheduled observation; the policy gets an explicit missing disposition.
4. **Conflicting sources:** preserve both records and their provenance rather than choosing the most favorable reading automatically.
5. **Latency change:** change delivery time only; do not rewrite original sample time or event identity.
6. **Information parity:** each paired baseline consumes the identical observation schedule unless information quality is the experimental variable being studied.
7. **Oracle leakage:** a policy implementation that reads latent state, future event lists or the final report fails conformance.
8. **Policy memory lineage:** any belief/cache/state used at tick (t) traces only to observations/actions available no later than (t).
9. **Generation mismatch:** stale sensor/component generations fail closed or are explicitly classified as unusable.
10. **Reproducibility:** same scenario and observation schedule yield identical information frames and decision receipts.

## Metrics

- Detection delay from latent event onset to first qualifying observation.
- False alarms and missed detections under explicitly defined synthetic event truth.
- Action latency from observation arrival to decision and then to accepted/executed intervention.
- Essential-service loss accumulated before detection and during intervention delay.
- Comparison under perfect, delayed, missing, stale and conflicting observations as separate arms.
- Information value only when the experiment isolates information from action budgets and other differences; never invent event probabilities.
- Zero future-information leakage and 100% decision/action/input lineage over the frozen conformance suite.
- All results report the synthetic observation-generation assumptions and do not imply real-world sensor performance.

## Connection to Symthaea and Mycelix

- Symtropy owns the deterministic synthetic timeline and outcome computation.
- Symthaea receives only `PolicyInformationFrameV1` plus its own traceable memory, then proposes decisions with uncertainty and abstention.
- Mycelix supplies references to approved context/floors/authorities and observation provenance where qualified; it does not turn a simulation into a measurement or grant actuation authority.
- Independent verifier checks lineage and no-lookahead receipts; the policy under test does not verify itself.

## Current boundary and implementation order

The current synthetic kernel is static + discrete-event. It has event records, per-tick outputs and an independent internal-consistency verifier, but it does not yet implement separate latent-state storage, a sensor latency/dropout generator, a policy-visible observation API, action execution/costing, or a no-lookahead paired-policy runner.

Recommended order:
1. Qualify the current v1 ledger and receipt checker.
2. Version the latent-state/observation contracts without silently changing v1 meaning.
3. Implement deterministic observation schedules and frozen positive/negative controls.
4. Add a policy conformance API exposing only observation frames and policy-local memory.
5. Add decision/intervention receipts, action resource costs and latency.
6. Run paired transparent baselines with common exogenous events and identical information.
7. Only then evaluate Symthaea-generated policies and external physical models.

## Research connection

NIST's Community Resilience Planning Guide calls for understanding infrastructure dependencies and cascading consequences during recovery: https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.1190v2.pdf

The 2025 systematic review of urban digital twins found a gap between technical ambitions and realized contribution because socio-technical integration, consensual contextualization and procedural operationalization are often underaddressed: https://doi.org/10.3390/smartcities8010032

The 2026 DAPP-MD framework makes uncertainty multi-dimensional and explicitly includes interacting risks, which reinforces the need to isolate observation uncertainty from hazard, demand, model-form and governance uncertainty: https://doi.org/10.1016/j.envsci.2026.104382

These sources motivate the evaluation contract; they do not validate the prototype or supply its synthetic parameters.


## Initial code slice now present

The settlement-core branch now includes `src/observation.rs` with:
- typed observation values and explicit Current, Stale, Missing, Conflicting, Unvalidated and Revoked dispositions;
- source/target identities and generations, sample tick, availability tick and a declared freshness window;
- canonical schedule ordering, duplicate-ID and impossible-timestamp rejection;
- a `PolicyInformationFrame` builder that includes only records available by the requested decision tick;
- current-to-stale conversion when a once-current reading ages beyond its freshness window;
- a frame digest and independent checks against future observation leakage and tampering;
- eight authored Rust tests including a no-lookahead metamorphic test: two schedules with the same available prefix but different future values produce the same current decision frame.

This is an **observation projection/conformance contract**, not yet a full latent-state simulator or sensor generator. The policy frame deliberately contains no latent-state or future-event-list field, but end-to-end policy decisions and interventions are still future work. All eight tests are authored and unverified until exact-head CI completes.


## Observation-bounded baseline decision API

The branch now also includes `src/policy.rs` with transparent `NoIntervention` and `ReactiveRepair` baselines. The policy evaluator receives only a verified `PolicyInformationFrame` and a versioned policy definition—never the observation schedule, latent state, future events or timeline outcome receipt.

A repair request is emitted only when the frame contains a current failed-component observation from a source explicitly allowed by the matching rule. The request declares target generation, triggering observation ID/digest, decision/dispatch/completion ticks, parts and crew requirements, priority and deterministic action ID. The decision is hashed and checked against the exact frame and policy rule.

This proves the **shape and information boundary of a request**, not successful maintenance. No repair is executed; no parts or crew are actually reserved; no active-action journal yet suppresses duplicate requests across ticks; and no safety/municipal authority grants execution permission. Those remain required before policy requests can be compared as realized interventions.
