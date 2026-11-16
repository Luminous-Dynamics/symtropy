// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic public-development challenge manifest generator.
//!
//! This module generates symbolic synthetic challenge manifests only. Its
//! SplitMix64 stream is reproducible, not cryptographically secure. It must not
//! be used to protect evaluator-held seeds or to assert physical validity.

use ring::digest::{SHA256, digest};
use serde_json::{Value, json};
use std::collections::BTreeSet;

pub const GENERATOR_ID: &str = "robot-stochastic-challenge-generator/0.1";
pub const GENERATOR_SCHEMA_VERSION: &str = "robot-config-mission-challenge-manifest-v0.1";
pub const SEED_ALGORITHM: &str = "SplitMix64-v1-public-development-not-cryptographic";
pub const SEED_DERIVATION: &str = "sha256-domain-separated-first-u64-v1";
pub const CANONICALIZATION: &str =
    "serde_json-1.0.154-default-map-order-compact-utf8-no-trailing-newline-v1";
pub const MAX_CHALLENGES: usize = 10_000;

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn array_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Vec<Value>> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_array()
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let value = digest(&SHA256, bytes);
    let mut output = String::with_capacity(value.as_ref().len() * 2);
    for byte in value.as_ref() {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn canonical_json(value: &Value) -> Result<Vec<u8>, String> {
    serde_json::to_vec(value).map_err(|error| format!("canonical_json_serialize:{error}"))
}

pub fn seed_hex(seed: u64) -> String {
    format!("{seed:016x}")
}

/// Derive a reproducible seed with a domain and index bound into the hash.
/// This provides domain separation, not seed secrecy or entropy.
pub fn derive_seed(root_seed: u64, domain: &str, index: u64) -> u64 {
    let material = format!("LUMINOUS-MIP-SEED-V0.1\0{root_seed:016x}\0{domain}\0{index:016x}");
    let hashed = digest(&SHA256, material.as_bytes());
    let mut first = [0_u8; 8];
    first.copy_from_slice(&hashed.as_ref()[..8]);
    u64::from_be_bytes(first)
}

/// SplitMix64 is used only as a small, fully specified deterministic PRNG for
/// public synthetic development generation. It is not a CSPRNG.
#[derive(Clone, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9e37_79b9_7f4a_7c15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xbf58_476d_1ce4_e5b9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94d0_49bb_1331_11eb);
        value ^ (value >> 31)
    }

    /// Uniform integer in [0, bound), using rejection sampling to avoid
    /// modulo bias. A zero bound is rejected instead of panicking.
    fn bounded(&mut self, bound: u64) -> Option<u64> {
        if bound == 0 {
            return None;
        }
        let threshold = bound.wrapping_neg() % bound;
        loop {
            let candidate = self.next_u64();
            if candidate >= threshold {
                return Some(candidate % bound);
            }
        }
    }
}

#[derive(Clone, Debug)]
struct Allocation {
    index: usize,
    count: usize,
    remainder: u128,
    tie_break: u64,
}

