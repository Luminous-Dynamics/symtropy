# Read-only VDA 5050 v3.0.0 observer v0.1

**Status:** bounded read-only message decoder with synthetic fixtures. This is not a VDA certification, MQTT client, fleet manager, robot controller, safety system, identity authenticator or construction acceptance authority.

## Purpose

This crate takes an explicit MQTT topic path/name plus the exact application payload bytes and emits a bounded typed observation. It does not connect to a broker, publish messages, dispatch orders, or issue motion commands.

Supported topics:
- order;
- state;
- factsheet;
- zoneSet;
- connection;
- instantActions;
- responses.

Other topics (including visualization) return a typed unsupported-topic error in this version.

## Topic identity and cloud broker adaptation

VDA 5050 v3 describes a suggested local MQTT topic hierarchy:

```text
interfaceName/majorVersion/manufacturer/serialNumber/topic
```

Example: `vda5050/v3/ACME/R-01/order`.

The guideline allows cloud-hosted brokers to adapt the topic hierarchy. The API therefore accepts:
- a topic name or full topic path;
- an optional explicit manufacturer/serial identity claim for cloud-specific topic structures;
- exact payload bytes.

When the suggested five-level local hierarchy is present, the observer checks manufacturer and serialNumber against the payload header. If an explicit identity is also supplied, both sources must agree. This check binds claims in the message and topic; it does not authenticate the publisher.

## Decoding and preserved meanings

- VDA payload header version must be exactly 3.0.0.
- Duplicate JSON object keys are rejected before conversion to a JSON value.
- Payload size is bounded at 16 MiB.
- The SHA-256 digest is computed over the exact original payload bytes, including whitespace and original JSON formatting.
- Unknown fields are not used as inferred semantics; the raw payload digest remains available for audit.
- Order ID and update ID remain VDA transport identifiers, not construction task or design revision identifiers.
- Node and edge progress remains route/executor progress.
- Action status FINISHED remains an executor report, not proof of physical construction output or acceptance.
- Map ID/version remains navigation-map metadata, not a design revision or site epoch.
- Factsheet remains a robot-reported capability claim, not independently verified configuration or project qualification.
- zoneSet and zone request/response messages remain fleet traffic observations, not construction permits, risk assessments or safety-rated motion authorization.
- Connection state is a protocol connection report, not robot health.

## Output boundary

The observation enum has only transport-order, robot-state, factsheet-claim, fleet-zone, connection, instant-action and zone-response variants. It has no variant for:
- construction work completed or independently accepted;
- physical inspection passed;
- site safety authorized or safe to move;
- authenticated robot identity;
- capability qualification.

A consumer must join the observations to the separately versioned construction contract, measurement evidence and authorities. It must not infer the missing state.

## Usage

Decode a local five-level topic:

```sh
cargo run --manifest-path research/construction-vda5050-observer-v0-1/Cargo.toml --locked -- \
  decode vda5050/v3/ACME/R-01/order \
  research/construction-vda5050-observer-v0-1/fixtures/order.json
```

Decode a cloud-specific topic using an explicit identity claim:

```sh
cargo run --manifest-path research/construction-vda5050-observer-v0-1/Cargo.toml --locked -- \
  decode cloud/tenant/project/vda/order \
  research/construction-vda5050-observer-v0-1/fixtures/order.json \
  --manufacturer ACME --serial-number R-01
```

## Synthetic fixtures and tests

The fixtures include valid order, state/action-FINISHED, factsheet, zoneSet, connection, instantActions and responses messages. Unit tests exercise:
- order transport identity distinct from task identity;
- local topic/header identity mismatch and cloud hierarchy identity binding;
- action FINISHED as executor-only;
- zone GRANTED and responses GRANTED as fleet traffic only;
- mapVersion as navigation-only;
- factsheet as a reported claim;
- zoneSet geometry/category as fleet information;
- connection state, instant action request and zone response handling;
- v2 version rejection, topic major mismatch, duplicate key rejection, unknown action status, unsupported topic and exact raw-payload digest changes;
- payload size limit.

The focused Rust 1.96 workflow runs metadata, formatting, tests and strict Clippy across Ubuntu, Windows and macOS, then exercises the CLI on the fixture messages.

## Upstream authority and known schema discrepancy

The VDA 5050 v3.0.0 PDF is authoritative when the repository JSON artifacts differ. The official document defines the factsheet key as `mobileRobotKinematics` (plural); the pinned repository factsheet schema snapshot has a singular spelling in the `typeSpecification.required` array. The independent schema-validation script therefore applies a transparent, in-memory one-field overlay to align that one required-field typo with the published PDF. It does not edit or relabel the upstream schema. The order/factsheet/visualization JSON trailing-comma issues at the v3.0.0 tag are handled by pinning the upstream corrective snapshot at `ad169568749f3799e3aac72ae814c4167a7677f9`, exactly as recorded in #3704.

The focused workflow validates all seven frozen fixtures against the pinned VDA schema snapshot, then separately runs the Rust parser tests and CLI. A schema check validates payload shape, not semantic correctness, publisher authenticity or interoperability.

## Upstream authority and limitations

The profile is pinned to VDA 5050 v3.0.0 semantics. The repository's 3.0.0 JSON tag has invalid JSON in three schemas (order, factsheet, visualization); the corrective repository snapshot is pinned separately in the adjacent crosswalk research under #3704. The VDA-published document remains authoritative if the repository schemas differ.

This decoder validates a bounded subset of each supported message and does not claim full JSON Schema validation or VDA conformance. It is a test/replay observer only. Full schema validation, unknown extension policy and interoperability claims remain plugfest work.

## Related work

- #3644: canonical construction task contract.
- #3704: VDA 5050 construction semantic crosswalk.
- #3706: independent mixed-fleet plugfest.
- #3709: byte-preserving sealed trace and Rust verifier.
- #3722: independent construction-layout benchmark.
- #3723: read-only VDA 3.0 observer.

## Claim ceiling

Passing unit tests establishes only that this implementation recognizes the declared VDA 3.0 message subset and preserves the stated semantic boundaries on synthetic inputs. It does not establish MQTT interoperability, VDA certification, supplier conformance, cybersecurity, authenticated identity, safety qualification, construction performance, independent acceptance or field interoperability.
