# RRVR compiled campaign plan v0.1

**Issue:** [#7027 — deterministic fault schedule compilation and replay identity](https://github.com/Luminous-Dynamics/symtropy/issues/7027)  
**Compiler:** src/campaign_plan.rs  
**Compile CLI:** rrvr_campaign_compile  
**Verify CLI:** rrvr_campaign_verify  
**Schema:** schemas/compiled_campaign_plan_v0_1.schema.json  
**Source manifest:** fixtures/fault_campaign_paired_v0_1.json

## Why a plan compiler

A validated campaign manifest should not be reinterpreted independently by every injector adapter. The compiler turns the frozen manifest into one immutable instruction plan with exact identities and cryptographic commitments. A simulator adapter, ROS injector or later HIL adapter can translate this plan into system-specific commands without redefining scenario identity, event order, target component, fault-domain or parameters.

The plan is not an injector. It contains no credentials, bearer permits, action API or native authority. Compiling a plan does not execute any event or prove fault delivery.

## Canonicalization contract

Version 0.1 uses a deliberately narrow JSON representation:

- UTF-8 JSON.
- Object keys sorted through serde_json::Value using the pinned serde_json 1.0.150 dependency configuration (the preserve_order feature is not enabled).
- JSON arrays preserve order. Fault events are never sorted or normalized after compilation.
- Integer seeds and sequence numbers only; no floating-point quantities are part of the plan schema.
- Enum values are serialized as their stable snake_case strings.
- String values are committed as their exact UTF-8 content; no locale or Unicode normalization is applied.
- No wall-clock timestamps, host paths, random IDs or machine-specific metadata are inserted.

The canonicalization identifier is serde-json-value-sorted-keys-v0.1. A dependency/canonicalization change requires a new version/identifier; it must not silently change the digest contract.

## Three digest layers

1. **Source manifest SHA-256**: hash of the exact input manifest bytes, including whitespace. The verifier requires the exact same bytes.
2. **Pair commitment SHA-256**: for each pair, hash the campaign ID, pair ID, control class, fault-model revision/digest, full platform and reset bindings, task, policy/checkpoint, scene, action-prefix, seed, logical-effect ID and exact ordered fault list. Both arms share the same pair commitment; scenario ID and arm remain separate fields.
3. **Scenario schedule SHA-256**: hash of the exact ordered fault-event array for that scenario, including sequence number, fault ID, cut point, target layer/component, fault-domain ID, fault kind and parameter digest.

The top-level plan_sha256 commits to the complete compiled-plan body with the self-referential plan_sha256 field omitted. The SHA-256 of the emitted plan file bytes is a separate artifact digest. They are not interchangeable.

## Compiler and verifier

Compile:

    cargo run --locked --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --bin rrvr_campaign_compile -- manifest.json compiled-plan.json report.json

Verify:

    cargo run --locked --manifest-path research/mobility-rrvr-runtime-trace/Cargo.toml --bin rrvr_campaign_verify -- compiled-plan.json manifest.json expected-plan-sha256 report.json

Compilation first validates the source campaign manifest. Verification then:
- strictly deserializes the plan and rejects unknown fields;
- checks the version and canonicalization identifier;
- recomputes every schedule digest;
- checks contiguous event sequence numbers;
- checks pair membership, scenario IDs and arm inventory;
- checks pair equivalence over task/policy/scene/action-prefix/seed/logical-effect/fault schedule;
- recomputes each pair commitment from the full platform/reset binding;
- verifies required layer/cut coverage;
- recomputes the top-level plan digest;
- checks exact source manifest bytes;
- checks canonical output bytes;
- compares against an externally pinned expected plan digest.

The external expected digest is essential: an embedded self-hash is not a signature. A consumer must pin the digest it trusts through an authenticated channel or release record. The verifier confirms byte/commitment integrity relative to that pin; it does not authenticate who published the plan.

## CI negative controls

The workflow compiles the same manifest twice and requires byte-identical plan outputs. It checks the plan shape and required digests, verifies it against the exact manifest bytes and expected plan digest, then mutates a fault event without updating commitments and requires the verifier to reject it.

Unit tests additionally require:
- same manifest bytes + compiler version -> same plan bytes and digest;
- same pair commitment across comparator/full-RRVR arms;
- changed source digest -> changed top-level plan identity;
- changed shared policy/checkpoint or fault parameters -> changed pair commitment;
- reordered fault events with resequenced positions -> changed pair commitment and schedule digest;
- changed platform component revision -> a changed pair commitment (through the bound platform profile);
- source-manifest byte mismatch -> verifier rejection;
- changed schedule/platform binding without refreshed commitments -> verifier rejection;
- unpaired or otherwise invalid source manifest -> compilation rejection.

## Adapter contract

The first adapters should be pure translators:

- simulator adapter: translate each event into a deterministic simulator fault command and return injection receipt IDs;
- ROS/runtime adapter: map layer/component IDs to a pinned ROS node, action/service/topic/executor boundary and preserve event sequence;
- controller/driver adapter: map the exact target and fault domain to a supported test hook, vendor diagnostic or hardware fault interface;
- HIL adapter: require a separately reviewed test harness and hardware reset/abort contract before translation.

Every emitted injector command should include campaign ID, plan SHA, pair ID, scenario ID, fault ID/sequence, cut point, target component, fault-domain ID and parameter digest. An adapter must fail closed when a target cannot be resolved or when the resolved revision differs from the compiled platform binding. Adapter execution receipts are separate outputs; they do not rewrite the source plan.

## Claim ceiling

A verified plan establishes only that the execution instruction artifact is deterministic and consistent with the exact manifest bytes and externally pinned plan digest. It does not mean:
- the injector ran or delivered a fault at the scheduled cut;
- a target layer/component mapping is complete or correct;
- the controller/driver/firmware inventory is complete;
- owner evidence or observer independence is authentic;
- simulator results generalize to HIL or physical hardware;
- any native action or permit was authorized;
- robot safety has been established.

The next engineering increment is a simulator adapter that emits injection receipts without changing plan identity, followed by ROS graph/controller/driver integration and a separate HIL qualification programme.


## Delivery commitments and execution evidence

Each planned fault now includes its scheduled simulation step, target revision and target-profile digest, plus an expected delivery-signature SHA-256. The platform binding pins the fault-injector ID/revision/binary digest/fault domain and native-adapter ID/revision. These fields are included in the fault schedule digest, pair commitment and top-level plan hash.

A later adapter must emit the separate execution bundle described in [EXECUTION_EVIDENCE_V0.1.md](EXECUTION_EVIDENCE_V0.1.md). It must not write execution receipts back into the plan. Unknown delivery or dispatch outcomes cannot be converted into a new injection without reconciliation.


## Gazebo/ROS simulation platform bindings

The platform digest now includes the ROS 2 Simulation Interfaces package revision and the exact SceneBroadcaster system ID, build revision and binary digest. The Gazebo capability preflight compares these values, plus simulator build, ROS distribution, environment/world profile and reset profile, against the captured capability report. See [GAZEBO_ADAPTER_V0.1.md](GAZEBO_ADAPTER_V0.1.md).

The preflight CLI is deliberately offline and labelled as a captured-profile check. A real simulator transport must discover the live feature set and verify state/reset behavior before any event can be scheduled.

## Observer quorum commitment

The compiled platform binding now includes the minimum delivery-observer quorum and the full observer profile set (observer ID, revision, binary SHA-256 and common-cause domains). These fields are included in the source manifest commitment, pair commitment and top-level plan digest. A consumer cannot replace the observer binary or silently change a declared shared failure domain without changing the plan identity.
