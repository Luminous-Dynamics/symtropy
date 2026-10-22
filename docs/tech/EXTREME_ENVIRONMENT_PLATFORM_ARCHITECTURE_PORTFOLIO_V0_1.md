# Extreme-Environment Mission Platform Architecture Portfolio v0.1

**Engineering status:** concept architecture only. These are hypotheses for prototypes and test plans, not completed designs, supplier selections, verified performance, safety cases, or permission to operate in any location.

## 1. The architecture shift

Do not build a single enormous vehicle and attempt to make it tolerate every environment. Build a **portfolio of carriers** around common, versioned mission-module, evidence, service and recovery interfaces. Commonality should lower support cost without forcing every job into the same chassis.

The unit to optimize is successful, recoverable mission service—not top speed, theoretical range, payload, autonomy percentage or ground clearance in isolation. Each design must be evaluated against a named mission, a declared operating envelope, the people it serves, environmental limits and its recovery plan.

## 2. Portfolio concepts

### A. RiverLink — modular shallow-draft logistics craft

**First fit:** Amazon and Congo Basin missions where an authorized waterway is the actual access network, particularly medical/cold-chain delivery, community logistics and water/ecosystem sensing.

**Concept:** shallow-draft, repairable hull; protected low-speed propulsion; replaceable electrical and navigation modules; secured cargo rail; a removable insulated/cold-chain pod; physical manual stop and operator-controlled recovery. Solar is an auxiliary energy source, not a promise of perpetual propulsion. Propulsion choice, draft, stability and load ratings must be determined from route/current/weather and vessel regulations.

**Design emphasis:** draft and landing compatibility, safe operation around community boat traffic, wet connectors, corrosion, water ingress, leak containment, battery isolation, manual recovery, refrigeration reserve, and availability of local repair/charging.

**Do not do:** enter unapproved waterways, treat an entire river basin as one route, discharge lubricants/batteries into water, or create forest access infrastructure merely to help a rover.

**Required prototype evidence:** loaded/unloaded static stability, turning/stop distance at declared speed and current, leak tests, power-loss response, drift/anchoring/recovery drills, water-ingress tests, payload restraint, cold-chain telemetry under worst-case ambient/door-opening conditions and supervised route trials.

### B. CanopyEdge — low-disturbance forest and peatland scout

**First fit:** approved trails and disturbed edges for biodiversity monitoring, water-quality support, route condition survey and light supplies in the Amazon/Congo. Not for free-roaming into pristine forest or unapproved peatland.

**Concept:** lightweight, low-speed 4-wheel or articulated rover; replaceable tires/wheels; low center of gravity; protected drive electronics; sealed, drainable service bays; removable sensor mast; swappable field-service modules; manual tow/recovery eyes. A wheeled chassis is a baseline, not a dogma: terrain-specific tests decide whether wheels, tracks, feet or no ground vehicle are appropriate.

**Design emphasis:** ground pressure and rut depth, vegetation contact, soil/moisture sensitivity, non-native biological transfer, water ingress, mud clearing, local tool repair, and data minimization for community-sensitive locations.

**Do not do:** infer soil bearing from satellite appearance alone, cut or widen trails, cross peatlands without site-specific approval, or use maximum payload as a substitute for low impact.

**Required prototype evidence:** soil-pressure/rut tests on permitted analog surfaces; cleaning/biosecurity verification between sites; wet-heat and connector cycling; obstacle/turnaround performance on a bounded route; recovery without importing heavy machinery; and ecological review with local practitioners before field movement.

### C. PolarSled / PolarCrawler — choose the lowest-complexity polar carrier

**First fit:** permitted scientific instruments, approved resupply and maintenance support in Antarctica or other polar conditions.

**Concept choice:** prefer a sled train pulled by an already-authorized, proven vehicle when the towing vehicle and route exist; use an independently powered crawler only when its additional capability justifies its additional maintenance, energy demand and environmental footprint. The subsystem includes cold-rated power, instrumented thermal management, replaceable seals, protected cabling, load restraint, spill/waste controls and defined remote-recovery points.

**Design emphasis:** cold soak before startup, battery and actuator derating, lubricant/seal behavior, snow/ice transition, visibility/whiteout stop policy, safe towing, long lead-time spares, waste retrieval and spill response.

**Do not do:** treat an outdoor cold test as Antarctic qualification or deploy without the relevant national procedure, permit and environmental impact assessment. The Antarctic Treaty Secretariat adopted revised Environmental Impact Assessment Guidelines in Resolution 1 (2026), on 21 May 2026.

