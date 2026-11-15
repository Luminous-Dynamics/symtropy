# AeroWeft Evidence Lifecycle Closure Oracle

A dependency-free, synthetic-first Rust contract checker for the Track T → U → V → externally authorized test → independent adjudication → new Track T qualification lifecycle.

It validates explicit transition receipts, exact subject and model-scope bindings, parent payload references, graph closure, authorization/design binding, execution identity, independent adjudication and fresh confirmatory manifests.

It does **not** orchestrate experiments, authorize tests, run simulators or aircraft, authenticate signatures or issuer authority, validate physical models, or grant flight/dispatch/separation permissions. P/Q owner dispositions are supplied as inputs and are not cryptographically verified by this crate.

This is an offline composition oracle. It composes AeroWeft #5304/#5337/#5351/#5362/#5371/#5394 and Mycelix's existing trust/event owners; it is not a replacement for any of them.
