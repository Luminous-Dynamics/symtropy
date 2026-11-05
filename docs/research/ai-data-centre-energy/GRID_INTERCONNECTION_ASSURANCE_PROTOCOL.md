# Grid-Interconnection Ride-Through and Recovery Assurance Protocol

**Protocol:** GRID-INTERCONNECTION-ASSURANCE-001  
**Version:** 0.1  
**Status:** contract-driven PCC waveform verifier; not grid-code certification

## 1. Objective

Verify that a data-centre connection remains within an externally supplied voltage/frequency ride-through envelope during a defined disturbance and that active/reactive power returns to its contracted operating point within a defined recovery deadline and settling window.

This is distinct from dispatch delivery (#3649): a dispatch response asks the data centre to change power; an interconnection disturbance is imposed by the grid and tests the site's ability to remain connected and recover without destabilizing behavior.

## 2. Standards context

IEEE P4200 is listed by IEEE SA as an active standards project / PAR for electric transmission and distribution system interconnection functional, performance and interoperability requirements for data centres and associated components. The project page identifies ride-through, post-fault active-power recovery timing, protection coordination, oscillation and ramp-rate management, power quality, monitoring and study-model documentation. It is not a published final standard. [IEEE P4200 project](https://standards.ieee.org/ieee/4200/12632/).

Use P4200's stated scope as a requirements watchlist only. Do not claim P4200 compliance until its normative text is final, applicable grid-code/interconnection provisions are mapped, and an independent test-lab/operator process is completed.

## 3. Contract and evidence

The Rust verifier requires:
- event/site/PCC identity;
- interconnection-agreement ID and caller-supplied envelope version;
- piecewise voltage/frequency ride-through bands with explicit elapsed-time bounds;
- pre-disturbance active and reactive power baseline;
- P/Q recovery tolerances;
- active/reactive recovery ramp ceilings;
- recovery deadline and stable settling window;
- maximum sample gap and measurement time-uncertainty bound;
- time-stamped PCC voltage pu, frequency Hz, active kW, reactive kvar, breaker state, quality classification and evidence ID.

The caller supplies all envelope values. The module contains no jurisdiction-specific default voltage/frequency thresholds, no universal ride-through curve and no assumption that IEEE P4200 is already normative.

## 4. Verification gates

1. **Structural completeness:** samples begin at event start, timestamps are strictly increasing, sample gaps are within the contract, and samples do not exceed the declared evidence horizon.
2. **Measurement quality:** every sample is Valid and clock uncertainty is no greater than the configured limit.
3. **Ride-through:** each sample in a ride-through time band remains within the corresponding voltage/frequency bounds.
4. **Connection continuity:** the breaker/connection flag remains closed through the observed trace.
5. **Recovery ramp:** observed interval-to-interval active and reactive power ramps after fault clearance remain within separately supplied limits.
6. **Settling:** P/Q must return to the baseline tolerance envelope no later than the recovery deadline and remain there continuously for the full settling duration. Later excursions invalidate an earlier candidate recovery time.

The evidence horizon extends through recovery deadline plus settling duration so that the final in-deadline candidate has enough subsequent samples to prove stability. A late recovery cannot be accepted merely because later samples eventually reach the baseline.

## 5. Limitations

This verifier does not calculate or certify:
- transient voltage/frequency stability, fault current or electromagnetic transients;
- relay/protection coordination, reclosing, anti-islanding or protection-setting correctness;
- harmonic distortion, flicker, voltage unbalance or power quality beyond sampled voltage/frequency;
- UPS/STS/generator transfer internals, inverter control stability or OEM certification;
- thermal/workload consequence beyond the connected breaker and P/Q samples;
- system-wide grid stability or N-1 system security.

Ramp is measured only at the supplied sampling resolution. Sub-sample transients are always marked unverified. Production use requires traceable meter calibration, synchronization, uncertainty accounting, correct PCC location and a laboratory/operator test plan.

## 6. Benchmark programme

Proposed engineering targets, not external standards:
- 100% agreement with synthetic known-truth voltage/frequency, breaker, ramp and recovery fixtures.
- Zero false passes for out-of-band ride-through, breaker trip, ramp violation, late recovery, oscillation or invalid measurement quality.
- Zero acceptance when the sample gap/clock uncertainty cannot resolve the declared event bands.
- Independent replay reproduces every gate from frozen contract and sample evidence.
- Measure recovery-time and P/Q ramp distributions over repeated tests; report uncertainty, not best-case only.
- Fault profiles include voltage dips/swells, frequency excursions, fault clearance, and phase/frequency events only when the applicable operator profile requires them.
- Include UPS transition, generator start/transfer, static switch, cooling transient and representative IT load interactions at the PCC.
- Validate with power-hardware-in-the-loop or utility grid simulators and independent meter chains before field certification.

## 7. Integration

- #3649: dispatch latency, ramp and sustain delivery.
- #3652: repeated-event response reliability.
- #3673: service-equivalent workload flexibility and rebound.
- #3675: battery/thermal terminal-state reconciliation.
- #3694: owner-controlled dispatch authorization; no external OT actuator.
- #3667/#3687: chronological adequacy and PRAS comparison.

This assurance layer reports how the PCC behaved. It must not expose protection or facility actuators to a market agent.
