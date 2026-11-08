# Panel disassembly synthetic corpus v0.1

24 synthetic cases validate bounded panel connection removal, condition, repair/requalification and post-reassembly function logic.

- No fixture number is a real fastener rating, removal torque, robot capability or panel safety limit.
- The evaluator must not compute robot motion, infer structural stability or authorize physical operation.
- Missing/stale connection, panel, calibration or requalification evidence remains Unknown.
- Damage, single-use restriction, failed requalification or failed function test prevents reuse.
- Shared fixtures/resources are counted once; overbooking blocks candidates.
- `AcceptedReusableAssembly` requires successful removal prerequisites, requalification and current post-reassembly fastening, torque, geometry and functional evidence.

See [PANEL-DISASSEMBLY-BENCHMARK-PROTOCOL-V0.1](./PANEL_DISASSEMBLY_BENCHMARK_PROTOCOL_V0_1.md) for the experimental protocol and claim boundaries.
