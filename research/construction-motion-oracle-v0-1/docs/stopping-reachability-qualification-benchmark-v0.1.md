# Construction Stopping-Reachability Qualification Benchmark v0.1

**Status:** preregistered research protocol. The current Rust kernel has synthetic mathematical tests only. No robot braking test, sensor detection test, HIL run, field qualification or safety certification is claimed.

## 1. Why this is a separate physical assurance problem

A path planner can show that a trajectory is collision-free at nominal speed and still have insufficient space to stop after a hazard is detected. A detector can advertise a maximum sensing range and still fail to detect a particular obstacle early enough, or a robot can have a good nominal deceleration while performing poorly on a slope, wet surface, high payload, low battery or worn brake.

We therefore separate three questions:

1. **Detection coverage:** was the relevant hazard detectable, in the relevant direction and environment, with a qualified lower-bound range?
2. **Stop reachability after trigger:** after the declared detection/stop trigger, how far can the complete robot continue to move during sensing/control/actuator latency and braking?
3. **End-to-end distance budget:** does the qualified minimum detection range exceed the stop reachability bound plus independent uncertainty and policy margins?

The v0.1 code evaluates question 2 from externally qualified dynamics bounds and compares question 1's lower-bound range with question 2 plus explicit margins. It does not establish that the detector actually sees all relevant hazards, that all uncertainty terms have been captured, or that the robot is safe to authorize.

## 2. Conditional dynamics model

For initial speed upper bound \(v_0\), maximum trigger-to-braking latency \(T\), and maximum positive acceleration \(a_+\) during that latency:

\[
d_{\text{latency}} = v_0T + \frac{1}{2}a_+T^2
\]
\[
v_1 = v_0 + a_+T
\]

After the guaranteed braking profile begins, deceleration magnitude must grow no slower than the qualified minimum jerk \(J_{\min}\), until it reaches the qualified minimum *net* deceleration \(A_{\min}\), then remain at least \(A_{\min}\) until stopped.

The time to reach the minimum deceleration plateau is:

\[
t_r = \frac{A_{\min}}{J_{\min}}, \qquad
\Delta v_r = \frac{A_{\min}^2}{2J_{\min}}
\]

If \(v_1 \leq \Delta v_r\), the stop is triangular and ends before reaching the plateau:

\[
t_s = \sqrt{\frac{2v_1}{J_{\min}}}, \qquad
d_b = v_1t_s - \frac{J_{\min}t_s^3}{6}
\]

Otherwise, the profile ramps to \(A_{\min}\), then holds that lower bound:

\[
d_r = v_1t_r - \frac{J_{\min}t_r^3}{6}
\]
\[
v_2 = v_1-\Delta v_r, \qquad
d_p = \frac{v_2^2}{2A_{\min}}, \qquad
t_p = \frac{v_2}{A_{\min}}
\]
\[
d_b = d_r+d_p, \qquad t_b=t_r+t_p
\]

The physical longitudinal stop distance is:

\[
d_{\text{stop}} = d_{\text{latency}}+d_b
\]

The kernel adds the externally qualified distance guard and outward-rounded numerical guard. The exact qualification must prove that the jerk and net-deceleration lower bounds hold over the frozen operating domain. A nominal deceleration number is not enough.

## 3. End-to-end hazard distance budget

The companion evaluator checks:

\[
D_{\text{detect,min}} \geq
D_{\text{stop,bound}}+
U_{\text{localization}}+
U_{\text{frame}}+
U_{\text{body geometry}}+
C_{\text{site policy}}
\]

All terms must use the same longitudinal direction, unit and declared coordinate context. \(D_{\text{detect,min}}\) is the *minimum qualified detection distance* for the stated sensor, obstacle class, pose, speed, surface, weather, lighting, reflectivity, occlusion and mounting configuration. It is not the manufacturer's maximum advertised range.

