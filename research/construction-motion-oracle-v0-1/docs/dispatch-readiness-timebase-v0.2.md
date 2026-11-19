# Timebase-qualified construction dispatch readiness v0.2

**Evidence class:** typed software contract + synthetic fixtures.  
**Claim ceiling:** non-actuating readiness assessment only; no safety permit, reservation, actuator command, robot stopping proof or physical qualification.

## 1. Why time is part of the evidence

A geometric occupancy result is only useful while the clocks used to construct and assess it remain comparable. A clock-domain string alone does not establish that:

- the monotonic clock has not rebooted or reset;
- a PTP grandmaster or time source has not changed;
- the offset/drift mapping into the site timebase is still the same;
- the local clock has remained within its stated error bound;
- the holdover/qualification interval has not elapsed;
- a delayed or replayed snapshot is newer than the snapshot used to create readiness.

The current ROS 2 control documentation deliberately distinguishes steady/monotonic time for hardware read/write and real-time loops from ROS time used by controller updates and distributed topics. This is the right design lesson: do not silently collapse local control time, simulation time, and cross-machine/site time into one timestamp. Sources: [ROS 2 Control clock documentation](https://control.ros.org/lyrical/doc/ros2_control/controller_manager/doc/clocks.html) and [ROS 2 Control Jazzy release notes](https://control.ros.org/jazzy/doc/ros2_control/doc/release_notes.html). The controller-manager implementation distinguishes steady/monotonic time for real-time hardware loops from ROS time used by the controller's distributed interfaces.

[IEEE 1588-2019](https://standards.ieee.org/ieee/1588/6825/) defines the Precision Time Protocol for synchronizing heterogeneous networked clocks. Its standard description states sub-microsecond synchronization support and better-than-nanosecond capability in properly designed networks. That is a protocol capability, not a promise that a particular construction site's switch topology, asymmetry, grandmaster, holdover, hardware timestamping and configuration achieve that error. Our dispatch policy consumes the measured/qualified bound supplied by the clock authority, not the standard's best-case headline.

## 2. v0.2 time evidence

The \`QualifiedSiteClockSnapshot\` binds:
- \`site_clock_domain_id\`: the named site-wide timebase;
- \`epoch_sha256\`: digest identifying the monotonic clock epoch (changes after reset/reboot/reinitialization);
- \`mapping_sha256\`: digest of the offset/drift mapping into the site timebase;
- \`authority_id\` and \`source_id\`: registered clock authority and source/grandmaster identity;
- \`sample_sequence\`: strictly increasing source sample counter;
- \`current_tick\` and \`uncertainty_bound_ticks\`: the current conservative interval;
- \`valid_until_tick\`: time qualification/holdover expiry;
- \`receipt_sha256\`: digest of the canonical clock qualification receipt.

The motion envelope and occupancy assessment each bind the same clock domain, epoch and mapping. They also carry their own generation/check tick uncertainty bounds. The current clock sample must be from a registered \`ClockAuthority\`; the application must verify the canonical receipt bytes and authority signature before constructing the typed snapshot.

## 3. Conservative interval reasoning

For a tick \(t\) and absolute error bound \(u\), the reducer uses the closed interval:

\[
[t-u,\;t+u]
\]

Subtraction/addition overflow, including an interval extending below tick zero, fails closed.

Readiness is blocked where uncertainty intervals overlap an ordering or expiry boundary. Examples:
- the current sample must be certainly after envelope generation;
- the occupancy check must be certainly after envelope generation;
- the current sample must be certainly after occupancy check;
- the current uncertainty interval must lie wholly before each qualification/validity expiry.

A nominal timestamp that is inside a window is not enough if its uncertainty interval crosses the window boundary. Uncertain means indeterminate, not ready.

The receipt expiry is the minimum of envelope expiry, occupancy expiry and clock qualification expiry. Immediately before handing the receipt to a separate authorization authority, the consumer supplies a newer qualified clock sample. It must preserve the same domain, epoch, mapping, authority and source; the sequence must increase; the new uncertainty interval must be strictly later than the recorded readiness interval and remain inside all expiry bounds. A reset, remapping, stale/replayed sample or changed clock receipt invalidates the readiness receipt.

## 4. Trusted policy is out-of-band

The robot request no longer supplies the trusted occupancy-engine digest, clearance-policy digest or maximum tolerated clock uncertainty. \`DispatchReadinessPolicy\` is supplied by a trust owner outside the untrusted request and contains:
- approved occupancy engine build digest;
- required clearance policy digest;
- clock uncertainty policy digest;
- maximum absolute clock error in site ticks.

The resulting readiness receipt preserves the clock-policy digest and bound so consumption can verify that the policy has not silently changed. The application must authenticate the policy source and the registry that maps \`authority_id\` to the \`ClockAuthority\` role. The crate checks policy consistency and actor registration; it does not authenticate signatures or independently measure clock error.

## 5. Tests and known-answer corpus

The v0.2 Rust test suite adds:
- clock authority absent from the trusted actor registry;
- same domain string but wrong clock epoch;
- same domain/epoch but wrong mapping;
- excessive current clock uncertainty;
- expired holdover qualification;
- current uncertainty interval straddling expiry;
- arithmetic underflow/overflow in interval construction;
- envelope generation and occupancy-check boundaries whose uncertainty intervals overlap the current reading;
- replayed/non-increasing clock sample at receipt consumption;
- epoch reset and mapping change after readiness;
- reused clock-receipt digest;
- changed clock uncertainty policy digest at consumption;
- unregistered new clock authority and excessive uncertainty at consumption.

The schema job validates the Draft 2020-12 v0.2 request/receipt contract and both synthetic fixtures with pinned \`jsonschema==4.26.0\`. Synthetic clock hashes and authority names are placeholders, not authenticated clock evidence.

## 6. Benchmark programme

A qualified clock adapter should be evaluated independently from the readiness reducer. Freeze test conditions and report:
- absolute offset against a calibrated reference, by device and site location;
- peak/steady-state error, drift rate, holdover growth and recovery after grandmaster failover;
- path asymmetry and network congestion sensitivity;
- reboot/reset/epoch-change detection latency;
- clock mapping revision detection;
- sequence gaps, stale samples, replayed receipts and source substitution;
- uncertainty-bound calibration coverage (fraction of reference errors contained by the declared bound);
- false-ready rate, conservative false-hold rate and readiness rejection reason distribution;
- readiness check and revalidation p50/p95/p99 latency.

Suggested goals, not current results:
- zero false-ready outcomes across a frozen 100,000-case generated boundary/replay/reset matrix;
- zero acceptance when the realized reference error exceeds the declared bound;
- 100% detection of injected epoch resets, mapping changes, stale sequence and expired holdover;
- clock-bound coverage at or above the preregistered 99.9% level with an independent calibrated reference;
- p95 software readiness/revalidation overhead below 2 ms at 10,000 candidate-pair receipt scale on the named reference host (measure end-to-end parsing and signature verification separately);
- report false holds and interval inflation as first-class costs; never loosen the bound merely to improve availability.

The 100,000-case test target is a software stress target, not a substitute for long-duration time-transfer/holdover measurements. Field evidence must name clock hardware, firmware, grandmaster, switch topology, timestamping mode, network load, calibration, traceability, environmental range, holdover and uncertainty budget.

## 7. Interoperability profile

The adapter boundary should support:
- IEEE 1588 PTP as one possible network synchronization transport;
- monotonic hardware/control-loop clock for local real-time control;
- a site timebase mapping for multi-robot/workcell schedule and occupancy;
- source-specific clock receipts from total stations, laser trackers, mobile robot controllers, surveying networks and independent metrology.

Do not require all robots to share a single hardware clock. Require a qualified mapping into the site timebase with explicit uncertainty and validity, or keep the clocks incomparable and do not issue a readiness result for cross-clock conflict assessment.

## 8. Security and safety boundaries

- Clock actor registration is necessary but not authentication. The caller verifies clock authority identity, signature and receipt canonicalization.
- A clock digest is not proof that the measured error is within the declared bound.
- PTP being enabled does not prove correct grandmaster selection, path symmetry, hardware timestamping, holdover quality or UTC traceability.
- The readiness receipt is not a reservation, permit-to-work, protective stop, safety function or motion command.
- A fresh timebase does not rescue stale/missing occupancy actors, incomplete geometry, unqualified stopping bounds or an unverified sensor/measurement source.
- Unknown, contradictory, expired or out-of-bound timing keeps readiness closed.
