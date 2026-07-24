# symtropy-firstlight

This is the authoritative product integration lane for the Firstlight opening.
It is headless by design: gameplay truth, persistence, residents, IRIS,
catastrophe progression, and Crawler budgets must remain testable without a
renderer or network transport.

The Old Waterworks Bevy example remains a useful presentation and interaction
prototype. New product rules should enter this crate or one of its owned
headless dependencies before being bound to Bevy.

```text
cargo run -p symtropy-firstlight -- demo
cargo test --workspace --all-targets
```
