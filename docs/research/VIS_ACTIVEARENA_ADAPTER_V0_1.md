# VIS-ACT-007: native ActiveArena controller-ablation contract

**Status:** integration design and source audit; no ActiveArena run has been executed from Symtropy. This file must not be read as a reproduction result.

## Why native evaluation is the next gate

The synthetic active-query benchmark has useful failure diagnostics, but neither loss nor its work-unit proxy has a valid numeric conversion to robot task success, wall time, VRAM, RAM or energy. ActiveArena is the primary external target because its initial observations are insufficient for many tasks and success requires a multi-round perception/action loop.

The current official [ActiveArena website](https://leeibo.github.io/ActiveArena/) reports the published task-macro mean for the strongest listed configuration, ActiveArena-OFT, as **62.97% ID / 47.60% OOD** over 35 tasks; ActiveArena-Plan is **60.97% / 45.54%**. Its OOD results highlight Multi-Object Loop (33.33%), Multi-Object Decision (27.20%) and Interactive Information Acquisition (35.33%) as particularly difficult. These are external published reference points, not results for Symtropy.

The current [repository README](https://github.com/leeibo/ActiveArena) and [reproduction guide](https://github.com/leeibo/ActiveArena/blob/main/docs/reproduction.md) specify 35 tasks, five task categories, an 18-dimensional action representation, fixed seed lists and two settings. The official protocol uses each task's committed 100-entry seed list and evaluates the first 50 ordered seeds in ID and OOD settings. Report each task's success rate and the task-macro average per setting. Do not pool frames or substitute the synthetic loss.

## Source audit snapshot

The upstream files checked for this contract are:
- `leeibo/ActiveArena/script/eval_policy.py`, blob `6f98fe08413c45c743e047754cec9ba21ec38a98`.
- `leeibo/ActiveArena/envs/_base_task.py`, blob `d0b26345a82ca55fd42a2ed3caf9ffb97fe14fdf`.
- `leeibo/ActiveArena/docs/reproduction.md`, blob `980dddbb9268d9f9b1e9deee71ae5ef1dc529fcf`.
- `leeibo/ActiveArena/ACTIVEARENA_EVAL_LAUNCH.md`, blob `7f659934cb764bd7dd2c9937fca4a893db43441c`.

The upstream setup separates simulator dependencies from the ActiveArena-VLA model server/weights. Its official launcher supports a dry-run configuration check; that is not a simulator smoke test. A one-episode actual smoke run must complete model inference and execute a task before baseline-parity claims are made.

## Proposed Rust-first boundary

Keep the evidence store, temporal belief update, action scoring, budgets, decision traces and metrics in Rust. Implement only the minimum glue required by the current upstream Python evaluator/model-server boundary. The shim must translate only:
1. the latest RGB observation, task instruction, timestamp/tick and legal public state supplied by the environment into a bounded versioned message;
2. a Rust controller response into the upstream action interface without changing action decoder or executor semantics;
3. reset/error/terminal signals and timing counters back into the Rust session.

Prefer a local, versioned JSON Lines or protobuf-like protocol between the shim and Rust controller over recreating the state logic in Python. Message schema needs explicit version, episode/session ID, sequence number, observation digest, timestamp, action, status, and error field. Never include environment oracle state, hidden object poses, evaluator seed labels, unqueried scene state, task success result or future observations in controller messages.

A useful first interface sketch is:

```text
ObservationV1 {
  protocol_version,
  episode_id,
  step_index,
  monotonic_timestamp_ns,
  instruction_utf8,
  camera_rgb_sha256,
  camera_rgb_bytes_or_shared_memory_ref,
  public_proprioception,
  previous_action_status
}

ControllerReplyV1 {
  protocol_version,
  episode_id,
  step_index,
  requested_view_or_action,
  optional_stop_or_abstain,
  explanation_code,
  state_digest_sha256
}
```

This is a design contract, not an existing upstream API. Bind the exact mapping only after checking the chosen upstream baseline and available action/view affordances.

## Required controller-only baselines

On the exact same backbone/checkpoint, prompt, image preprocessing, subtask supervision, action head/decoder, action executor, environment revision, task seed order and inference precision, compare:
1. no persistent memory / permitted current observation context;
2. FIFO or recent-window memory;
3. uniform reservoir sampling;
4. query/evidence-keyed memory;
5. hazard-aware or source-time-aware memory;
6. sparse/planner-guided memory management;
7. active query / view selection;
8. combined memory + active selection only after the individual ablations are complete.

Do not compare these as a controller ablation if changing the backbone, checkpoint, planner, action head or subtask supervision at the same time. Those can form a separate factorial experiment afterward.

## Execution gates

1. Pin upstream commit SHA, task configs, assets, weights and licenses; record all hashes.
2. Dry-run the official launcher and verify all paths/checkpoints/config without claiming it executed the task.
3. Run a one-task/one-episode actual smoke test and preserve logs, rollout and success evaluator output.
4. Reproduce a named subset of official baseline tasks before full evaluation.
5. Run all 35 tasks × first 50 ordered ID seeds and all 35 tasks × first 50 ordered OOD seeds. Count failed or crashed episodes as failures according to the official protocol; report missing runs separately and do not silently drop them.
6. Only then swap the Rust controller in, keeping the rest of the stack unchanged.
7. Confirm with a paired per-task/per-seed comparison and bootstrap over episodes grouped by task. Report ID and OOD separately, per-category and per-task results, aggregate task-macro success, success gap, failed-reset/action rates, measured latency, VRAM, system RAM, observation count and view/evidence efficiency.

Proposed ambitious internal gate: **+5 absolute percentage points OOD task-macro success** over the strongest eligible baseline reproduced locally, with paired 95% interval above zero and no more than 3 percentage points absolute ID regression. This is an engineering target chosen for the project, not a general community standard.

## Current decision

Finish and qualify the synthetic budget frontier/stop-cost sensitivity first, then implement this integration boundary in a separate PR. Do not let another synthetic win postpone native evaluation, and do not claim Symtropy beats ActiveArena-OFT until an eligible controller-only comparison demonstrates that on the official evaluator.
