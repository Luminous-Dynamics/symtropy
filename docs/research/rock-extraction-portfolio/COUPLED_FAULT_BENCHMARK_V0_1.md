# Coupled-fault and interaction diagnosability benchmark v0.1

**Status:** higher-order fault-set contract with synthetic combined responses and an additive-differential screening null. No physical interaction, mine fault coverage or operational diagnosis is qualified.

## 1. Why isolated fault pairs are not enough

A real conveying path may contain multiple simultaneous faults:
- developing blockage with filter/cyclone loading;
- partial restriction with high solids loading;
- blower/VFD degradation with restriction;
- physical restriction with pressure-tap fault;
- material moisture with filter loading;
- common power, clock or data-acquisition failure affecting multiple channels.

The combined response need not equal a sum of singleton signatures. Interacting physical processes, causal propagation, sensor faults and common-cause dependencies can make a closed-set single-fault classifier mis-isolate the actual state.

## 2. Research basis

- Borg, Refalo & Francalanza (2026), *Measurement* 258A, 118992, DOI 10.1016/j.measurement.2025.118992, evaluate pneumatic fault monitoring across system-complexity and fault-profile levels. This is compressed-air evidence, not mine-scale coarse-hard-rock combined-fault qualification.
  https://doi.org/10.1016/j.measurement.2025.118992
- Wilhelm et al. (23 February 2026), *International Journal of Advanced Manufacturing Technology* 143, 2521–2544, DOI 10.1007/s00170-025-17136-9, construct ontology-driven Bayesian fault models. Their discussion of failure-mode independence alongside causal fault propagation is a reason to explicitly represent co-occurrence, causal chains and common causes rather than assume independence by default.
  https://link.springer.com/article/10.1007/s00170-025-17136-9
- Kościelny & Sztyber-Betley (2025), *Mechanical Systems and Signal Processing* 236, 112979, DOI 10.1016/j.ymssp.2025.112979, motivate pairwise fault isolability as a diagnostic-system property.
  https://doi.org/10.1016/j.ymssp.2025.112979

## 3. Paradigm shift: fault sets and interaction screens

The benchmark separates:
- the *fault set* (what faults were independently adjudicated active);
- the *singleton observations* (baseline and each fault measured alone);
- the *combined observation* (measured separately with exact channel, domain, time and source identity);
- the *null model* (an explicitly declared additive differential screening hypothesis);
- the *interaction status* (overlap, deviation, or insufficient evidence);
- pairwise diagnosability between combined states.

The combined observation is never created by adding singleton records. It must be independently recorded. A model cannot validate itself.

## 4. Additive differential screening null

For a baseline response interval B and singleton intervals S1…Sn, define the additive differential null:

`S_null = sum(S_i) - (n - 1) * B`

This is equivalent to baseline plus the sum of singleton deviations from baseline. Conservative interval arithmetic gives:

`null_lower = sum(singleton_lower_i) - (n - 1) * baseline_upper`

`null_upper = sum(singleton_upper_i) - (n - 1) * baseline_lower`

The combined response interval is compared with the null interval:
- overlap => CONSISTENT_WITH_DECLARED_NULL_MODEL;
- disjoint => DEVIATION_OBSERVED;
- missing/degraded/unqualified response, or unavailable singleton needed for the null => INSUFFICIENT_EVIDENCE.

**Interpretation:** a deviation is not proof of a physical interaction. It can also result from material/path mismatch, nonlinear measurement behaviour, calibration drift, sampling bias, residence-time misalignment or an invalid additive assumption. The null is a screening comparator, not a physical law.

If the additive null interval is outside a declared non-negative measurand domain (for example, entirely negative dry-solid mass flow), the evaluator returns INSUFFICIENT_EVIDENCE rather than a spurious interaction claim.

The interval arithmetic propagates declared bounds; it does not assign probabilistic joint coverage or estimate covariance. The underlying calibration and uncertainty authorities remain #3991.

## 5. v0.1 executable contract

Standalone crate: `symtropy-coupled-fault-bench`.

