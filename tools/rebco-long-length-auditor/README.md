# REBCO long-length auditor

Run from the Symtropy repository root:

```sh
cargo test --locked -p rebco-long-length-auditor
cargo run --locked -p rebco-long-length-auditor -- docs/research/fixtures/rebco_long_length_synthetic_v0_1.json
```

The auditor validates reel/segment geometry, observation identity, evidence-state requirements, source digest shape and conservative full-width critical-current coverage. Missing or false calibration status excludes a reading from qualified coverage and is reported separately as observed-but-unqualified. It explicitly does not evaluate conductor acceptance. The bundled fixture is synthetic only. See `docs/research/superconductivity/SC_LONG_LENGTH_QUALIFICATION_V0_1.md` for scope and claim limits.
