# Motion- and Time-Aware Acquisition Integrity Contract v0.1

**Status:** research specification; synthetic contract only  
**Parent:** [Symtropy #6845](https://github.com/Luminous-Dynamics/symtropy/issues/6845)  
**Work item:** [ORBITAL-ACQUISITION-INTEGRITY-001 / #7350](https://github.com/Luminous-Dynamics/symtropy/issues/7350)  
**Related:** [#6925](https://github.com/Luminous-Dynamics/symtropy/pull/6925), [#7136](https://github.com/Luminous-Dynamics/symtropy/issues/7136), [#7164](https://github.com/Luminous-Dynamics/symtropy/issues/7164), [#7222](https://github.com/Luminous-Dynamics/symtropy/issues/7222), [#7339](https://github.com/Luminous-Dynamics/symtropy/issues/7339), [#4671](https://github.com/Luminous-Dynamics/symtropy/issues/4671), [#5631](https://github.com/Luminous-Dynamics/symtropy/issues/5631)

## 1. Why a view must become a space-time measurement

A geometric view is commonly represented by a camera pose and target geometry. An actual image is integrated over a time interval. For a moving camera, moving target, or both, there is no single exact camera pose for the whole exposure. Rolling-shutter sensors add row-dependent exposure times. Structured-light systems add a projector pattern and synchronization sequence that must correspond to the captured frame.

The missing bridge is:

geometry candidate -> physical acquisition interval -> admitted frame -> downstream measurement gates

This contract qualifies only whether the supplied motion and timing evidence keeps acquisition inside a declared image-space budget. It does not infer image usability, feature measurability, calibrated accuracy or defect detection.

## 2. Four distinct temporal failure mechanisms

Do not collapse these into one image-quality scalar.

1. **Exposure smear:** target/camera relative motion during a pixel's exposure integration interval.
2. **Rolling-shutter distortion:** different sensor rows correspond to different exposure times and therefore different camera/target poses.
3. **Clock and timestamp misassociation:** camera exposure is paired with the wrong robot state, transform, target state or illumination event.
4. **Projector-pattern misassociation:** the camera observes a different, transitioning or ambiguous structured-light pattern than the frame metadata claims.

A fifth state, **model insufficiency**, applies when the available motion/time evidence cannot bound one of the four mechanisms. Insufficient evidence is INDETERMINATE, not a pass.

A global shutter only removes row-time offset. It does not remove motion during a long exposure. A short exposure does not remove rolling-shutter row-time distortion. Post-hoc image correction does not prove that the underlying measurement information was recoverable.

## 3. Input contract

Every acquisition record must bind exact IDs and revisions for:

### Camera and imaging mode

- Camera/sensor serial and model.
- Global or rolling shutter.
- Sensor mode, resolution, crop/window, frame rate and pixel-coordinate convention.
- Exposure duration and whether it is fixed, per-frame or per-region.
- Timestamp event: trigger reception, exposure start, exposure midpoint, exposure end, readout start/end or host arrival.
- Rolling-shutter readout direction and line delay / total readout time where applicable.
- Lens/intrinsic calibration reference and the matching #7339 calibration-validity receipt.

Unknown shutter type, timestamp reference, exposure interval, row direction or required readout parameter must not receive a convenient default.

### Motion state

- Robot ID/configuration and exact encoder/joint-state source.
- Camera/robot transform and kinematic calibration revision.
- Target/asset frame and its motion source or explicit static-target evidence assumption.
- Time-stamped joint/pose observations bracketing exposure and, for rolling shutter, all required row-time intervals.
- Motion interpolation/model ID, operating envelope and model validity receipt.
- Uncertainty/bounds for pose, velocity, acceleration and timing as required by the chosen model.

Do not interpolate across missing/stale samples, an unmodelled discontinuity, calibration invalidation or out-of-domain motion. Host receive time is not a substitute for acquisition time.

### Clock and trigger state

- Camera, robot, trigger, projector and reference-clock IDs.
- Time-map/offset/drift receipt, validity interval and bounded uncertainty.
- Trigger-to-exposure latency and jitter.
- Sequence ID, monotonically ordered event IDs, duplicate/drop/reorder disposition.
- Explicit separation of event time, acquisition time, readout time, transport time and host-ingest time.

Use the exact time-map contract owned by #5631 where available. Do not create a second generic clock synchronization service.

### Structured-light state (when applicable)

- Projector ID, pattern sequence/revision and each pattern ID.
- Pattern transition/stabilization interval.
- Camera exposure interval for the corresponding frame.
- Hardware/software synchronization method and timing evidence.
- Explicit proof that the full camera integration interval maps to the intended pattern state, including clock uncertainty.

Passive-camera cases must mark projector timing as not applicable by declared sensor type, not by silently omitting a required field.

### Task-specific error budget

Declare the maximum permissible acquisition-time image-space error from the intended feature/measurand and downstream measurement profile. Examples include maximum smear in pixels, maximum row-time reprojection difference and maximum feature-space displacement. The limit must carry units, owner/source, profile revision and applicability. Do not invent one universal pixel or micrometre limit.

## 4. Conservative model

The v0.1 evaluator should support a bounded model before any statistical model.

Let X(t) be a target point in the target frame, T_wc(t) the camera pose in world coordinates, and pi(.) the admitted camera projection function. A camera-space point is:

x_c(t) = T_cw(t) X_w(t)

and its image coordinate is:

u(t) = pi(x_c(t)).

All time arguments are intervals or carry explicit bounded uncertainty. The evaluator uses the worst admissible time/state within declared bounds, not only a nominal midpoint.

### Exposure smear

For a global-shutter exposure interval E = [t_s, t_e], use a declared reference time t_r (normally the midpoint only where timestamp mapping establishes it). For feature sample set F, a conservative geometric smear bound is:

B_smear = sup over X in F and t in E of ||u_X(t) - u_X(t_r)||_2.

For rolling shutter, each row has a row-specific integration interval E(r); compute per-row exposure smear from that interval. The supremum includes camera and target motion uncertainty and timestamp uncertainty.

If an analytic bound is unavailable, use a declared interval enclosure or dense deterministic simulation as a reference comparator; do not label a finite sample grid as a proof of a continuous-time supremum.

### Rolling-shutter row-time distortion

Define t_r as the exposure reference for the declared reference row. For each image row y, compute its exposure reference time using the admitted readout model:

t(y) = t_0 + direction * y * line_delay

with exact row indexing, origin, direction and line-delay units frozen in the schema. Then compute:

B_row = sup over X in F and y in rows of ||u_X(t(y)) - u_X(t_ref)||_2.

This is a temporal projection difference, not exposure smear. Report the reference row and the row/time at which the maximum occurs. Include clock and readout uncertainty in the bound.

The implementation may later add a continuous-time rolling-shutter optimization or correction model. v0.1 qualifies raw acquisition bounds; it must not assume corrected images are equivalent to a global-shutter image.

### Timestamp association

A frame is eligible only when:
- timestamp event semantics are known;
- clock map is valid for the entire event interval;
- robot/target motion samples or model enclose relevant exposure and row-time intervals;
- clock uncertainty plus trigger jitter does not push the motion/image-space bound beyond the task budget.

A frame timestamp outside its declared source clock or a stale time-map receipt is INDETERMINATE or INVALID_INPUT, not a numeric zero-offset assumption.

### Projector-pattern association

For every structured-light frame, require one unambiguous pattern ID whose stable projection interval contains the entire exposure interval after including clock and trigger uncertainty. Any overlap with a pattern transition or a second pattern yields PROJECTOR_PATTERN_MISMATCH or INDETERMINATE. Pattern sequence gaps, duplicates and out-of-order frames remain visible in the receipt.

## 5. Outcomes and receipt

Suggested top-level status values:

- ACQUISITION_VALID_WITHIN_DECLARED_BUDGET
- MOTION_BLUR_BUDGET_EXCEEDED
- ROLLING_SHUTTER_BUDGET_EXCEEDED
- TIMING_UNCERTAINTY_EXCEEDED
- PROJECTOR_PATTERN_MISMATCH
- OUT_OF_MOTION_MODEL_DOMAIN
- INDETERMINATE
- INVALID_INPUT

A frame can fail more than one channel. Preserve the full reason-code set and all raw bounds.

The receipt should include:
- acquisition/frame/sequence and all source identities;
- camera model, shutter type and exact exposure/readout configuration;
- timestamp semantic and clock-map/trigger receipt identities;
- motion model, encoder sample IDs, target-state IDs and calibration-validity receipt;
- per-channel numeric bounds, budgets, units, worst-case feature sample and worst-case time/row;
- event completeness, missing/delayed/duplicated event inventory;
- all reason codes and final deterministic status;
- exact canonical input digest, schema/benchmark versions and evidence class;
- explicit claim ceiling and non-authorization fields.

Keep VALID state distinct from later photometric, sampling-density, feature-measurability, metrological uncertainty and decision states.

## 6. Frozen P0 challenge corpus

At minimum:

1. Static target and static camera with a known global-shutter exposure.
2. Camera translation during exposure, just below and just above the smear budget.
3. Camera rotation during exposure, checking angular and image-space displacement separately.
4. Target motion with a stationary camera.
5. Global shutter with long exposure: smear must still be detected.
6. Rolling shutter with short exposure: row-time distortion must still be detected.
7. Reverse rolling-shutter readout direction; verify time ordering and worst row.
8. Trigger, exposure-start, midpoint, end, readout-complete and host-arrival timestamp declarations; reject mismatched semantics.
9. Camera/robot clock offset and drift inside/outside the uncertainty budget.
10. Trigger latency and jitter at the exact budget boundary.
11. Valid bracketing encoder samples versus stale/missing/gapped samples.
12. Motion discontinuity or acceleration/jerk outside the declared model domain.
13. Calibration changes between planning and acquisition; consume #7339 and reject stale evidence.
14. Projector pattern changes during exposure.
15. Dropped, duplicated, reordered or delayed camera/projector trigger events.
16. Multi-exposure/HDR sequences with mismatched pattern IDs.
17. Missing shutter/readout/exposure/clock/motion parameters.
18. Time-origin shifts, input-order permutations and frame reparameterizations that preserve the physical event sequence.

P1 stateful sequences should inject robot acceleration, vibration, flex, payload/tool changes, encoder packet loss, clock drift, buffering, projector latency, strobe illumination timing and target motion. Keep sensor acquisition, transport and host-arrival timelines separate.

## 7. Independent verification and benchmark ladder

### D0 — deterministic arithmetic

- Independent Rust implementation and standard-library Python oracle.
- Hand-computed known answers for constant-velocity linear motion, constant angular motion, timestamp-boundary cases and projector pattern windows.
- Exact canonical receipt equality where fields are integer/rational; declared outward-rounded tolerances for unavoidable trigonometric bounds.
- Input schema, unit, frame, timestamp, row-index, duplicate-ID and event-completeness mutations.
- Metamorphic time-origin shifts, row-direction reversal, frame transforms and input-order permutation.

### D1 — temporal reference simulator

Compare conservative bounds to a denser, independently implemented continuous-time reference simulator over frozen motion trajectories. Report bound conservatism and missed excursions separately. Dense sampling is not proof of a continuous-time bound.

### D2 — matched acquisition policies

Use identical trajectories, target features, image dimensions, exposure budgets, lighting and motion schedules to compare:
- static-pose planner;
- constant-velocity / constant-angular-rate smear constraint;
- global-shutter constraint;
- rolling-shutter-aware row-time constraint;
- timestamp/trigger uncertainty-aware candidate screening;
- joint exposure/trajectory/illumination candidate screening.

Report planned path length, speed, exposure setting, lighting/strobe cost, valid acquired-frame count, invalid-frame count, rescan burden and characteristic-level measurement quality. Never compare one policy's selected exposure or speed against another policy without matching resource/illumination envelope.

### D3 — physical acquisition pilot

Use a traceable moving target/artefact, calibrated motion stage or robot trajectory, hardware trigger instrumentation, camera with known shutter/readout model and independent time reference. Pre-register acquisition procedure, motion regimes, exposure settings, uncertainty budget, ground truth, holdout trajectories and acceptance criteria. Include global- and rolling-shutter sensors when available. For structured light, independently observe projector pattern state and camera exposure.

Physical testing is not required to complete the synthetic contract. No physical motion-capture accuracy is inferred from synthetic results.

### Report metrics separately

- false-valid and false-hold;
- exposure smear (pixels and feature-space units);
- rolling-shutter row-time distortion;
- camera/robot/projector clock uncertainty;
- trigger latency and jitter;
- motion-model residual and out-of-domain rate;
- projector-pattern association failures;
- frame acceptance fraction, rescan/time/energy burden;
- downstream feature measurement error and uncertainty only after owning calibrated measurement gates execute;
- runtime, memory and deterministic replay.

A zero false-valid count on a finite synthetic corpus is a contract result, not a physical reliability rate.

## 8. Research and industrial context

- Zhang et al., [Rolling Shutter Camera Self-Calibration](https://arxiv.org/abs/2608.01509) (2 August 2026), estimates camera intrinsics and rolling-shutter readout/time behavior using continuous-time trajectory geometry. This establishes that camera geometry and row-time are a coupled calibration problem; it does not provide a universal camera timing profile.
- Axis, [Global shutter vs rolling shutter](https://whitepapers.axis.com/en-us/global-shutter-vs-rolling-shutter) (August 2026), distinguishes exposure-duration motion blur from row-offset rolling-shutter distortion. It is an industrial explanatory source, not a metrology standard.
- [Specifying and optimizing robotic motion for visual quality inspection](https://doi.org/10.1016/j.rcim.2021.102200), Robotics and Computer-Integrated Manufacturing, DOI 10.1016/j.rcim.2021.102200, studies robot motion as an image-quality variable and notes blur from high-speed motion.
- [FPGA Based Adaptive Rate and Manifold Pattern Projection for Structured Light 3D Camera System](https://www.mdpi.com/1424-8220/18/4/1139) describes camera/projector synchronization and how synchronization affects point-cloud quality. Its specific hardware and implementation are not assumed by this contract.

## 9. Claim ceiling

PASS means only that supplied acquisition timing and motion evidence satisfies a declared synthetic image-space budget. It does not establish camera calibration, image quality in an uncontrolled environment, feature measurability, metrological accuracy, defect detection, NDE/POD, structural acceptance, certification or flight readiness. Physical claims require calibrated instruments, traceable timing and motion references, held-out trajectories and independent adjudication.
