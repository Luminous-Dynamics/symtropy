# Navigation generalization split benchmark v0.1

**Owner:** NAV-GENERALIZATION-001 [#6685](https://github.com/Luminous-Dynamics/symtropy/issues/6685)  
**Parent corpus:** NAV-TRANSITION-001 [PR #6435](https://github.com/Luminous-Dynamics/symtropy/pull/6435)

## Purpose

This package prevents common train/test leakage in navigation robustness and detector benchmarks. It validates the *declared metadata split*, not the underlying physical independence or correctness of the labels.

A random record split can put adjacent windows from one recording session, the same physical event, or the same environment/device/source into training and test. That may yield impressive in-domain metrics while hiding poor generalization.

## Supported protocols

Each manifest represents one fold and one primary holdout value:

| Protocol | Primary axis |
|---|---|
| `grouped_in_domain` | None; event and sequence groups still must be disjoint |
| `leave_one_dataset_out` | `dataset_id` |
| `leave_one_environment_out` | `environment_id` |
| `leave_one_device_out` | `device_id` |
| `leave_one_firmware_out` | `firmware_id` |
| `leave_one_event_class_out` | `event_class` |
| `leave_one_failure_domain_out` | `source_failure_domain_id` |

For holdout protocols, the held-out value must be present in test and absent from train/validation. All sequence-group and physical-event identifiers must be partition-disjoint. Every record must declare dataset, environment, device, firmware, event class and at least one failure-domain ID. Unknown values are not treated as evidence of independence.

A manifest is a single fold; aggregate k-fold results must preserve the exact fold-level manifests and digests.

## Validate the example

```sh
cargo run --manifest-path research/nav-transition-validator/Cargo.toml \
  --bin nav-generalization-validate -- \
  docs/research/nav-generalization-v0.1/nav_generalization_split.schema.json \
  docs/research/nav-generalization-v0.1/fixtures/leave_site_out.example.json
```

The executable first runs JSON Schema Draft 2020-12 validation, then semantic split checks.

## Benchmark reporting

Always report the in-domain grouped baseline alongside held-out folds. For classifiers, include AUROC/AUPRC where valid, detection probability at fixed false-alert rate, and false alerts per hour or route-kilometre. For navigation, include p50/p95/p99 position, velocity and orientation errors; drift accumulation; integrity risk/coverage; false-safe exposure; abstention and requalification delay; compute, energy and latency.

A single score should never erase service-level trade-offs. Report the paired results by environment/device/firmware/failure domain and provide confidence intervals with the number of independent groups, not merely sample count.

## Why these protocols matter

Recent 2026 GNSS anti-spoofing studies report substantially lower detection performance under cross-domain or held-out attack scenarios than under mixed/in-domain splits. See the linked issue #6685 for bibliographic references and reported protocols. Those papers are motivations and comparators, not results reproduced by this validator.

## Claim ceiling

A passing manifest proves only that the declared IDs satisfy the implemented split rules. It does not prove:
- raw or derived data are physically independent;
- training labels or closed test truth are correct;
- data rights permit reuse or redistribution;
- sensor/site/clock dependencies have been completely inventoried;
- the model did not leak information through preprocessing, feature caches, calibration or shared artifacts.

Use the separate oracle provenance, data admission and rights-review gates.