### Fault catalog
Each fault declares:
- unique fault ID;
- exact singleton state ID in #4005;
- layer (PROCESS, SENSOR, COMMON_CAUSE);
- causal group;
- optional mutually-exclusive group;
- affected channel IDs;
- source reference.

### Combined-state event
Each combined state declares:
- at least two unique known active fault IDs;
- combination type (co-occurrence, causal chain, common cause);
- onset relation (simultaneous, known order, unknown order);
- exact order when KNOWN_ORDER;
- exact union of affected sensor channels;
- evidence class, event window and independent adjudication reference.

Mutually exclusive faults cannot be active together. A causal chain requires a known onset order in v0.1.

### Combined response matrix
Every combined state × parent sensor channel requires exactly one response record with:
- availability and a numeric interval only when VALID;
- channel unit and exact #4005 material/path/geometry domain;
- synchronized event-window identity;
- unique source and adjudication references;
- explicit evidence role.

A channel declared faulted by an active sensor fault must be DEGRADED or MISSING. A model estimate cannot establish a physical combined response.

### Combined-state pairwise diagnosis
Required combined-state pairs use #4005 interval-separation semantics. Channels in the same independence-principle group count once. Known unavailable channels due to a declared sensor fault are not counted as healthy evidence; unexpected missing/unknown signals make the pair INSUFFICIENT_EVIDENCE.

## 6. Synthetic challenge fixture

The fixture has:
- 5 fault definitions;
- 3 combined states;
- 7 inherited sensor channels;
- 21 unique combined-state response records;
- 3 required combined-state comparisons.

Structural replay demonstrates:
- blockage + filter loading: synthetic line pressure [45, 50] versus additive null [30, 41] => DEVIATION_OBSERVED;
- the same combination's filter differential-pressure [15, 17] overlaps its null [13, 20] => CONSISTENT_WITH_DECLARED_NULL_MODEL;
- blockage + blower fault: gravimetric additive null is entirely negative => INSUFFICIENT_EVIDENCE, not an interaction;
- blockage + pressure-sensor fault: line pressure is degraded and carries no numeric value => INSUFFICIENT_EVIDENCE for that channel;
- pairwise combined-state signatures can still be compared using remaining independent modalities.

All interval values are fabricated. These results validate only the software's interval arithmetic, fault-set semantics and evidence admission.

## 7. Benchmark gates

Software:
- 100% rejection of duplicate/unknown fault IDs, contradictory exclusive groups, unknown singleton states and incomplete matrices;
- 100% rejection of a declared faulted sensor channel reported VALID;
- every measured combined response has exact domain/window/source/adjudication provenance;
- no combined response may be generated from singleton values;
- additive interval bounds match golden vectors;
- null-model domain violations produce INSUFFICIENT_EVIDENCE;
- pairwise combination diagnosis de-duplicates independent-principle groups;
- synthetic evidence remains SYNTHETIC_ONLY.

Physical/model:
- blinded independently adjudicated single and simultaneous fault campaigns;
- leave-one-combination-out, leave-one-material-cohort-out and leave-one-path-out testing;
- interaction detection power and false-interaction rate with uncertainty;
- pairwise combined-state mis-isolation at matched critical-event recall;
- OOD and common-cause failure evaluation;
- uncertainty-aware null-model residuals and prospective replication.

## 8. Integration

- #3993/#4005: singleton state taxonomy, sensor channels, response intervals and independence groups.
- #3991: calibration, sample/timebase and uncertainty evidence.
- #3877/#3913: independently adjudicated event records.
- #3927: stage-resolved mass/PSD context.
- #3917/#4027: EVSI for additional observations.
- #4026: this higher-order fault-set and interaction benchmark.

## 9. Claim boundary

A DEVIATION_OBSERVED result is a departure from a declared additive screening null, not proof of a causal physical interaction. No unsafe fault injection, operating setting or machine action is prescribed. Physical tests require competent engineering and mine-safety approval. Measured interaction claims require independent calibrated baseline, singleton and combined-state observations on the same declared material/path domain.
