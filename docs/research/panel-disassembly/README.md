# Panel disassembly synthetic corpus v0.1

24 synthetic cases validate bounded panel connection removal, condition, repair/requalification and post-reassembly function logic.

- No fixture number is a real fastener rating, removal torque, robot capability or panel safety limit.
- The evaluator must not compute robot motion, infer structural stability or authorize physical operation.
- Missing/stale connection, panel, calibration or requalification evidence remains Unknown.
- Damage, single-use restriction, failed requalification or failed function test prevents reuse.
- Shared fixtures/resources are counted once; overbooking blocks candidates.
- `AcceptedReusableAssembly` requires successful removal prerequisites, requalification and current post-reassembly fastening, torque, geometry and functional evidence.

See [PANEL-DISASSEMBLY-BENCHMARK-PROTOCOL-V0.1](./PANEL_DISASSEMBLY_BENCHMARK_PROTOCOL_V0_1.md) for the experimental protocol and claim boundaries.


## Exact-head qualification contract

The standalone Rust oracle runs against this corpus with the pinned Rust 1.96.0 toolchain. Its receipt must bind the exact Git HEAD and corpus SHA-256. Formatting, tests, strict Clippy and locked dependency resolution are separate required steps. A workflow status of queued, action_required or skipped is not a test result.

The oracle consumes declared fixture evidence. It does not authorize a physical removal, calculate a trajectory, prove a component is structurally suitable or certify the building.
