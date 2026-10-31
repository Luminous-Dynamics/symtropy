# SIM-REL-020 counterexample-guided model-family refinement

This crate turns an exact out-of-family trace alarm into a controlled append-only model-family revision.

A revision is accepted only when the original complete model family is contradicted by the supplied trace, a new candidate ID is unique, its complete joint stochastic kernel validates under the shared semantic contract, and the candidate assigns exact nonzero likelihood to the entire counterexample trace. The candidate is appended; no prior world is removed, modified or reordered.

The revision receipt stores the old ordered world IDs, candidate ID, trace, optional human rationale, exact candidate trace likelihood, and both the original and expanded family audits. `verify_model_family_revision` independently recomputes both audits and rejects any tampering.

This is model-set expansion, not model selection. A candidate's ability to explain one counterexample does not make it true, scientifically plausible outside its supplied kernel, or a complete account of the process. Human review and external evidence remain necessary.
