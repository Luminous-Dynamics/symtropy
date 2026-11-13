# F1 — Time-indexed membrane observability fault traces v0.1

**Issue:** [#5059](https://github.com/Luminous-Dynamics/symtropy/issues/5059)  
**Parent F0:** [#5056](https://github.com/Luminous-Dynamics/symtropy/issues/5056)  
**Status:** nine synthetic eight-step traces (72 time windows); no physical process data.

## Purpose

Static F0 scenarios test diagnostic cause separation. F1 adds temporal onset, progression, persistence and clearance so event delay and false alarms can be measured. These are not estimates of real-plant lead time or false-alarm rates.

## Trace set

Nine traces at 5-minute sampling:
- stable negative control;
- gradual normalized-resistance growth;
- pressure-sensor drift only;
- temperature regime ramp;
- pump-speed regime transition;
- intermittent critical-channel dropout;
- resistance growth plus pressure drift;
- salinity shift without normalized resistance growth;
- pressure drift that clears.

Inputs, thresholds and expected dispositions are separate from the oracle. Each time window retains its own measured/derived features; no future window is required by the baseline rule.

## Metrics

- Event detection delay from declared physical-event onset.
- Detection delay distribution in steps and minutes.
- False fouling windows during sensor-only and regime-only traces.
- False alerts normalized by trace exposure.
- Multiple-cause preservation.
- Recovery after injected sensor fault clears.
- Abstention under unobservable input.
- Zero CIP recommendations (hard advisory-only boundary).

All windows from a trace must stay in one split. Hold out complete trace IDs, fault families, severity ranges and generator seeds. Random-window splitting is forbidden.

## Nonclaims

Synthetic 72-window replay is not plant-level event detection, lead time, false alarm rate, generalization or cleaning performance.
