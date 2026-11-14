# Aerial receiver observability v0

Seed implementation for [AIRBORNE-ENERGY-OBSERVABILITY-001 #4115](https://github.com/Luminous-Dynamics/symtropy/issues/4115).

## Ownership boundary

- #3512 owns primary-side online inductance/load identification and its estimator excitation/uncertainty.
- #3640 owns primary-side self-sensing/coupling signatures and separation from authentication/protection.
- #3458 owns optical field propagation, receiver physics and optical safety.
- #4080 owns the session state machine and consumes observability evidence; it does not infer position from received power.
- #4115 owns the independent receiver-pose identifiability benchmark and its eventual ObservabilityReceipt.

The v0 evaluator does not build an RF or optical field model, pose tracker, controller or interlock. It evaluates a finite, explicitly supplied synthetic pose/signature map only.

## Files

- `observability.schema.json`: closed JSON Schema 2020-12 fixture contract.
- `observability.NOT_RUN.template.json`: evidence-free template, not a pass.
- `observability.unique.synthetic.json`: positive finite-sample map with three in-domain poses and one out-of-domain sample. The out-of-domain sample intentionally aliases an in-domain signature and must not change the admitted map.
- `observability.ambiguous.synthetic.json`: three materially distinct in-domain poses have identical signatures and equally high received-power proxies. The result must be AMBIGUOUS; signal strength does not identify pose.
- `tools/airborne-energy-observability-validator`: independent Rust evaluator and CLI.

## Finite v0 semantics

For each pair of in-domain poses:
1. Treat positions as materially distinct when translation separation reaches the declared threshold OR attitude separation reaches its declared threshold. Yaw separation uses wrapped millidegree difference; pitch/roll use absolute differences.
2. Two observation signatures are indistinguishable when every feature differs by no more than twice its declared per-observation noise tolerance.
3. A materially distinct pose pair with indistinguishable signatures is an alias. Any alias makes the case AMBIGUOUS.
4. A pose sample that participates in no alias is counted as uniquely observable within this finite sample set.
5. Out-of-domain samples are reported but excluded from the in-domain alias analysis.

The report carries integer numerator/denominator counts; it does not turn sample count into physical volume coverage.

## Important limitations

- This is a finite-sample synthetic identifiability preflight, not a continuous pose-space proof, Fisher-information qualification, confidence-calibrated physical estimator or measured capture basin.
- The synthetic signatures are invented known-truth values. They are not a simulated 24 GHz, IPT/CPT or optical field model.
- The simple noise rule assumes per-feature bounded observation error. A later physical estimator needs calibrated stochastic/temporal noise models, synchronized ground truth, blind held-out trajectories, receiver revisions and confidence/coverage evaluation.
- The v0 modality set is RF_MICROWAVE and OPTICAL. Near-field IPT/CPT must consume upstream evidence from #3512/#3640; this crate must not duplicate those physical estimators.
- A UNIQUE_WITHIN_DECLARED_SAMPLE_SET result is not a physical ObservabilityReceipt and cannot authorize POWER_ACTIVE. It must not be wired into #4080 as an operational permit.
- The received-power proxy is intentionally ignored by the pose-identifiability decision. High power with ambiguous signatures remains AMBIGUOUS.
- No flight, energized beam test, spectrum use, optical operation, aviation approval or physical safety certification is authorized by this package.
