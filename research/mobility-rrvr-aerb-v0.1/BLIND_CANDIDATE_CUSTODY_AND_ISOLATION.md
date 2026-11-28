# AERB blind candidate execution custody and isolation v0.1

**Status:** sandbox-command builder authored; process isolation and campaign custody are not yet qualified.

## Paradigm shift

A blind evaluator must not merely hide labels in JSON. It must enforce a one-way information boundary between an evaluator that holds hidden truth and a candidate process that sees only candidate-authorized observations. Candidate execution is treated as an untrusted computation inside a disposable, resource-bounded namespace.

The campaign protocol is:

1. Evaluator custodian generates the hidden assignments with the CSPRNG-backed primitive in blind_assignment_v0_1.rs.
2. Custodian commits to the evaluator reveal before candidate execution.
3. Materializer writes only candidate-visible manifests, episodes, admitted observations and a pinned candidate runtime bundle into isolated staging roots.
4. Candidate executes in a namespace with no evaluator mount, no network namespace connectivity, a cleared environment, read-only input/runtime mounts and one writable output mount.
5. Candidate output is sealed and hashed before evaluator-side scoring.
6. Evaluator scores using its private truth and verifies the post-run reveal against the pre-run commitment.
7. An independent verifier recomputes the commitment, output digests, corpus counts, exclusions and all failure denominators.

The public deterministic seed-42 corpus remains a conformance fixture. It is not an evaluation corpus and must not be included in blind-performance denominators.

## Implemented first slice

Source: evaluator/src/blind_execution_sandbox_v0_1.rs.

The pure builder validates:
- all mount roots exist, canonicalize to directories, and are pairwise disjoint;
- evaluator secret root is disjoint from candidate input, runtime and output roots;
- bubblewrap, prlimit and candidate executable are executable files;
- candidate executable resolves strictly inside the sealed runtime bundle;
- runtime bundle contains the expected bin directory;
- candidate arguments do not contain host-absolute paths, path traversal or NUL;
- every resource limit is positive.

The builder emits a direct prlimit → bubblewrap → candidate argv chain, with no shell:
- explicit --unshare-user, --unshare-pid, --unshare-ipc, --unshare-net and --unshare-uts (required namespaces fail closed);
- --disable-userns plus --assert-userns-disabled after explicit --unshare-user, so nested user namespace creation is not best-effort;
- --cap-drop ALL, --new-session, and --die-with-parent; the canary verifies CapEff, CapPrm and CapBnd are all zero inside the candidate;
- read-only mounts for /candidate/input and /runtime;
- the sole writable host mount at /candidate/output;
- private /proc, minimal /dev, and a size-bounded tmpfs at /tmp;
- host launcher uses env_clear before starting prlimit/Bubblewrap, preventing CI/evaluator variables from reaching the setup parent; stdin/stdout/stderr are redirected to /dev/null so candidate-controlled streams cannot leak into evaluator logs; the candidate environment is then cleared and explicitly sets HOME, PWD, TMPDIR, PATH, LANG and TZ;
- rlimits for address space, CPU time, process count, open files, per-file output size and core dumps disabled.

The unit tests inspect command construction and reject path-overlap, evaluator-root overlap, host paths in arguments, path traversal, an executable outside the runtime, and zero limits. A separate static musl canary probes the actual Bubblewrap runtime and is executed by the dedicated workflow.

## What this does not establish

