# Kitchen Capability Admission V1

Status: draft implementation and synthetic conformance cases. No physical appliance or module is qualified by this package.

This module composes the existing RecipeTaskGraphV1, KitchenPlanV1, action ontology, power-aware scheduling and KitchenModuleCapabilityV1. It does not create a second planner, module registry, evidence ledger or safety authority.

## What it checks

Each recipe task must have one explicit binding to:
- one exact module ID and revision;
- one exact capability ID on that revision;
- an explicit action-ontology group that matches the capability;
- declared input and output forms;
- a scheduled resource already present in the task;
- batch mass within any declared mass limits, where quantities are explicit mg/g/kg;
- a task duration within the declared maximum continuous run time;
- an explicit task power profile within that exact module revision's continuous, peak and surge limits.

At request level, the evaluator also requires the power-plan task graph to exactly equal the recipe graph's mapped scheduling plan, and independently revalidates the submitted schedule with the existing household power-envelope verifier. This covers aggregate household background load, simultaneous task demand, continuous real-power limits, peak limits and surge duration.

All mass comparisons use checked integer arithmetic at nanogram resolution. Volume and count units are not converted to mass. Missing mass, power traces, module power limits, maximum runtimes, or non-comparable units remain indeterminate. A large but representable integer quantity is compared exactly against declared bounds; the evaluator does not label a valid arithmetic result as overflow. Only a genuinely unrepresentable checked sum would be indeterminate. Known mismatches, invalid evidence structures, and exceeded physical declaration bounds are blocked.

## Dispositions

- `within_declared_envelope`: submitted declarations are internally consistent for this binding and the household schedule passes the existing verifier.
- `indeterminate`: a required declaration or comparison bound is missing or cannot be compared. This never means safe or compatible.
- `blocked`: a known mismatch, invalid record, duplicate or exceeded bound exists.

A report is admitted only if all tasks are within their declared envelope and there are no request-level issues. No disposition grants device-control permission.

## Tests

Run: cargo test -p symtropy-kitchen-planner --all-targets

Tests use existing synthetic recipe/module fixtures, simulated power profiles and the versioned admission case manifest. Exact-head CI is required; the local authoring session has not compiled or executed this crate.

## Claim ceiling

The result is a structural and declared-envelope consistency check. It does not prove mechanical fitment, interoperability, measured electrical behavior, food-contact suitability, sanitation efficacy, food safety, electrical/mechanical safety, legal compliance, physical performance, product superiority or human usability.
