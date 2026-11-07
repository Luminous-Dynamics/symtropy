# Coupled-fault and interaction diagnosability benchmark v0.1

**Status:** higher-order fault-set contract with synthetic combined responses and an additive-differential screening null. No physical interaction, mine fault coverage or operational diagnosis is qualified.

## Research motivation

A conveying line can exhibit simultaneous faults: developing blockage with filter loading, restriction with high solids, blower/VFD degradation with blockage, moisture with filter loading, or a sensor fault concurrent with a physical process fault. A closed-set single-fault model can mis-isolate those states.

Wilhelm et al. (2026), *International Journal of Advanced Manufacturing Technology* 143, 2521–2544, DOI 10.1007/s00170-025-17136-9, use ontology-driven Bayesian fault models and fault chains; this supports representing causal propagation and common causes explicitly. Borg et al. (2026), *Measurement* 258A, 118992, DOI 10.1016/j.measurement.2025.118992, evaluate pneumatic fault monitoring across system complexity and fault profiles. These are adjacent evidence, not coarse hard-rock qualification.

## Paradigm shift

Represent a state as a set of independently adjudicated faults, and collect combined-state responses independently. Do not synthesize combined responses by summing singleton signatures.

Use an additive differential model only as a declared null hypothesis:

`S_null = sum(S_i) - (n - 1) * B`

where B is a measured baseline response and S_i are measured singleton responses. Conservative interval arithmetic yields the null envelope. The combined response must be independently measured on the same channel, unit, material/path/geometry domain and synchronized event window.

A disjoint combined interval produces DEVIATION_OBSERVED—not proof of physical interaction. Calibration drift, sampling bias, temporal misalignment, model-domain mismatch or an invalid additive assumption can also cause a deviation. If the calculated null is physically inadmissible (for example, entirely negative dry-solid mass flow), return INSUFFICIENT_EVIDENCE.

## v0.1 contract

The standalone crate `symtropy-coupled-fault-bench` validates:
- exact parent #4005 case and physical trial/configuration identity;
- unique fault IDs mapped to singleton states;
- process, sensor and common-cause fault layers;
- causal and mutually-exclusive groups;
- combined fault sets with onset relation and exact known order;
- faulted-channel union derived from the active sensor-fault definitions;
- a complete combined-state × channel response matrix;
- exact unit, material/path/geometry domain, synchronized event-window, source and adjudication binding;
- unavailable sensor channels carry no numeric values;
- measured claims require measured singleton and combined responses;
- pairwise comparison of combined states using #4005 interval/separation and independence-group semantics.

## Synthetic fixture

The fixture contains five fault definitions, three combined states and 21 unique response records over seven channels. It demonstrates:
- blockage + filter loading: line-pressure combined interval [45, 50] versus additive null [30, 41] → DEVIATION_OBSERVED;
- the same combination's filter differential pressure [15, 17] versus null [13, 20] → CONSISTENT_WITH_DECLARED_NULL_MODEL;
- blockage + blower fault: the gravimetric null envelope is entirely negative → INSUFFICIENT_EVIDENCE;
- blockage + pressure-sensor fault: line pressure is degraded and carries no numeric interval → INSUFFICIENT_EVIDENCE.

The values are fabricated conformance data. No mine fault response or physical interaction is demonstrated.

## Benchmarks

Software gates:
- reject duplicate/unknown/contradictory active fault IDs;
- reject incomplete/duplicate combined response cells;
- reject faulted sensors reported healthy;
- reject unit/domain/window/source mismatches;
- do not accept model estimates as physical combined responses;
- ensure exact null interval arithmetic and physical-domain guards;
- compare combined states using independent principle groups.

Physical/model targets:
- blinded single and simultaneous fault campaigns;
- leave-one-combination-out, material-cohort-out and path-out evaluation;
- interaction detection power and false-interaction rate with uncertainty;
- pairwise mis-isolation at matched critical-event recall;
- common-cause and OOD fault campaigns;
- prospective replication before claiming hard-rock transfer.

## Integration and claim boundary

Composes with #3993/#4005 observability, #3991 metrology, #3877/#3913 event evidence, #3927 stage-resolved flow/PSD and #3917/#4027 experiment selection. This is an offline evaluator only. It does not prescribe fault injection, machine settings, pressure/airflow adjustments, blockage clearing or operational actions. Competent engineering and mine-safety authorities must approve any physical test programme.