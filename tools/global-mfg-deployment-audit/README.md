# Global Manufacturing Deployment Audit

Standalone, read-only Rust tool for the Global Manufacturing Deployment Profile v0.1 contract.

## Purpose

The tool validates two things:

1. The deployment corpus is a valid instance of its Draft 2020-12 JSON Schema, and the schema itself is valid against the Draft 2020-12 meta-schema.
2. The corpus satisfies the additional Luminous deployment invariants: five independent jurisdiction dimensions are present; synthetic jurisdiction IDs are used in the synthetic fixture; all regional profiles preserve the same product family/revision; source and obligation records remain explicitly synthetic/unknown; market approval and production authorization remain false; and the complete frozen negative-control catalogue is present.

It is a structural and semantic contract audit. It does not interpret law, infer legal applicability, calculate customs origin, validate tariff eligibility, authenticate a supplier, assess a physical site, verify a test laboratory, certify a product, grant market approval or authorize production.

## Run locally

From the repository root:

```bash
cargo generate-lockfile --manifest-path tools/global-mfg-deployment-audit/Cargo.toml
cargo fmt --manifest-path tools/global-mfg-deployment-audit/Cargo.toml -- --check
cargo test --locked --manifest-path tools/global-mfg-deployment-audit/Cargo.toml
cargo run --locked --quiet --manifest-path tools/global-mfg-deployment-audit/Cargo.toml -- \
  schemas/global/global_mfg_deployment_profile_v0_1.schema.json \
  fixtures/global/global_mfg_deployment_profiles_v0_1.json
cargo clippy --locked --manifest-path tools/global-mfg-deployment-audit/Cargo.toml --all-targets -- -D warnings
```

The direct JSON Schema validator dependency is pinned in `Cargo.toml`. The workflow generates and hashes the exact `Cargo.lock` used for its test/audit run; the lock is included in its uploaded receipt.

## Output and claim ceiling

A successful report uses disposition `STRUCTURALLY_VALID_SYNTHETIC_ONLY` and always sets:

- `legal_or_market_approval = false`
- `physical_site_qualification = false`
- `production_authorization = false`

The four initial profiles are synthetic operating archetypes, not country representations. Their identifiers, source obligations, jurisdictions and values do not establish any real market, facility or supplier capability. The synthetic negative-control catalogue specifies expected rejection/abstention cases; this version validates the corpus contract and its required controls, not a real legal database.

## Evidence required for real regional profiles

A source-backed country/site/market profile requires exact authoritative instruments, editions, publication and application dates, actor/product/operation scope, origin/transformation evidence, local expert review, and all relevant site/product/quality/safety/environment permissions. It must be separately versioned from the synthetic corpus. A profile audit cannot replace the responsible economic operator, regulator, technical service, inspector or competent local professional.