/// Validate profile invariants not expressible by the closed JSON Schema,
/// particularly uniqueness across nested strata and atomic bundle identity.
pub fn distribution_profile_invariants(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    if string_at(profile, &["schema_version"]) != Some("robot-config-mission-distribution-v0.1") {
        errors.push("distribution_profile_schema_version_mismatch".to_owned());
    }
    if string_at(profile, &["generator_id"]) != Some(GENERATOR_ID) {
        errors.push("distribution_profile_generator_id_mismatch".to_owned());
    }
    if string_at(profile, &["visibility"]) != Some("public_synthetic_development") {
        errors.push("distribution_profile_visibility_not_public_development".to_owned());
    }
    if string_at(profile, &["coherence_policy"])
        != Some("atomic_named_profile_bundle_not_independent_axis_sampling")
    {
        errors.push("distribution_profile_coherence_policy_not_atomic".to_owned());
    }

    let Some(strata) = array_at(profile, &["strata"]) else {
        return {
            errors.push("distribution_profile_strata_missing".to_owned());
            errors.sort();
            errors
        };
    };
    if strata.len() < 8 {
        errors.push("distribution_profile_requires_at_least_eight_strata".to_owned());
    }

    let mut stratum_ids = BTreeSet::new();
    let mut bundle_ids = BTreeSet::new();
    for (stratum_index, stratum) in strata.iter().enumerate() {
        let Some(stratum_id) = string_at(stratum, &["stratum_id"]) else {
            errors.push(format!("distribution_stratum_id_missing:{stratum_index}"));
            continue;
        };
        if !stratum_ids.insert(stratum_id.to_owned()) {
            errors.push(format!("distribution_stratum_id_duplicate:{stratum_id}"));
        }
        if stratum
            .get("weight")
            .and_then(Value::as_u64)
            .is_none_or(|weight| weight == 0)
        {
            errors.push(format!("distribution_stratum_weight_invalid:{stratum_id}"));
        }
        if array_at(stratum, &["changed_component_ids"]).is_none_or(Vec::is_empty) {
            errors.push(format!(
                "distribution_changed_components_empty:{stratum_id}"
            ));
        }
        if string_at(stratum, &["coherence_group"]).is_none_or(str::is_empty) {
            errors.push(format!("distribution_coherence_group_missing:{stratum_id}"));
        }

        let Some(bundles) = array_at(stratum, &["bundles"]) else {
            errors.push(format!("distribution_bundles_missing:{stratum_id}"));
            continue;
        };
        if bundles.is_empty() {
            errors.push(format!("distribution_bundles_empty:{stratum_id}"));
        }
        for (bundle_index, bundle) in bundles.iter().enumerate() {
            let Some(bundle_id) = string_at(bundle, &["bundle_id"]) else {
                errors.push(format!(
                    "distribution_bundle_id_missing:{stratum_id}:{bundle_index}"
                ));
                continue;
            };
            if !bundle_ids.insert(bundle_id.to_owned()) {
                errors.push(format!("distribution_bundle_id_duplicate:{bundle_id}"));
            }
            for field in [
                "task_profile",
                "site_profile",
                "odd_profile",
                "severity",
                "factor_profile_ref",
            ] {
                if string_at(bundle, &[field]).is_none_or(str::is_empty) {
                    errors.push(format!(
                        "distribution_bundle_field_missing:{stratum_id}:{bundle_id}:{field}"
                    ));
                }
            }
            if string_at(bundle, &["factor_profile_ref"])
                .is_some_and(|reference| !reference.starts_with("fixture://factor-profile/"))
            {
                errors.push(format!(
                    "distribution_factor_profile_ref_not_synthetic:{stratum_id}:{bundle_id}"
                ));
            }
        }
    }

    errors.sort();
    errors.dedup();
    errors
}

fn allocate_strata(
    strata: &[Value],
    count: usize,
    root_seed: u64,
) -> Result<(Vec<usize>, u64), Vec<String>> {
    let mut total_weight = 0_u64;
    for (index, stratum) in strata.iter().enumerate() {
        let Some(weight) = stratum.get("weight").and_then(Value::as_u64) else {
            return Err(vec![format!("stratum_weight_missing_or_invalid:{index}")]);
        };
        if weight == 0 {
            return Err(vec![format!("stratum_weight_zero:{index}")]);
        }
        total_weight = total_weight
            .checked_add(weight)
            .ok_or_else(|| vec!["stratum_weight_sum_overflow".to_owned()])?;
    }
    if total_weight == 0 {
        return Err(vec!["stratum_weight_sum_zero".to_owned()]);
    }

    let mut allocations = Vec::with_capacity(strata.len());
    let mut allocated = 0_usize;
    for (index, stratum) in strata.iter().enumerate() {
        let weight = stratum["weight"].as_u64().expect("validated above");
        let scaled = (count as u128) * (weight as u128);
        let base = (scaled / total_weight as u128) as usize;
        let remainder = scaled % total_weight as u128;
        let stratum_id = string_at(stratum, &["stratum_id"]).unwrap_or("missing");
        allocations.push(Allocation {
            index,
            count: base,
            remainder,
            tie_break: derive_seed(
                root_seed,
                "stratum-allocation-tie",
                stable_text_index(stratum_id),
            ),
        });
        allocated = allocated.saturating_add(base);
    }

    let remaining = count.saturating_sub(allocated);
    let mut order = allocations
        .iter()
        .enumerate()
        .map(|(index, allocation)| (index, allocation.remainder, allocation.tie_break))
        .collect::<Vec<_>>();
    order.sort_by(|left, right| {
        right
            .1
            .cmp(&left.1)
            .then_with(|| left.2.cmp(&right.2))
            .then_with(|| left.0.cmp(&right.0))
    });
    if remaining > order.len() {
        return Err(vec!["largest_remainder_allocation_inconsistent".to_owned()]);
    }
    for (allocation_index, _, _) in order.into_iter().take(remaining) {
        allocations[allocation_index].count += 1;
    }

    allocations.sort_by_key(|allocation| allocation.index);
    Ok((
        allocations
            .into_iter()
            .map(|allocation| allocation.count)
            .collect(),
        total_weight,
    ))
}