A non-negative remainder means only that the declared scalar distance budget closes. It is not named SAFE, does not grant dispatch readiness, does not replace a protective field or exclusion zone, and cannot authorize actuation. Sensor coverage/failure probability and human entry are separate evidence questions.

## 4. Physical qualification input contract

Every dynamics qualification should bind:
- robot manufacturer, serial, build, firmware and exact drive/brake configuration;
- tool, payload, carried load, centre of gravity and configuration mass;
- brake/tyre wear state, battery state, temperature and maintenance condition;
- surface material and measured condition, slope/grade and travel direction;
- qualified operating-speed range;
- trigger definition and end-to-end trigger-to-brake-onset latency measurement;
- maximum acceleration during that latency;
- minimum net deceleration and minimum braking-jerk build-up rate;
- test campaign, independent metrology, timebase, calibration and uncertainty budget;
- environmental / target condition domain and explicit exclusions;
- a shared scenario-context digest binding the stopping and detection test strata.

The qualification record must be derived from independent measurements over a declared test matrix. The runtime evaluator must reject speed, robot or condition profiles outside that domain. A profile digest is an identifier, not proof; the consumer must verify canonical evidence bytes and signatures.

## 5. Physical test matrix

Begin in a guarded, instrumented cell with no people or uncontrolled traffic in the robot's stopping area. Use independent high-speed optical tracking or a calibrated laser tracker, independent timebase, wheel/drive telemetry, brake-command and actuator-onset instrumentation. The robot's own odometry is a diagnostic channel, not the reference truth.

Cross the following conditions:
- speed: minimum, intermediate, maximum qualified speed and transition speeds;
- payload: empty, nominal, maximum, centre-of-gravity extremes and asymmetric loads;
- grade: level, maximum declared uphill and downhill grade, both directions where applicable;
- surface: dry, dusty, damp, low-friction challenge material and transitions;
- battery: high, nominal and low permitted state-of-charge;
- temperature: minimum, nominal and maximum qualified temperature;
- brake/tyre condition: new, service-limit and controlled degradation profiles where permitted;
- command path: local trigger, networked perception trigger, supervisory stop and communication loss;
- hazard: rigid obstacle, low-profile obstacle, dark/low-reflectance object, angled object, partially occluded object, edge/opening and movable temporary worksite object;
- sensor state: nominal, dirty/partially obscured, calibrated drift, time synchronization loss and sensor dropout.

Detection tests and braking tests are separate before they are combined. Do not infer obstacle detectability from stop distance. Do not allow human exposure during injected sensor or braking faults.

## 6. Benchmark metrics and proposed targets

These are research goals, not achieved results:

| Benchmark | Metric | Proposed target / gate |
|---|---|---|
| Analytic kernel | Difference from independent time-step reference | Maximum absolute distance error ≤0.05 mm over the frozen supported input grid |
| Monotonicity | More speed/latency/acceleration or less jerk/deceleration must not shorten the computed stop distance | Zero monotonicity violations in deterministic generated corpus |
| Qualification-domain enforcement | Out-of-profile speed/configuration/condition | 100% typed refusal; no extrapolation |
| Physical stop containment | Predicted integer bound vs independently measured stop distance + reference uncertainty | Zero under-bounds in a frozen challenge corpus; report one-sided confidence bound and common-cause grouping |
| Bound efficiency | Ratio of computed bound to independently measured upper-tail stop distance | Target median inflation ≤20% and p95 ≤35% after the qualification sample is sufficient; do not optimize this at the expense of containment |
| Trigger latency | Independent trigger-to-brake-onset p50/p95/p99 and worst observed | Profile's maximum must cover the declared operating domain plus metrology and sampling uncertainty |
| Sensor detection | Minimum detection distance and missed-detection rate by target class/pose/environment | No pooling across target classes; zero missed detections in the frozen qualification corpus, with confidence bounds and declared domain |
| End-to-end distance budget | Minimum qualified detection range minus stop bound and all margins | Non-negative across every declared challenge condition; any negative or unresolved condition blocks dispatch readiness |
| Repeatability | Independent lab / serial unit / operator / test-day replication | At least 3 test campaigns, 2 independently calibrated measurement chains and a second site before a transferability claim |
| Operational response | Controlled stop, protective stop, communication-loss stop, recovery and requalification behavior | Every path preserves bounded stop occupancy and refuses re-entry until independently cleared |

