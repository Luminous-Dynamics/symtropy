# Scan-to-BIM candidate/evaluator isolation threat model v0.1

**Status:** design contract only; no candidate process sandbox has been implemented or qualified.  
**Owner:** #4116 SCAN-BIM-REPLAY-001.  
**Current implementation:** PR #4151, replay manifest v0.2.  
**Date:** 2026-11-08.

## 1. Purpose

Prevent a candidate reconstruction pipeline from learning evaluator-only references, ground-truth labels, holdout membership, official scores or future-scene artifacts before its output is frozen and committed.

A valid external replay must make leakage difficult by construction. A manifest field that says candidate and evaluator paths are separate is not a security boundary.

## 2. Protected assets and failure model

Protected assets include reference IFC/BIM, reference surfaces and labels; hidden holdout assignments and building-group mappings; official score tables, evaluator masks and independent checkpoints; restricted scans and derived reference geometry; and candidate output digests before evaluation reveal.

Treat the candidate as potentially buggy or adversarial. It may attempt to:
- open guessed, absolute or traversal-based evaluator paths;
- follow symlinks, hard links, procfs magic links or mount aliases;
- read environment variables, logs, CI metadata, shell history, host workspace or sibling runs;
- use outbound network access to fetch holdout/reference data;
- alter an input or executable after digest verification;
- write outside its declared output directory;
- leave a child process running after timeout;
- smuggle evaluator content through caches, temporary files, shared volumes or logs.

The threat model does not assume a malicious kernel, hypervisor, CI administrator or organizational custodian. Those require separate trust and custody controls.

## 3. Current qualification boundary

The current Rust v0.2 verifier provides schema/manifest validation, typed candidate and evaluator namespaces, per-file SHA-256 recomputation below distinct local roots, path traversal rejection, observed symlink rejection, and explicit run/evaluation-profile metadata.

It does not execute the candidate command; isolate a candidate process; remove evaluator mounts, credentials or metadata from a candidate; deny network access; guarantee race-free file opening against a concurrent malicious writer; prove that a source commit produced the supplied executable; or establish independent organizational evaluator custody.

Root separation inside one process is namespace separation, not a sandbox. Preserve this language in user-facing docs and receipts.

## 4. Required trust domains

### A. Orchestrator and evidence controller

The orchestrator may resolve authorized dataset access, verify candidate build/model/configuration and inputs, stage candidate-only and evaluator-only roots, launch workers, freeze outputs, start the evaluator after output commitment, and create evidence receipts using the existing #1226 owner.

It must not emit reference geometry, evaluator-derived bounding boxes, labels, holdout identifiers or reference file metadata into candidate-visible logs.

### B. Candidate worker

The candidate runs under a separate non-privileged identity with an explicitly constructed filesystem and network view.

Read access is limited to the declared partial observations, authorized prior model assets, sensor/registration/calibration inputs, and exact executable/model/configuration artifacts.

Write access is limited to a dedicated output directory and bounded temporary storage. Inputs are read-only.

The candidate must not have:
- evaluator or reference roots mounted or addressable;
- ground-truth labels, hidden holdout membership or official scores;
- access to host workspace, home directory, CI tokens, credentials or sibling runs;
- write access to source staging or executable/model/configuration files;
- outbound network access by default.

Candidate descendants must be terminated before evaluator startup. Candidate outputs must be frozen and hashed from a separate controller context.

### C. Evaluator worker

The evaluator runs under a distinct identity/process and can read evaluator-only references plus committed candidate outputs. It cannot mutate candidate outputs. It recomputes candidate-output hashes before scoring and fails closed on any mismatch.

Organizationally independent custody or a second organization may be needed for high-stakes blind studies. OS isolation alone does not prove organizational independence.

## 5. Linux implementation direction

Start with an auditable launcher adapter, not a claim that a manifest library is itself a sandbox.

A candidate worker profile should include:
- an unprivileged UID/GID, no supplementary groups and no capabilities;
- private mount namespace and private mount propagation;
- a minimal filesystem root, with read-only binds for candidate inputs/runtime and one writable output bind;
- no evaluator bind, no workspace or secret bind;
- user, PID, IPC and network namespaces where supported;
- network namespace with no external interface by default;
- no_new_privs, process/resource limits, timeout and complete process-group teardown;
- a clean environment allowlist;
- runtime, namespace, mount, UID/GID, command and launcher-version details recorded in the run receipt.

Bubblewrap is one candidate primitive: its documentation describes unprivileged user namespaces, a fresh mount namespace, explicit filesystem visibility, read-only mounts, and optional PID/network namespaces. It is not a one-flag substitute for a threat-modelled mount allowlist or independent review. See https://github.com/containers/bubblewrap and https://manpages.debian.org/trixie/bubblewrap/bwrap.1.en.html.

Set no_new_privs before executing the candidate so exec cannot grant setuid/setgid or file-capability privileges that were not already available. This flag is inherited and irreversible, but it is only one hardening layer, not a sandbox by itself. See https://docs.kernel.org/userspace-api/no_new_privs.html.

