# AeroWeft Causal Receipt Binding Oracle

A dependency-free Rust structural verifier for synthetic, subject-scoped evidence receipt graphs.

It checks exact subject identity, typed parent edges, payload-digest references, temporal causality, validity-at-evaluation, graph acyclicity and replay/fork conditions. It does not recalculate the domain claims made by PNT, surveillance, reachability, conflict or conformance owners.

The strongest positive output is `CAUSALLY_BOUND_ONLY`. Byte integrity, signature verification, issuer authority and revocation evaluation are explicitly `NOT_EVALUATED`. A SHA-256-shaped string is only a syntactic identifier here; it is not proof of authentic bytes, a valid signature, authorized issuer or physical truth.

All receipt IDs, timestamps, scopes and digests in the tests are fictional.
