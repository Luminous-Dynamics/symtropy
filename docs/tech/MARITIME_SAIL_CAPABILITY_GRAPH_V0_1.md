# Maritime sail capability graph v0.1

**Owner:** MARITIME-SAIL-CAPABILITY-GRAPH-001 #5254  
**Parent:** MARITIME-QUALIFICATION-SCOPE-001 #5219 / PR #5221  
**Status:** synthetic domain profile; no physical capability asserted  
**Version:** 0.1.0

## Purpose

Represent sailing as a set of separately qualified functions, not one vessel-wide Boolean called "autonomous". A capability is supportable only for the exact asset/configuration, operating regime and required evidence classes declared for that capability.

This profile deliberately composes the existing qualification scope compiler. It does not create a second generic qualification engine, fault diagnosis system, ODD evaluator, route planner, dynamics simulator, control authority or actuator runtime.

## Key distinction

The generic qualification compiler accepts an explicit `required_dependency_ids` set for each capability-specific scope record. When omitted, its v0.1 default still requires all seven registered dependency classes. The sail profile uses the opt-in form so a wind-observation capability does not inherit artificial dependencies on a sail-force model or weather-routing source, while a VMG route capability does require them.

Each capability is represented by one qualification-scope case. This enables the existing exact-scope evaluator to return a distinct disposition for:
- monitoring true wind;
- estimating navigation state;
- holding a course;
- tacking;
- gybing;
- planning a VMG route;
- reefing/furling.

The graph adds domain-specific prerequisites between those independently evaluated capability records.

## Capability dependency profile

| Capability | Depends on | Minimum evidence dependency classes |
|---|---|---|
| `sail.monitor_wind` | none | sensor topology, calibration, timebase, evaluator, validation profile |
| `sail.estimate_navigation` | none | sensor topology, calibration, timebase, diagnostic model, evaluator, validation profile |
| `sail.hold_course` | wind monitoring, navigation estimate | sensor topology, calibration, timebase, diagnostic model, evaluator, validation profile |
| `sail.tack` | course holding | sensor topology, calibration, timebase, polar/source payload, dynamics model, evaluator, validation profile |
| `sail.gybe` | course holding | sensor topology, calibration, timebase, polar/source payload, dynamics model, evaluator, validation profile |
| `sail.plan_route_vmg` | wind monitoring, navigation estimate, tack capability | all seven evidence classes |
| `sail.reef_furl` | wind monitoring, course holding | sensor topology, calibration, timebase, dynamics model, evaluator, validation profile |

The component identity list is explanatory domain metadata. A changed hull, rig, sail plan, loading, rudder, sail actuator, sensor, calibration or software configuration must change the relevant identity and be reconciled against qualification scope; the profile does not authenticate an asset register.

## Disposition semantics

The referenced scope cases use the parent compiler and preserve its dispositions:
- `SUPPORTED`: the exact scope and declared dependencies match;
- `REQUALIFICATION_REQUIRED`: configuration/prerequisite changed or evidence was revoked;
- `EXPIRED`: qualification or required time-bounded evidence expired;
- `NOT_EVALUATED`: required evidence absent, unknown or malformed;
- `CONFLICTED`: a required source reports conflicting state;
- `OUT_OF_SCOPE`: requested operating regime is not in the declared qualification.

No case can emit actuator commands or grant authority. A graph parent being supported does not make a child supported: each child still must pass its own exact scope case.

## Change-impact closure

The graph is now evaluated under frozen component/evidence changes, not only inspected as static structure. The separate impact corpus and Rust evaluator compute:

1. **Direct impact:** a capability names a changed hardware/software component or a changed evidence-dependency class.
2. **Transitive impact:** any capability depending on a directly or transitively impacted capability is also impacted.
3. **Unaffected set:** all remaining graph nodes, which must be explicitly enumerated and match the frozen expected set exactly.

An unknown changed component, unknown dependency class, empty change event, missing capability, incomplete/overlapping expected partition or cyclic graph is rejected. It is not interpreted as “no impact”.

The eight frozen synthetic change events cover wind-sensor loss, wind-calibration expiry, polar revision, rudder-actuator uncertainty, forecast-source revocation, GNSS receiver failure, shared-timebase revocation and reef-actuator failure.

| Change | Directly affected | Total impact closure | Unaffected |
|---|---:|---:|---:|
| Wind sensor lost | 3 | 6 | 1 |
| Polar surface revised | 3 | 3 | 4 |
| Rudder actuator unverified | 3 | 5 | 2 |
| Forecast source revoked | 1 | 1 | 6 |
| GNSS receiver failed | 1 | 6 | 1 |
| Shared timebase revoked | 7 | 7 | 0 |
| Reef actuator failed | 1 | 1 | 6 |
| Wind calibration expired | 1 | 6 | 1 |

These are expected graph-closure counts, not failure-rate or vessel-performance statistics. They demonstrate the intended least-justified-withdrawal behavior: a local reef actuator fault need not withdraw wind monitoring, while a shared timebase change justifiably reaches every capability that depends on it.

The impact receipt proves only that the static graph and event fixtures reconcile. It does not inspect real vessel state, infer that an actual failure occurred, revoke a live qualification, restrict a vessel or grant authority. The existing exact-scope qualification compiler still evaluates each affected capability against current evidence.

## Research comparators and claim boundary

