# Claim Admissibility V0.2.1 — Evidence Vectors, Computed Decisions

**Issue:** [#3526](https://github.com/Luminous-Dynamics/symtropy/issues/3526)  
**Validator:** [Rust package](../../../../crates/apps/symtropy-construction-evidence-validator)  
**Status:** draft research implementation; no physical qualification

## Core change

Evidence origin, independence, metrology, provenance/authentication, custody, adjudication, ODD coverage and replication are separate dimensions. Profiles are claim-specific; there is no global evidence ladder or composite confidence score.

The packet contains source facts and the requested scope, **not a decision or reason codes**. The validator computes each claim's outcome. This prevents a submitter from self-awarding admissibility.

## Validator trust boundary

- Closed Draft 2020-12 packet/profile/result schemas.
- Schemas and the requirements catalog are embedded into the Rust binary and SHA-256 pinned in source.
- No runtime schema/profile override and no remote JSON Schema reference resolution.
- Each claim binds the exact bytes of the whole frozen requirements catalog.
- Output commits to packet bytes, all schema/profile digests, validator name and version.
- Claim-specific outcomes are `admissible_for_named_claim`, `inadmissible_for_named_claim`, `indeterminate`, or `not_evaluated`.

The pinned profile digest prevents a CLI caller from substituting a weaker rule document. It does not prove the executable itself came from an approved commit. Production trust additionally requires reproducible build provenance, a verified release signature, and policy-controlled deployment.

## Evidence semantics

A synthetic trace can support a synthetic software-contract claim, not a physical claim. A bench measurement can support a bounded bench stopping-distance claim, not field safety. Field human-detection requires independent target ground truth, metrology, custody, site/ODD-stratified sampling, and claim-level independent adjudication. Productivity requires accepted-output units and an explicit matched baseline assessment. Cross-site generalization requires independent site **and** configuration holdouts.

Counts are never summed across evidence units in this version. Repeated trials, sites, and configurations are tracked as distinct replication dimensions. A single label such as `independent_site_replication` cannot silently satisfy every form of replication.

## Composed ownership

This work reuses rather than duplicates:
- #3522 physical trial receipt and raw measurement identity;
- #3505/#3506 physical validation protocol;
- #1607 surveying/metrology;
- #1300 blind challenge custody;
- Symthaea #4918 generic evidence dependency/common-mode lineage;
- #3317 snapshot safety envelope and #3499 temporal evidence;
- #1595 scenario identity and #1172 replay.

## Status and limits

The fixture identifiers, hashes, assessor references and measurements are invented. Schema and validator tests qualify only the software contract. This does not authenticate external actors or data, establish physical truth, certify a robot, or authorize a task. Field and cross-site claims remain unevaluated until the independent physical evidence exists.


## Exact-byte cross-platform policy

The pinned schema/profile files are checked out as LF via the repository `.gitattributes` rule for this research directory. The focused workflow also runs when `.gitattributes` changes, so Windows verifies the same bytes as Unix. The earlier Windows run exposed CRLF conversion and failed the embedded digest check; the fix is to preserve the committed bytes, not normalize or bypass digest verification in the validator. The re-run after this policy change is the required acceptance evidence.