The builder itself does not launch a process. The separate canary binary exercises a real launch when its CI workflow passes. Until that exact-head canary succeeds, the process boundary remains unqualified. It does not prove that:
- the current host permits user/PID/network namespaces or that Bubblewrap is trustworthy and correctly configured;
- the requested namespace was actually created (the launcher's successful exit and canary tests must prove this);
- the candidate cannot exploit the host kernel or Bubblewrap;
- cgroup v2 memory, CPU, I/O and aggregate process limits are enforced;
- the writable output directory has an aggregate storage quota (RLIMIT_FSIZE is a per-file ceiling);
- candidate inputs/runtime are immutable between path validation and mount (the staging custodian must create private, immutable, content-addressed roots and prevent path replacement);
- the runtime bundle contains every dynamic loader/library dependency (use a pinned hermetic runtime, patchelf'd loader/RPATH, or statically linked executable);
- stdout/stderr are bounded or free of hidden-label content (the evaluator must cap/censor logs before retaining them);
- evaluator-side post-run reveal and scoring are independently reproducible.

Bubblewrap's --size applies to the next tmpfs, and the profile places it immediately before --tmpfs /tmp. The full filesystem baseline is created from scratch: no host root/home/etc/sys is bound into the candidate namespace.

The command builder uses canonicalized host paths rather than open directory file descriptors. Until the staging custodian owns and freezes those roots for the entire setup/launch interval, there is a path-replacement race. Do not treat this API alone as a security boundary.

For high-adversary campaigns, the production target should be a microVM or a hardened gVisor/Firecracker-style boundary with a pinned kernel/runtime, cgroup v2 resource control, seccomp syscall policy, output quotas, separate evaluator host/identity and an independently verified image digest. Bubblewrap remains a useful lightweight defence-in-depth layer, not a claim of kernel-level immunity.

## Required execution canary before any blind claim

An executable canary binary and dedicated CI workflow are now authored:
- evaluator/src/bin/aerb_sandbox_canary.rs;
- .github/workflows/aerb-blind-sandbox-canary.yml.

The workflow builds the canary as a static musl binary, installs Bubblewrap, and invokes it through the exact command builder. **Its exact-head CI result is pending.** A successful unit test of the argv builder is not a process-isolation pass.

The host runner deliberately sets AERB_EVALUATOR_SECRET to a sentinel; SandboxCommand::process_command clears it before launching prlimit. Candidate mode checks both its own environment and the visible process-1 environment for the sentinel. The CI canary executes the binary and reports:
1. evaluator canary path and host secret path are not visible;
2. candidate input and runtime writes fail with read-only errors;
3. candidate output writes succeed and are confined to the output mount;
4. loopback and external network access are unavailable;
5. host process tree, PID namespace and environment isolation match the declared profile, including no sentinel in the process table;
6. nested user namespace creation is disabled and the in-sandbox user.max_user_namespaces limit is 1;
7. CPU/address-space/process/file limits are observed at their exact boundary;
8. timeout, crash, OOM, output overflow, malformed output and sandbox setup failure all produce explicit failed/invalid campaign states, never a score-as-pass;
9. no host path, evaluator label, seed, hidden-manifest field or canary name appears in candidate-visible argv, environment, filesystem, stdout/stderr or error paths.

A failed canary invalidates the entire campaign. No fallback to running the candidate outside the sandbox is allowed.

## Metadata and adaptive attack suite

Even a successful process sandbox does not make an evaluation blind if candidate inputs leak labels. The separate #7732 attack suite must test:
- metadata-only prediction using ID, scenario, timestamp, path, file order, sizes, source/profile/calibration digests and schema versions;
- candidate state persisted across secret episode order and repeated observations;
- path/URL/URI/error/log traversal and canary discovery;
- attempts to enumerate sibling directories, host mounts, process namespaces, /proc, environment, open FDs and network;
- candidate output attempting to forge or rewrite manifests, report digests, evaluator outcomes or the post-run reveal;
- evaluator restart, crash and partial-campaign recovery, preserving invalid/unresolved denominators.

The acceptance threshold and attack classifiers must be frozen before the blind campaign. Any above-chance signal beyond the preregistered metadata baseline invalidates and replaces the campaign rather than adjusting the score after seeing results.

## Benchmarks and claims

Keep software contract tests, process isolation canaries, metadata leakage attacks, benchmark performance, simulator, HIL and physical robotics evidence as separate tiers.

Prospective target:
- zero evaluator-secret reads, host writes, network egress or nested namespace creation;
- zero successful host-path traversal;
- 100% canary agreement with the evaluator-owned isolation oracle;
- zero score-as-pass on crash, timeout, OOM, output-limit or sandbox setup failure;
- zero candidate-visible hidden-world metadata leakage above the frozen classifier threshold;
- reproducible pre-run commitment and independently verified post-run reveal;
- all episodes accounted for as valid, invalid or unresolved; no denominator deletion.

These are prospective gates, not achieved results. No confidential campaign, blind generalization, SOTA superiority, kernel isolation, simulator/HIL/physical qualification or robot safety certification is claimed by the command-builder slice.

## Existing ownership

- #7732 owns blind corpus leakage and custody.
- PR #7815 supplies random hidden assignment and commit/reveal primitives.
- #7728/#7729 supply candidate-visible typed evidence materialization.
- #7515/#7565 own source-qualified evidence capsules and trace adapters.
- #7695 owns public candidate policy validation.
- #7814/#7788 and #6503 own support freshness / physical action admission, not sandboxing.

This branch advances the process-boundary primitive only; it does not replace the materializer, evaluator custody service, attack harness or independent verifier.


## Statistical power for leakage and safety claims

Do not equate a successful canary or a zero observed leak count with zero leakage probability. For `n` independent trials and zero observed events, the exact one-sided upper confidence limit at confidence `1 - alpha` is `p_upper = 1 - alpha^(1/n)`. Zero leaks in 3,000 independent episodes gives an illustrative 95% upper bound of about 0.1% only for the measured population. If eight world classes need simultaneous 95% bounds under Bonferroni correction, approximately 5,073 zero-event trials per class are required to put each upper bound below 0.1%. These are statistical illustrations, not risk-acceptance criteria.

Leakage and safety evaluations must publish counts by world, metadata family, robot/environment, campaign and candidate version. Treat episodes sharing a campaign, scene, controller or trajectory as clusters; do not count correlated frames or repeated observations as independent trials. Freeze the attack classifier, metadata fields, statistical test, multiple-comparison correction, exclusion policy and thresholds before the blind campaign. Any invalid, crashed, timed-out or unresolved episode remains in the campaign accounting, not silently dropped.