For race-resistant local artifact resolution, use descriptor-relative opens rooted at a pre-opened directory descriptor. Linux openat2 supports RESOLVE_BENEATH, RESOLVE_NO_SYMLINKS, RESOLVE_NO_MAGICLINKS and optionally RESOLVE_NO_XDEV. These constrain path resolution at open time and are materially stronger than canonicalize-then-open. openat2 was introduced in Linux 5.6; unsupported kernels must fail closed for the hardened profile. See https://www.man7.org/linux/man-pages/man2/openat2.2.html.

Do not silently fall back to a weaker path resolver or a process running in the same host filesystem view. Return a typed UnsupportedIsolationProfile or SandboxSetupFailed result.

## 6. Mandatory negative-control canaries

The isolation harness must run a deliberately hostile test worker whose purpose is to attempt forbidden access. A passing manifest that never attempts an access is not evidence of isolation.

Minimum canaries:
1. Read a unique evaluator-only file by its host absolute path.
2. Read that same file via a guessed sandbox path.
3. Traverse parent directories from every candidate input/output root.
4. Follow symlink and procfs magic-link escapes to evaluator-only files.
5. Enumerate host workspace, home, sibling runs, environment and CI credential locations.
6. Reach a predeclared network test endpoint.
7. Write outside the declared output/temp directories.
8. Modify a read-only input, executable, model or configuration artifact.
9. Fork a child that attempts to survive timeout and worker teardown.
10. After candidate termination, prove the evaluator can read only the committed candidate outputs and independently re-hash them.

Any forbidden access that succeeds is a hard isolation failure. Record the attempted operation, observed denial, worker/runtime profile and exact source/build digest without disclosing evaluator data.

## 7. Receipt sequence

1. Freeze manifest, dataset/split lineage, candidate source/build/model/configuration identity and evaluation profile.
2. Verify candidate runtime and input bytes.
3. Stage candidate-visible files into a read-only candidate view; keep evaluator references outside that view.
4. Establish the OS security profile and record the realized namespaces, mounts, UID/GID, network state, environment policy and launcher version.
5. Run canaries and require every forbidden access to be denied.
6. Execute the candidate exactly once under the frozen profile.
7. Terminate and reap all candidate descendants; revoke mounts and credentials.
8. Hash candidate outputs from the controller; freeze the artifact list and digests.
9. Commit a run receipt through the existing evidence/custody owner.
10. Start the evaluator with the immutable candidate outputs and evaluator-only references.
11. Re-hash both sides; score under the bound metric/tolerance profile and report all per-scene/per-building denominators, curves, unknowns and exclusions.
12. Preserve failures as evidence; a failed or unsupported sandbox cannot produce a benchmark PASS.

Do not expose evaluator filenames, counts, bounding boxes, masks or detailed scoring errors to the candidate before output commitment.

## 8. Software supply-chain provenance

The current manifest binds a source-commit string and actual runtime file digest separately. It does not prove that the source commit produced that executable.

For stronger provenance, attach a signed build attestation whose subject digest matches the candidate executable and whose build definition identifies source revision, builder identity, toolchain, dependencies and parameters. SLSA provenance is designed to describe where, when and how artifacts were built and to support verification/rebuild. Adopt the current SLSA provenance predicate only after reviewing the exact version and builder trust model: https://slsa.dev/spec/v1.2/provenance.

This is an additional integrity chain; it does not replace candidate/evaluator isolation or prove model quality.

## 9. Acceptance criteria for a future sandbox implementation

- [ ] Candidate process runs under a separate non-privileged identity.
- [ ] Candidate cannot enumerate or read evaluator roots or canaries.
- [ ] Candidate cannot read host workspace, CI secrets or sibling runs.
- [ ] Network is denied by default and a network canary proves it.
- [ ] Inputs and runtime artifacts are read-only.
- [ ] Writes are confined to declared output and bounded temporary storage.
- [ ] All descendants are terminated; mounts and credentials are revoked before evaluator startup.
- [ ] The exact realized sandbox profile is bound to the run receipt.
- [ ] Evaluator independently re-hashes frozen candidate outputs.
- [ ] Unsupported platforms/profiles fail closed; no silent fallback.
- [ ] Path resolution is descriptor-relative and no-follow, or equivalent race-resistant.
- [ ] Canary suite proves denial of every forbidden access class.
- [ ] No restricted datasets, evaluator canaries or references are uploaded to CI artifacts or public logs.
- [ ] Organizational independence is claimed only when separately evidenced.

## Claim ceiling

Until these criteria pass, call the system a local artifact-integrity verifier with declared candidate/evaluator namespaces. Do not call it a blind benchmark harness, OS sandbox or independent evaluator. Even after this gate passes, it does not prove organizational independence, prior training-set cleanliness, reference validity or engineering acceptance.