fn stable_text_index(value: &str) -> u64 {
    // FNV-1a is used only as a stable input to SHA-256 domain separation.
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x0000_0100_0000_01b3);
    }
    hash
}

fn shuffle<T>(items: &mut [T], seed: u64) {
    if items.len() < 2 {
        return;
    }
    let mut rng = SplitMix64::new(seed);
    for index in (1..items.len()).rev() {
        let other = rng
            .bounded((index + 1) as u64)
            .expect("shuffle bound is always non-zero") as usize;
        items.swap(index, other);
    }
}

fn challenge_value(
    challenge_index: usize,
    challenge_seed: u64,
    stratum: &Value,
    bundle: &Value,
    stratum_weight: u64,
    total_weight: u64,
    allocated_count: usize,
) -> Result<Value, String> {
    let challenge_id = format!("MIPG-{:06}", challenge_index + 1);
    let stratum_id = string_at(stratum, &["stratum_id"])
        .ok_or_else(|| "challenge_stratum_id_missing".to_owned())?;
    let mutation_family = string_at(stratum, &["mutation_family"])
        .ok_or_else(|| "challenge_mutation_family_missing".to_owned())?;
    let bundle_id = string_at(bundle, &["bundle_id"])
        .ok_or_else(|| "challenge_bundle_id_missing".to_owned())?;
    let task_profile = string_at(bundle, &["task_profile"])
        .ok_or_else(|| "challenge_task_profile_missing".to_owned())?;
    let site_profile = string_at(bundle, &["site_profile"])
        .ok_or_else(|| "challenge_site_profile_missing".to_owned())?;
    let odd_profile = string_at(bundle, &["odd_profile"])
        .ok_or_else(|| "challenge_odd_profile_missing".to_owned())?;
    let severity =
        string_at(bundle, &["severity"]).ok_or_else(|| "challenge_severity_missing".to_owned())?;
    let coherence_group = string_at(stratum, &["coherence_group"])
        .ok_or_else(|| "challenge_coherence_group_missing".to_owned())?;
    let factor_profile_ref = string_at(bundle, &["factor_profile_ref"])
        .ok_or_else(|| "challenge_factor_profile_ref_missing".to_owned())?;
    let changed_component_ids = stratum
        .get("changed_component_ids")
        .cloned()
        .ok_or_else(|| "challenge_changed_component_ids_missing".to_owned())?;
    let bundle_count = array_at(stratum, &["bundles"])
        .ok_or_else(|| "challenge_stratum_bundles_missing".to_owned())?
        .len() as u64;

    let mut challenge = json!({
        "challenge_id": challenge_id,
        "challenge_index": challenge_index,
        "challenge_seed_hex": seed_hex(challenge_seed),
        "seed_derivation_domain": "MIP-CHALLENGE-SEED-V0.1",
        "stratum_id": stratum_id,
        "mutation_family": mutation_family,
        "changed_component_ids": changed_component_ids,
        "factor_bundle_id": bundle_id,
        "factor_profile_ref": factor_profile_ref,
        "coherence_group": coherence_group,
        "task_profile": task_profile,
        "site_profile": site_profile,
        "odd_profile": odd_profile,
        "severity": severity,
        "source_configuration_ref": format!("fixture://generated-config/{challenge_id}/A"),
        "target_configuration_ref": format!("fixture://generated-config/{challenge_id}/B"),
        "stratum_allocation": {
            "weight": stratum_weight,
            "total_weight": total_weight,
            "allocated_count": allocated_count
        },
        "bundle_selection_probability": {
            "numerator": 1,
            "denominator": bundle_count
        }
    });
    let digest = sha256_hex(&canonical_json(&challenge)?);
    challenge
        .as_object_mut()
        .ok_or_else(|| "challenge_not_object".to_owned())?
        .insert("challenge_digest".to_owned(), Value::String(digest));
    Ok(challenge)
}

