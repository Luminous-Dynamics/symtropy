# EV factory sequence integrity and changeover metrics v0.1

**Related issue:** [EV-FACTORY-SEQ-001 #2412](https://github.com/Luminous-Dynamics/symtropy/issues/2412)  
**Status:** synthetic diagnostic instrumentation, not a production dispatch optimizer

## Why this was added

The 20-seed paired simulation showed that adding one server at an upstream station could slightly reduce reported good throughput in the current toy line, even though the end-of-line test is the dominant constraint. One plausible structural mechanism is product-sequence disruption: parallel upstream completions reorder variant batches, and downstream resources incur extra sequence-dependent changeovers. Another is a boundary effect from the finite measurement horizon. Without counters, the report cannot distinguish these explanations.

The correct response is not to declare the effect real or dismiss it as noise. Measure the underlying event paths and test alternative dispatch policies.

## Added station-level measurements

Each station reports:
- `changeovers_started_after_warmup`: count of station starts whose selected server had last processed a different variant, meaning the configured changeover duration was added.
- `output_transfers_after_warmup`: accepted outputs transferred from the station after the warm-up boundary, including the final delivery station.
- `out_of_order_output_transfers_after_warmup`: number of those transfers whose simulation job ID is lower than the highest job ID previously transferred by that same station during the measured window.

Output-order metrics update at the actual transfer/acceptance boundary, not at service completion, because a completed job can be blocked waiting for a finite buffer. Each successful job contributes at most one output transfer per station; internal rework attempts don't count as separate output transfers. Warm-up outputs do not establish the measured-window sequence high-water mark.

With 20 replications, `station_sequence_statistics` reports distributions of all three measures for every station in every line scenario. This lets baseline and one-server interventions be compared on output, setup burden and sequence integrity under matched seeds.

## Why this is a paradigm shift

A factory isn't a set of independent cycle times. When mixed variants share workcells, station assignment, buffer release, product sequence and changeover rules interact. Raising a station's nominal capacity may change the sequence presented downstream and shift the true system constraint. The optimization problem is therefore multi-stage and sequence-aware: good output, changeovers, finite WIP, blocked/starved/down time and quality/rework must be considered together.

## What this version does not do

- Job IDs are used as a simple release-sequence proxy. Real production orders can have priorities, due dates, family grouping, rework loops and explicit resequencing requests, none of which are modeled.
- A count of out-of-order transfers is not a full permutation-distance metric, cannot quantify customer delivery delay, and doesn't itself prove that order inversion caused lost output.
- Changeover events are inferred from each synthetic server's last variant and the fixture duration; no actual production setup logs are used.
- This instrumentation slice did not implement a sequence-aware dispatch policy. A stacked follow-on [draft PR #2443](https://github.com/Luminous-Dynamics/symtropy/pull/2443) now implements a bounded local `minimize_changeover` queue policy against the FIFO control, reports per-variant queue waits, and compares both policies on paired seed streams. It remains synthetic-only and is not a scheduler optimizer or production-release mechanism.
- No due-date model, customer-priority semantics, global route optimization, or plant/MES/PLC control is implied by the follow-on policy.
- The 20 seeds quantify only stochastic variation under this synthetic model, not uncertainty in the model structure, measurement system or parameters.
- Site release, product safety, battery/HV processes, process capability and jurisdiction-specific approvals remain hard-false.

## Next benchmark

Use an intentionally controlled no-setup/homogeneous-product scenario as a structural control. Then test mixed variants with controlled batch sizes, station count, buffer sizes and setup penalties. Compare current server-first dispatch against a specified FIFO/sequence-preserving and variant-aware policy under paired seed inputs. Report throughput and paired uncertainty *alongside* changeover count, sequence inversions, WIP, dwell time, rework and scrap. If additional capacity lowers output, trace it to verified sequence/setup dynamics or fix the simulation before optimizing against it.

This is software/measurement instrumentation only; all inputs are synthetic and no supplier, integrator, plant, landlord or agency was contacted. No quotes requested.
