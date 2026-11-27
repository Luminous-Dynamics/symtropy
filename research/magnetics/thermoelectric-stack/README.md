# AM Thermo-Electrical Winding and Cooling Stack Benchmark

**Program:** MAG-AM-THERMO-ELECTRICAL-STACK-001  
**Issue:** #7824  
**Version:** 0.1  
**Status:** research protocol and evaluator prototype; no physical superiority claim

## Thesis

Evaluate AM-enabled windings and integrated cooling as a complete thermo-electrical-hydraulic-dielectric stack. The benchmark does not reward geometric complexity or a hotspot reduction in isolation. It asks whether the stack delivers a defensible improvement against optimized conventional winding/cooling references under the same duty, waveform, coolant, thermal boundary, and acceptance policy.

A thermal gain is not an advantage if purchased with higher AC copper loss, dielectric weakness, excess pressure drop or pump input, leakage/corrosion risk, poor thermal-cycle life, low accepted-part yield, or higher total cost.

## Comparator families

- **ConventionalWindingCooling:** optimized conventional winding and cooling; mandatory reference.
- **ConventionalHighPerformanceCooling:** second conventional comparator where applicable.
- **AmIntegratedWindingCooling:** AM-enabled winding and/or integrated cooling candidate.
- **HybridThermalSubassembly:** AM thermal hardware with conventional conductor/insulation.
- **ReferenceControl:** secondary context only; cannot replace the conventional baseline.

## Evidence and decisions

The evaluator requires M4 component acceptance and physical observations with method, calibration, uncertainty, and immutable evidence digests. Simulation and coupons may guide experiments but cannot establish stack superiority. M5 production qualification is separate.

Hard limits are non-compensatory. Candidate intervals must satisfy every absolute requirement and be conservatively non-inferior to every mandatory comparator. An advantage requires strict uncertainty-separated improvement in at least one same benefit metric against every required comparator. There is no universal weighted score.

See `MAG_AM_THERMOELECTRICAL_STACK_BENCHMARK_2026-11-27.md`, `benchmark-plan.toml`, and `evaluator/`.

## Industry baseline requirement

“Conventional” must include the strongest applicable incumbent, not just a water jacket. The evaluator has distinct families for direct oil cooling, multi-nozzle oil jets, hairpin oil spray, slot-embedded cooling and hollow conductors. The application must preregister which families are mandatory. A water-jacket-only result cannot support an industry-superiority claim where direct cooling is already used in the target segment.