Commercial offerings already combine vessel/engine data, anomaly detection, rule-based diagnostics and expert advisory. Wärtsilä describes Expert Insight as real-time vessel data compared to a vessel-specific operating profile with AI/rules and expert analysis. Everllence describes PrimeServ Assist as a 24/7 data/AI/expert service using CEON and reports Bureau Veritas SMART(MH3) software type approval; the approval covers a substantial portion of the software review pathway, while vessel-specific elements remain. This project should therefore complement deployed OEM services with a vendor-neutral evidence and qualification layer, not claim that such services lack assurance or that Symtropy currently outperforms them.

The 2026 review of more than 200 marine-engine diagnostic papers highlights continued dependence on laboratory, test-bench, simulation and single-engine evidence, with reusable benchmark data and operational validation still needed. The Aalto marine engine dataset is useful measured test-bench evidence for its specific three-cylinder engine and induced faults; it does not establish fleet transfer.

Relevant standards include ISO 13379-1:2025 (machine-condition data interpretation/diagnostics) and ISO 13381-1:2025 (prognostics). Alignment is a design objective, not a certification claim. The IMO MASS Code was adopted in May 2026 and entered into force on 1 July 2026; autonomy-related software or simulation results do not grant a vessel MASS status or operational approval.

## Selective withdrawal and partial recovery

The profile also freezes five change scenarios. For each one, the validator starts from the directly invalidated capability IDs, then computes the transitive dependent closure over the DAG. The declared invalidated and preserved sets must partition all seven capabilities exactly.

| Scenario | Directly invalidated | Expected withdrawal |
|---|---|---|
| Wind sensor lost | wind monitoring | wind monitoring, course hold, tack, gybe, VMG route, reef/furl |
| Rudder feedback lost | course hold | course hold, tack, gybe, VMG route, reef/furl |
| Sheet actuator feedback lost | tack, gybe | tack, gybe, VMG route |
| Forecast payload expired | VMG route | VMG route only |
| Wind restored; rudder fault remains | course hold | course hold, tack, gybe, VMG route, reef/furl |

This is the important non-monotonicity insight: capability recovery is not a vessel-wide reset. Once wind sensing recovers, wind monitoring can be restored while tack/gybe and route functions remain withdrawn because the rudder-feedback fault still blocks their dependency chain. Likewise, a stale forecast should withdraw weather-dependent route planning without erasing a separately supported wind-observation capability.

These are synthetic dependency-closure cases, not a claim that the current component-to-capability mapping is physically validated. That mapping must be independently reviewed for a named craft and configuration.

## Frozen synthetic corpus

`benchmarks/maritime-observability/sail-capability-profile.v0.1.json` binds seven capabilities and five selective change/recovery scenarios. `benchmarks/maritime-observability/sail-capability-scope-cases.v0.1.json` has 16 cases: seven nominal capabilities plus missing sensor evidence, conflicted and revoked sensor state, stale calibration, changed rig configuration, polar/model change without retention, stale forecast payload, heavy-weather out-of-scope and expired qualification.

The expected outcomes are synthetic known truth. They are not literature statistics, physical sailing performance or vessel operating thresholds.

## Validation

The profile validator checks:
1. exact schema and unique capability IDs;
2. every `depends_on` reference exists;
3. the dependency graph is acyclic;
4. every capability maps to exactly one qualification-scope case;
5. the qualification case contains only the matching capability ID;
6. the explicit dependency set and operating regime match between graph and qualification case;
7. the case's expected disposition is present and included in the separately executed parent-scope receipt;
8. authority fields remain read-only, with no mutation, authorization or actuator output.

The parent qualification-scope binary evaluates the case corpus; this profile validator checks the graph-to-scope binding. These remain separate proof obligations.

## Reproduction

From the Symtropy workspace root:

```sh
cargo test --locked -p maritime-observability-evidence-validator --bin sail_capability_profile
cargo test --locked -p maritime-observability-evidence-validator --bin sail_capability_impact
cargo test --locked -p maritime-observability-evidence-validator --bin qualification_scope
cargo clippy --locked -p maritime-observability-evidence-validator --all-targets -- -D warnings

source_commit="$(git rev-parse HEAD)"
profile="benchmarks/maritime-observability/sail-capability-profile.v0.1.json"
cases="benchmarks/maritime-observability/sail-capability-scope-cases.v0.1.json"
profile_sha="$(sha256sum "$profile" | awk '{print $1}')"
cases_sha="$(sha256sum "$cases" | awk '{print $1}')"

cargo run --quiet --locked -p maritime-observability-evidence-validator --bin qualification_scope -- \
  "$cases" --source-commit "$source_commit" --case-sha256 "$cases_sha"

cargo run --quiet --locked -p maritime-observability-evidence-validator --bin sail_capability_profile -- \
  "$profile" "$cases" --source-commit "$source_commit" \
  --profile-sha256 "$profile_sha" --cases-sha256 "$cases_sha"
```

## Claim ceiling

Passing establishes the deterministic profile-to-scope mapping and synthetic qualification dispositions for the exact corpus and source revision. It does not establish physical wind sensing, hydrodynamic or aerodynamic validity, tack/gybe performance, heel or capsize stability, COLREG compliance, seaworthiness, autonomous sailing, class/flag approval, MASS approval or permission to operate.