/// Generate a public synthetic development challenge manifest.
///
/// The manifest's challenge digests cover each challenge without its digest
/// field. The manifest digest covers the complete manifest without its digest
/// field. Serde JSON's default sorted object map order is part of the v0.1
/// canonicalization contract.
/// Recompute a generated manifest from its public profile, root seed and
/// requested count. This rejects edited/reordered output that no longer
/// reproduces under the declared generator version and canonicalization.
pub fn manifest_invariants(manifest: &Value, profile: &Value) -> Vec<String> {
    let Some(seed_text) = string_at(manifest, &["root_seed_hex"]) else {
        return vec!["manifest_root_seed_missing".to_owned()];
    };
    if seed_text.len() != 16 {
        return vec!["manifest_root_seed_format_invalid".to_owned()];
    }
    let Ok(root_seed) = u64::from_str_radix(seed_text, 16) else {
        return vec!["manifest_root_seed_format_invalid".to_owned()];
    };
    let Some(count) = manifest
        .get("requested_count")
        .and_then(Value::as_u64)
        .and_then(|value| usize::try_from(value).ok())
    else {
        return vec!["manifest_requested_count_invalid".to_owned()];
    };
    let expected = match generate_manifest(profile, root_seed, count) {
        Ok(expected) => expected,
        Err(errors) => return errors,
    };
    let actual_bytes = match canonical_json(manifest) {
        Ok(bytes) => bytes,
        Err(error) => return vec![error],
    };
    let expected_bytes = match canonical_json(&expected) {
        Ok(bytes) => bytes,
        Err(error) => return vec![error],
    };
    if actual_bytes != expected_bytes {
        return vec!["manifest_not_reproducible_from_profile_seed_and_count".to_owned()];
    }
    Vec::new()
}

