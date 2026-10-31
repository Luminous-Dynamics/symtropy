# SIM-REL-019 exact model-family trace auditor

This crate replays an observed action/outcome trace independently against a finite family of complete joint stochastic model worlds. Each world keeps its kernel identity for the entire trace and updates its own exact joint belief over mechanism, physical state and sensor state.

The auditor distinguishes exact zero support from a very rare but nonzero event. A world is eliminated at its first zero-likelihood prefix. The family is out-of-declared-family only when every supplied world has been eliminated; the report gives the first prefix at which the model set becomes empty.

A surviving world is not declared true, and the surviving family is not claimed complete. When a trace is impossible under every supplied world, the auditor emits an explicit model-adequacy alarm and returns no normalized empty posterior. Hazard is an explicit terminal symbol; any later action in that world is a contradiction.

The implementation uses exact rational likelihoods, exact conditional joint posterior updates, shared-vocabulary validation through SIM-REL-015 semantics, and fail-closed resource caps. Tests cover per-world survival, staggered model elimination, rare nonzero likelihood, hazard termination, bad IDs, incompatible action costs, and exact posterior normalization.

This is a falsification tool for a declared finite model family, not a method for inventing the true alternative model or certifying physical adequacy.