For zero observed misses in \(n\) independent representative Bernoulli trials, report the corresponding one-sided confidence bound; zero observed is not zero risk. Trials sharing a robot, target, sensor, site or environmental cause are clustered and must not be treated as independent samples without justification. Safety-related reliability claims require a statistically defensible campaign, not a handful of demonstrations.

## 7. Industry and standards context

The industry already has important building blocks, but product specifications often describe separate pieces of the end-to-end chain.

- Dusty Robotics' published FieldPrinter 2 specification lists obstacle sensors with approximately 0.9 m forward range, plus separate cliff sensors with finite placement/field-of-view and trigger-threshold constraints. It also describes narrow opening cases where the sensors may not detect a hazard. This is why a maximum sensing-range number cannot be treated as a verified minimum detection distance for all construction obstacles. Use the exact platform documentation and test stratum rather than generalizing the example to every robot.
- NIOSH's 7 October 2026 construction robotics bulletin identifies unauthorized entry into robot operating areas, software/wireless communication risks, sensing limitations and multi-robot coordination as emerging hazards. Braking dynamics alone do not close those hazards.
- The June 2026 systematic review of 375 construction-robotics studies finds that deployment remains constrained by worksite variability, workforce acceptance, safety accountability and limited field readiness. The September 2026 construction HRC review of 163 studies emphasizes cognitive, affective, physical and usability responses, and the need to link human factors to system-level safety and supervision.
- ISO 3691-4:2023 is the published driverless industrial truck safety/verification reference where the actual machine and system fall within its scope. ISO/DIS 3691-4 edition 3 is in development; it is not yet the published replacement. Applicability to a particular construction layout robot must be assessed rather than assumed.
- AMRA-201:2026 provides general mobile-robot requirements and test methods on solid travel surfaces. NIST's mobile-manipulator safety test-method research is a useful precedent for instrumented performance and safety testing. Neither by itself certifies this construction-specific end-to-end distance budget.

Source register:
- Dusty FieldPrinter 2 specifications: https://support.dustyrobotics.com/hc/en-us/articles/52682349645851-FieldPrinter-Specs
- NIOSH, Safety in Construction Robotics (7 Oct 2026): https://www.cdc.gov/niosh/bulletin/2026/safety-in-construction-robotics.html
- Construction robotics systematic review, 375 studies (June 2026): https://www.sciencedirect.com/science/article/pii/S2590123026014404
- Construction HRC human-factors review, 163 studies (September 2026): https://www.sciencedirect.com/science/article/pii/S0022437526001489
- ISO 3691-4:2023: https://www.iso.org/standard/83545.html
- ISO/DIS 3691-4, edition 3: https://www.iso.org/standard/88615.html
- AMRA-201:2026: https://amr-alliance.org/amra-201/
- NIST mobile-manipulator safety test methods: https://www.nist.gov/publications/test-methods-evaluation-manufacturing-mobile-manipulator-safety

## 8. Claim discipline and development ladder

1. **Analytic/software verified:** formula tests, independent time-step oracle, domain refusal and schema validation.
2. **Hardware-in-the-loop:** independently measured trigger/actuator timing with brake/drive controller and fault injection.
3. **Guarded physical test cell:** measured stop distance for the exact serial/configuration/surface/load/grade profile.
4. **Supervised site pilot:** only after competent safety assessment, physical qualification, site-specific sensing/occupancy and emergency-response planning.
5. **Cross-site / cross-vendor replication:** a new independent measurement chain and separately qualified profile for each configuration.

No software pass moves a system up this ladder automatically. The current v0.1 kernel is level 1 only.