**Required prototype evidence:** cold chamber startup and shutdown, full-load energy budget at temperature, braking/holding/tow tests, frozen connector serviceability, communication-loss behavior, battery fault containment, environmental containment drills and witnessed recovery in a controlled cold analog.

Source: [Antarctic Treaty Secretariat, Resolution 1 (2026)](https://www.ats.aq/devAS/e/Meetings/Measure/874?s=1).

### D. DuneRunner — dust-sealed desert utility rover

**First fit:** remote sensing, inspection, instrument maintenance and modest cargo delivery across a surveyed, permitted route in hot/dusty terrain.

**Concept:** serviceable multi-wheel rover with replaceable tire/suspension parts, accessible dust filters, protected thermal paths for batteries/electronics, shade for parked equipment, optional deployable solar supply, and an explicit recovery/energy-cache plan. Evaluate independent wheels versus tracks under actual sand/rock/wadi profiles; neither gets a universal terrain advantage in advance.

**Design emphasis:** solar load, hot-soak limits, abrasive dust, filter service interval, thermal derating, sand sinkage, rock strike, flash-flood/wadi risk, water independence and remote recovery. Solar assists energy availability but does not remove storage, heat or traction constraints.

**Do not do:** claim a platform is “desert ready” based on a cool-day drive, operate beyond its validated thermal envelope, or send it out with no retrieval path.

**Required prototype evidence:** staged heat soak; dust exposure followed by inspection and service; sand/rock transition loops with logged wheel slip/energy; sealed-enclosure thermal tests; recovery with carried tools; and operation under degraded communications.

Research anchor: [NASA/JPL's 2026 desert field test with an advanced rover prototype](https://science.nasa.gov/photojournal/desert-field-test-with-nasa-advanced-rover-prototype/). The test establishes that desert analogues are useful test sites, not that our design is qualified.

### E. AidRover — civilian humanitarian carrier

**First fit:** documented medical delivery, water/essential-supply transport, infrastructure assessment and disaster response along an authorized route, including selected remote-access cases in Afghanistan.

**Concept:** low-speed carrier with a secured removable cargo pod, human-visible status indicators, teleoperation/manual fallback appropriate to the local rules, physical stop controls, communications-loss safe state, data minimization and a simple manual recovery interface. Prefer predictable and repairable behavior over autonomy that cannot be audited by the operating team.

**Design emphasis:** route authorization, human supervisor availability, conservative stopping, load restraint, cold/heat limits, privacy, local repair and recovery. The platform must stop when required route or hazard evidence is unknown.

**Afghanistan safety boundary:** suspected explosive ordnance is a no-go absent an authorized humanitarian process establishing safe access. The platform is not to autonomously approach, probe, manipulate or clear explosive hazards. It is not an armed, targeting or tactical system.

**Required prototype evidence:** supervised trials only on authorized, assessed routes; loss-of-comms stop tests; measured stopping/holding limits; safe payload handover; privacy audit; accessibility and usability review with local operators; and recovery drills. Do not use synthetic route gates as actual route-clearance evidence.

Source: [WHO Afghanistan Public Health Situation Analysis, March 2026](https://www.who.int/publications/m/item/public-health-situation-analysis---afghanistan---march-2026) and [UNMAS Afghanistan](https://www.unmas.org/en/afghanistan).

### F. AirScout — optional small aerial sensing/cargo tool

**Role:** aerial mapping, environmental observation and small-item transport only where the aircraft, airspace, operator and mission are authorized. Treat it as an accessory within the portfolio, not the default solution for dense canopy, high wind, extreme cold or life-critical delivery.

**Gates:** airspace/legal permission, weather envelope, payload restraint, communications-loss response, return/landing plan, privacy and wildlife buffer. If any mandatory gate is unknown, the mission does not launch.

## 3. Common interfaces without forced uniformity

Use the following as **design contracts to prototype**, not as a claim that a standard connector or bus already solves every integration problem:

- **Mission module manifest:** module ID/version, mass, center-of-gravity envelope, restraint method, electrical demand, thermal/environment limits, payload-specific hazards, service tasks, required approvals, and supported host classes.
- **Mechanical attachment:** keyed attachment points, positive latch indication, load envelope, captive fasteners where appropriate, visible inspection status, and defined manual release procedures. The host platform must reject modules outside its load/stability envelope.
- **Energy:** separate traction energy from auxiliary loads; define voltage/current and isolation per platform class, polarity/keying, pre-charge/disconnect behavior where applicable, state-of-health reporting, thermal derating and safe battery transport. No blind hot-swap or mixed-battery chemistry assumptions.
- **Control and telemetry:** versioned command/status contract; vehicle-side safety authority; physical emergency stop and an independent stop path appropriate to the system; logged manual/autonomy mode transitions; operator-visible uncertainty; watchdog and comms-loss behavior; no dangerous continuation merely because the network disappeared.
- **Service:** accessible wear components, reproducible fault codes, safe isolation steps, tool requirements, locally sourceable consumables where possible, spares list, repair log and recovery procedure.
- **Evidence:** exact hardware/software/configuration IDs, test conditions, raw captures, calibration state, failures, recovery records and environmental observations. Never let a passing synthetic readiness report become a field capability claim.

Common interfaces should serve module interchange, monitoring, repair and evidence portability. They do not require one chassis, battery pack, autonomy stack or vendor across all settings.

## 4. Verification sequence

1. **Mission and site review:** co-design with local users; define the exact route, season, cargo, access conditions, ecological constraints, operating permissions and the definition of mission success.
2. **Contract/simulation:** evaluate the declared mission envelope, hard gates, missing evidence, power/thermal estimates, payload stability and recovery assumptions. Use negative controls deliberately.
3. **Bench qualification:** measure cold/heat soak, humidity, water/dust ingress, vibration/shock, load retention, battery protection, ingress after wear, manual stop and safe-state behavior. Select environmental test methods with a qualified lab.
4. **Hardware-in-the-loop:** inject sensor faults, inconsistent measurements, stale maps, communications loss, low energy, over-temperature and actuator failures. Test behavior must be bounded and logged.
5. **Controlled analogue:** permitted sites only; instrumented closed tests with retrieval assets and human supervisor. No community or protected ecosystem is the first proving ground.
6. **Limited supervised pilot:** only after applicable certification/permit, local operator training, documented recovery, consent and incident process. Record all failures and near misses.

## 5. Standards as inputs, not marketing badges

- [ISO 12100:2010](https://www.iso.org/cms/%20render/live/en/sites/isoorg/contents/data/standard/05/15/51528.html) is a machinery risk-assessment and risk-reduction methodology; it explicitly does not itself address environmental harm. Add environmental assessment separately.
- [ISO 3691-4](https://www.iso.org/standard/83545.html) addresses safety of driverless industrial trucks and their systems within its scope. The ISO page also notes limitations for severe conditions; it must not be presented as a blanket qualification for extreme-environment autonomous rovers. The third edition was under development in 2026, so verify the current status at each design review.
- [IEC 60068-2-53](https://webstore.iec.ch/en/publication/539) describes combined climatic and dynamic environmental tests; choose applicable cold, dry heat, humidity, vibration or shock methods based on the actual equipment/environment.
- For Antarctica, apply the relevant Antarctic Treaty System and national environmental assessment requirements; a generic machinery standard is not a substitute.

## 6. Benchmark targets and measurement discipline

- 100% of a candidate's required mission gates and metric records present exactly once; otherwise reject the assessment.
- Zero failed/unknown mandatory gates counted as eligible; zero unapproved operation, hazard handling or ecological no-go violation in controlled testing.
- 100% successful explicit safe-state entry across the predefined fault-injection suite, with expected behavior agreed by the safety analysis before execution.
- Report mission completion, energy per payload-distance, recovery success, repair time, spare-parts burden, environmental disturbance and operator workload by mission/environment stratum. Do not collapse them to one composite score.
- Use 95% lower confidence bound for a stated mission-success target, not a single best run. Under independent Bernoulli trials, 59/59 success is about the minimum to support a one-sided 95% lower bound near 95%, and 299/299 for a lower bound near 99%. Repeated runs may not be independent; stratify and disclose the test design.
- Establish site-specific ecological thresholds with qualified local experts and permissions; there is no defensible universal disturbance index that makes tropical peatland impacts comparable to Antarctic wildlife impacts.
- Recovery is a primary capability: every test must report planned versus actual retrieval method, staff, equipment, time, environmental impact and whether the asset was recovered intact.

## 7. First prototype sequence

Start with a low-risk modular sensor rover on a controlled, permitted test site to validate the common payload/service/evidence interface, then a shallow-draft logistics craft for authorized waterway analogue tests. Build the heat/dust and cold variants from separately qualified subsystems rather than assuming one rover will survive both. The humanitarian carrier should proceed only with a partner-defined civilian mission and formal route/safety controls. AirScout remains optional until the actual mission demonstrates that aircraft access and privacy/wildlife rules are satisfied.

**Decision rule:** if a mission can be served more safely and with less ecological/maintenance burden by an existing boat, sled, wheeled carrier, human team or fixed sensor network, the benchmark should say so. Our objective is to solve the mission—not to justify building a robot.