pub fn generate_manifest(
    profile: &Value,
    root_seed: u64,
    challenge_count: usize,
) -> Result<Value, Vec<String>> {
    let errors = distribution_profile_invariants(profile);
    if !errors.is_empty() {
        return Err(errors);
    }
    if challenge_count == 0 || challenge_count > MAX_CHALLENGES {
        return Err(vec![format!(
            "challenge_count_out_of_range:1..={MAX_CHALLENGES}"
        )]);
    }
    let strata = array_at(profile, &["strata"]).expect("profile invariant checked");
    let (allocations, total_weight) = allocate_strata(strata, challenge_count, root_seed)?;

    let mut challenges = Vec::with_capacity(challenge_count);
    let mut next_index = 0_usize;
    for (stratum_index, stratum) in strata.iter().enumerate() {
        let allocated_count = allocations[stratum_index];
        let stratum_weight = stratum["weight"]
            .as_u64()
            .expect("profile invariant checked");
        let bundles = array_at(stratum, &["bundles"]).expect("profile invariant checked");
        for _ in 0..allocated_count {
            let challenge_seed =
                derive_seed(root_seed, "MIP-CHALLENGE-SEED-V0.1", next_index as u64);
            let bundle_seed = derive_seed(challenge_seed, "MIP-ATOMIC-BUNDLE-CHOICE-V0.1", 0);
            let mut rng = SplitMix64::new(bundle_seed);
            let bundle_index = rng
                .bounded(bundles.len() as u64)
                .expect("validated stratum contains at least one bundle")
                as usize;
            let challenge = challenge_value(
                next_index,
                challenge_seed,
                stratum,
                &bundles[bundle_index],
                stratum_weight,
                total_weight,
                allocated_count,
            )
            .map_err(|error| vec![error])?;
            challenges.push(challenge);
            next_index += 1;
        }
    }

    shuffle(
        &mut challenges,
        derive_seed(
            root_seed,
            "MIP-CHALLENGE-ORDER-V0.1",
            challenge_count as u64,
        ),
    );

    let stratum_counts = strata
        .iter()
        .enumerate()
        .map(|(index, stratum)| {
            json!({
                "stratum_id": string_at(stratum, &["stratum_id"]).unwrap_or("missing"),
                "weight": stratum["weight"],
                "total_weight": total_weight,
                "allocated_count": allocations[index],
                "bundle_count": array_at(stratum, &["bundles"]).map_or(0, Vec::len)
            })
        })
        .collect::<Vec<_>>();

    let profile_id = string_at(profile, &["profile_id"]).unwrap_or("unknown");
    let profile_digest = sha256_hex(&canonical_json(profile).map_err(|error| vec![error])?);
    let mut manifest = json!({
        "schema_version": GENERATOR_SCHEMA_VERSION,
        "generator_id": GENERATOR_ID,
        "seed_algorithm": SEED_ALGORITHM,
        "seed_derivation": SEED_DERIVATION,
        "canonicalization": CANONICALIZATION,
        "visibility": "public_synthetic_development",
        "distribution_profile_id": profile_id,
        "distribution_profile_sha256": profile_digest,
        "root_seed_hex": seed_hex(root_seed),
        "requested_count": challenge_count,
        "allocation_method": "largest_remainder_then_seeded_shuffle",
        "bundle_selection_method": "uniform_within_stratum_atomic_bundle",
        "total_stratum_weight": total_weight,
        "stratum_counts": stratum_counts,
        "rejected_draw_count": 0,
        "rejected_draw_reason_counts": [],
        "challenges": challenges,
        "nonclaims": [
            "Generated challenge records are synthetic symbolic fixtures and do not define a physical test procedure.",
            "Named factor profile references are not calibrated physical ranges or a validated physics model.",
            "SplitMix64 and the public deterministic seed derivation are not cryptographically secure and do not protect evaluator-held seeds.",
            "Manifest and challenge digests establish byte identity under the declared canonicalization only; they do not establish truth, provenance, custody, physical authenticity or safety."
        ]
    });
    let manifest_digest = sha256_hex(&canonical_json(&manifest).map_err(|error| vec![error])?);
    manifest
        .as_object_mut()
        .expect("manifest constructed as object")
        .insert("manifest_digest".to_owned(), Value::String(manifest_digest));
    Ok(manifest)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile() -> Value {
        let root = crate::repository_root().join("research/robot-config-mission-impact-v0-1");
        crate::read_json(&root.join("challenge-distribution-v0.1.json"))
            .expect("distribution profile parses")
    }

    #[test]
    fn distribution_profile_has_unique_strata_and_atomic_bundles() {
        let value = profile();
        assert!(distribution_profile_invariants(&value).is_empty());
    }

    #[test]
    fn same_profile_seed_and_count_produce_identical_canonical_manifest() {
        let value = profile();
        let first = generate_manifest(&value, 0x0123_4567_89ab_cdef, 64).unwrap();
        let second = generate_manifest(&value, 0x0123_4567_89ab_cdef, 64).unwrap();
        assert_eq!(
            canonical_json(&first).unwrap(),
            canonical_json(&second).unwrap()
        );
        assert_eq!(first["manifest_digest"], second["manifest_digest"]);
    }

    #[test]
    fn different_root_seeds_change_manifest_identity() {
        let value = profile();
        let first = generate_manifest(&value, 0x0123_4567_89ab_cdef, 64).unwrap();
        let second = generate_manifest(&value, 0xfedc_ba98_7654_3210, 64).unwrap();
        assert_ne!(first["manifest_digest"], second["manifest_digest"]);
    }

    #[test]
    fn exact_largest_remainder_allocation_covers_all_equal_weight_strata() {
        let value = profile();
        let strata = array_at(&value, &["strata"]).unwrap();
        let (counts, total_weight) = allocate_strata(strata, 19, 0x55aa).unwrap();
        assert_eq!(total_weight, 8);
        assert_eq!(counts.iter().sum::<usize>(), 19);
        assert_eq!(
            counts.iter().max().unwrap() - counts.iter().min().unwrap(),
            1
        );
    }

    #[test]
    fn each_generated_challenge_binds_one_atomic_coherence_bundle() {
        let value = profile();
        let manifest = generate_manifest(&value, 0x3141_5926_5358_9793, 128).unwrap();
        let challenges = array_at(&manifest, &["challenges"]).unwrap();
        let strata = array_at(&value, &["strata"]).unwrap();
        assert_eq!(challenges.len(), 128);
        for challenge in challenges {
            let stratum = strata
                .iter()
                .find(|stratum| {
                    string_at(stratum, &["stratum_id"]) == string_at(challenge, &["stratum_id"])
                })
                .unwrap();
            assert_eq!(
                string_at(stratum, &["coherence_group"]),
                string_at(challenge, &["coherence_group"])
            );
            let bundle = array_at(stratum, &["bundles"])
                .unwrap()
                .iter()
                .find(|bundle| {
                    string_at(bundle, &["bundle_id"]) == string_at(challenge, &["factor_bundle_id"])
                })
                .unwrap();
            for field in [
                "task_profile",
                "site_profile",
                "odd_profile",
                "severity",
                "factor_profile_ref",
            ] {
                assert_eq!(string_at(bundle, &[field]), string_at(challenge, &[field]));
            }
        }
    }

    #[test]
    fn bounded_sampling_rejects_zero_and_never_exceeds_bound() {
        let mut rng = SplitMix64::new(0xdec0_de01);
        assert_eq!(rng.bounded(0), None);
        for bound in [1_u64, 2, 3, 7, 16, 257, u32::MAX as u64] {
            for _ in 0..10_000 {
                assert!(rng.bounded(bound).unwrap() < bound);
            }
        }
    }

    #[test]
    fn seed_domains_are_separated_and_replayable() {
        let root = 0x0123_4567_89ab_cdef;
        assert_eq!(
            derive_seed(root, "challenge", 7),
            derive_seed(root, "challenge", 7)
        );
        assert_ne!(
            derive_seed(root, "challenge", 7),
            derive_seed(root, "bundle", 7)
        );
        assert_ne!(
            derive_seed(root, "challenge", 7),
            derive_seed(root, "challenge", 8)
        );
    }

    #[test]
    fn public_golden_vector_matches_exact_manifest_digest_and_strata() {
        let root = crate::repository_root().join("research/robot-config-mission-impact-v0-1");
        let profile = profile();
        let golden = crate::read_json(&root.join("golden-vectors-v0.1.json"))
            .expect("golden vector artifact parses");
        let vector = &golden["vectors"][0];
        let seed = u64::from_str_radix(
            string_at(vector, &["seed_hex"]).expect("golden seed exists"),
            16,
        )
        .expect("golden seed is valid hexadecimal");
        let count = vector["challenge_count"]
            .as_u64()
            .expect("golden challenge count exists") as usize;
        let manifest = generate_manifest(&profile, seed, count).unwrap();
        assert_eq!(
            string_at(&manifest, &["manifest_digest"]),
            string_at(vector, &["expected_manifest_sha256"])
        );
        assert_eq!(
            string_at(&manifest, &["distribution_profile_sha256"]),
            string_at(vector, &["profile_sha256"])
        );
        let expected_counts = array_at(vector, &["expected_stratum_counts"]).unwrap();
        let actual_counts = array_at(&manifest, &["stratum_counts"]).unwrap();
        assert_eq!(actual_counts.len(), expected_counts.len());
        for expected in expected_counts {
            let id = string_at(expected, &["stratum_id"]).unwrap();
            let expected_count = expected["count"].as_u64().unwrap();
            let actual = actual_counts
                .iter()
                .find(|row| string_at(row, &["stratum_id"]) == Some(id))
                .expect("every golden stratum must exist in the manifest");
            assert_eq!(actual["allocated_count"].as_u64(), Some(expected_count));
        }
    }

    #[test]
    fn manifest_invariants_accept_generated_manifest() {
        let value = profile();
        let manifest = generate_manifest(&value, 0x1234, 16).unwrap();
        assert!(manifest_invariants(&manifest, &value).is_empty());
    }

    #[test]
    fn manifest_invariants_reject_mutated_bundle_or_digest() {
        let value = profile();
        let mut manifest = generate_manifest(&value, 0x1234, 16).unwrap();
        manifest["challenges"][0]["factor_bundle_id"] =
            Value::String("unregistered-bundle".to_owned());
        assert_eq!(
            manifest_invariants(&manifest, &value),
            vec!["manifest_not_reproducible_from_profile_seed_and_count".to_owned()]
        );
    }

    #[test]
    fn manifest_digest_is_canonical_and_recomputable() {
        let value = profile();
        let manifest = generate_manifest(&value, 0x1234, 16).unwrap();
        let expected = string_at(&manifest, &["manifest_digest"]).unwrap();
        let mut unsigned = manifest.clone();
        unsigned.as_object_mut().unwrap().remove("manifest_digest");
        assert_eq!(
            expected,
            sha256_hex(&canonical_json(&unsigned).unwrap()).as_str()
        );
    }

    #[test]
    fn count_bounds_fail_closed() {
        let value = profile();
        assert!(generate_manifest(&value, 1, 0).is_err());
        assert!(generate_manifest(&value, 1, MAX_CHALLENGES + 1).is_err());
    }
}
