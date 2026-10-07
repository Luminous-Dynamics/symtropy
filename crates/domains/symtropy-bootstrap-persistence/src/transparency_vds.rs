// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Service-independent RFC 9162-style Merkle consistency verification.
//!
//! This module intentionally contains only VDS tree mechanics. It does not decide
//! whether a checkpoint is authoritative, trusted, fresh, or admissible for execution.
//! Those decisions remain in the transparency policy/admission layer.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub const MERKLE_CONSISTENCY_SCHEMA_VERSION: u32 = 1;
pub const MERKLE_CONSISTENCY_ALGORITHM: &str = "SHA-256-RFC9162-MERKLE-CONSISTENCY-v1";
pub const MAX_MERKLE_CONSISTENCY_PROOF_HASHES: usize = 63;
pub const MERKLE_INCLUSION_SCHEMA_VERSION: u32 = 1;
pub const MERKLE_INCLUSION_ALGORITHM: &str = "SHA-256-RFC9162-MERKLE-INCLUSION-v1";
pub const MAX_MERKLE_INCLUSION_PROOF_HASHES: usize = 63;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleConsistencyProofV1 {
    hashes: Vec<String>,
}

impl MerkleConsistencyProofV1 {
    #[must_use]
    pub fn empty() -> Self {
        Self { hashes: Vec::new() }
    }

    pub fn new(hashes: Vec<String>) -> Result<Self, MerkleVdsError> {
        if hashes.len() > MAX_MERKLE_CONSISTENCY_PROOF_HASHES {
            return Err(MerkleVdsError::Invalid(
                "Merkle consistency proof exceeds 63 hashes".to_string(),
            ));
        }
        for hash in &hashes {
            decode_hex::<32>(hash)?;
        }
        if hashes.is_empty() {
            return Err(MerkleVdsError::Invalid(
                "Merkle consistency proof must contain at least one hash".to_string(),
            ));
        }
        Ok(Self { hashes })
    }

    #[must_use]
    pub fn hashes(&self) -> &[String] {
        &self.hashes
    }

    pub fn validate_basic(&self) -> Result<(), MerkleVdsError> {
        if self.hashes.len() > MAX_MERKLE_CONSISTENCY_PROOF_HASHES {
            return Err(MerkleVdsError::Invalid(
                "Merkle consistency proof exceeds 63 hashes".to_string(),
            ));
        }
        for hash in &self.hashes {
            decode_hex::<32>(hash)?;
        }
        Ok(())
    }

    /// Verify an RFC 9162-style consistency proof for two non-empty tree heads.
    ///
    /// The algorithm is deliberately SHA-256 specific in this v1 profile. It uses
    /// the RFC 9162 leaf/node domain separation rules and the RFC 9162 consistency
    /// path verification algorithm. A future C2SP adapter can translate its wire
    /// checkpoint and proof representation into this semantic object.
    pub fn verify_sha256(
        &self,
        first_size: u64,
        first_root: &str,
        second_size: u64,
        second_root: &str,
    ) -> Result<(), MerkleVdsError> {
        self.validate_basic()?;
        if first_size == 0 || second_size == 0 {
            return Err(MerkleVdsError::Invalid(
                "Merkle consistency verification requires non-empty tree heads".to_string(),
            ));
        }
        if first_size >= second_size {
            return Err(MerkleVdsError::Invalid(
                "first Merkle tree size must be smaller than second tree size".to_string(),
            ));
        }

        let first_hash = decode_hex::<32>(first_root)?;
        let second_hash = decode_hex::<32>(second_root)?;

        let mut path = Vec::with_capacity(self.hashes.len() + 1);
        for hash in &self.hashes {
            path.push(decode_hex::<32>(hash)?);
        }

        // RFC 9162 section 2.1.4.2: for an exact power-of-two first tree,
        // prepend its advertised root to the consistency path.
        if first_size.is_power_of_two() {
            path.insert(0, first_hash);
        }

        if path.is_empty() {
            return Err(MerkleVdsError::Invalid(
                "Merkle consistency verification path became empty".to_string(),
            ));
        }

        let mut fn_index = first_size - 1;
        let mut sn_index = second_size - 1;

        // RFC 9162 step 4.
        while fn_index & 1 == 1 {
            fn_index >>= 1;
            sn_index >>= 1;
        }

        let mut first_reconstructed = path[0];
        let mut second_reconstructed = path[0];

        for current in path.iter().skip(1) {
            if sn_index == 0 {
                return Err(MerkleVdsError::Invalid(
                    "Merkle consistency proof contains trailing data".to_string(),
                ));
            }

            if fn_index & 1 == 1 || fn_index == sn_index {
                first_reconstructed = merkle_parent_sha256(current, &first_reconstructed);
                second_reconstructed = merkle_parent_sha256(current, &second_reconstructed);

                if fn_index & 1 == 0 {
                    while fn_index != 0 && fn_index & 1 == 0 {
                        fn_index >>= 1;
                        sn_index >>= 1;
                    }
                }
            } else {
                second_reconstructed = merkle_parent_sha256(&second_reconstructed, current);
            }

            fn_index >>= 1;
            sn_index >>= 1;
        }

        if sn_index != 0 || first_reconstructed != first_hash || second_reconstructed != second_hash
        {
            return Err(MerkleVdsError::ProofMismatch);
        }

        Ok(())
    }
}

/// An RFC 9162 Merkle inclusion proof for one leaf in a tree head.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct MerkleInclusionProofV1 {
    schema_version: u32,
    algorithm: String,
    leaf_index: u64,
    leaf_hash: String,
    hashes: Vec<String>,
}

impl MerkleInclusionProofV1 {
    pub fn new(
        leaf_index: u64,
        leaf_hash: impl Into<String>,
        hashes: Vec<String>,
    ) -> Result<Self, MerkleVdsError> {
        let value = Self {
            schema_version: MERKLE_INCLUSION_SCHEMA_VERSION,
            algorithm: MERKLE_INCLUSION_ALGORITHM.to_string(),
            leaf_index,
            leaf_hash: leaf_hash.into(),
            hashes,
        };
        value.validate_basic()?;
        Ok(value)
    }

    pub fn validate_basic(&self) -> Result<(), MerkleVdsError> {
        if self.schema_version != MERKLE_INCLUSION_SCHEMA_VERSION
            || self.algorithm != MERKLE_INCLUSION_ALGORITHM
        {
            return Err(MerkleVdsError::Invalid(
                "unsupported Merkle inclusion proof schema or algorithm".to_string(),
            ));
        }
        decode_hex::<32>(&self.leaf_hash)?;
        if self.hashes.len() > MAX_MERKLE_INCLUSION_PROOF_HASHES {
            return Err(MerkleVdsError::Invalid(
                "Merkle inclusion proof exceeds 63 hashes".to_string(),
            ));
        }
        for hash in &self.hashes {
            decode_hex::<32>(hash)?;
        }
        Ok(())
    }

    #[must_use]
    pub const fn leaf_index(&self) -> u64 {
        self.leaf_index
    }

    #[must_use]
    pub fn leaf_hash(&self) -> &str {
        &self.leaf_hash
    }

    #[must_use]
    pub fn hashes(&self) -> &[String] {
        &self.hashes
    }

    /// Verify inclusion using the RFC 9162 section 2.1.3.2 algorithm.
    pub fn verify_sha256(&self, tree_size: u64, root_hash: &str) -> Result<(), MerkleVdsError> {
        self.validate_basic()?;
        if tree_size == 0 || self.leaf_index >= tree_size {
            return Err(MerkleVdsError::Invalid(
                "Merkle inclusion leaf index is outside the advertised tree".to_string(),
            ));
        }

        let root = decode_hex::<32>(root_hash)?;
        let mut fn_index = self.leaf_index;
        let mut sn_index = tree_size - 1;
        let mut reconstructed = decode_hex::<32>(&self.leaf_hash)?;

        for hash in &self.hashes {
            if sn_index == 0 {
                return Err(MerkleVdsError::Invalid(
                    "Merkle inclusion proof contains trailing data".to_string(),
                ));
            }
            let current = decode_hex::<32>(hash)?;
            if fn_index & 1 == 1 || fn_index == sn_index {
                reconstructed = merkle_parent_sha256(&current, &reconstructed);
                if fn_index & 1 == 0 {
                    while fn_index != 0 && fn_index & 1 == 0 {
                        fn_index >>= 1;
                        sn_index >>= 1;
                    }
                }
            } else {
                reconstructed = merkle_parent_sha256(&reconstructed, &current);
            }
            fn_index >>= 1;
            sn_index >>= 1;
        }

        if sn_index != 0 || reconstructed != root {
            return Err(MerkleVdsError::ProofMismatch);
        }
        Ok(())
    }
}

#[must_use]
pub fn merkle_leaf_hash_sha256(entry: &[u8]) -> String {
    encode_hex(&merkle_leaf_hash_bytes(entry))
}

fn merkle_leaf_hash_bytes(entry: &[u8]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x00]);
    hasher.update(entry);
    hasher.finalize().into()
}

/// Verify the append-only boundary used by C2SP-style witnesses.
///
/// This wrapper handles the two special cases that are outside the strict
/// 0 < first_size < second_size RFC 9162 consistency-proof API:
///
/// - first_size == 0: the old root must be the empty-tree hash and the proof
///   must be absent/empty.
/// - first_size == second_size: no growth occurred, so the roots must match
///   and no proof is necessary.
///
/// For a genuinely growing non-empty tree, a proof is required and is checked
/// by the RFC 9162 verifier above.
pub fn verify_append_only_sha256(
    first_size: u64,
    first_root: &str,
    second_size: u64,
    second_root: &str,
    proof: Option<&MerkleConsistencyProofV1>,
) -> Result<(), MerkleVdsError> {
    let first_hash = decode_hex::<32>(first_root)?;
    let second_hash = decode_hex::<32>(second_root)?;
    if let Some(proof) = proof {
        proof.validate_basic()?;
    }
    let empty_root: [u8; 32] = Sha256::digest(b"").into();

    if first_size == 0 {
        if first_hash != empty_root {
            return Err(MerkleVdsError::ProofMismatch);
        }
        if let Some(proof) = proof {
            if !proof.hashes.is_empty() {
                return Err(MerkleVdsError::Invalid(
                    "non-empty consistency proof is invalid from the empty tree".to_string(),
                ));
            }
        }
        if second_size == 0 {
            if second_hash != empty_root {
                return Err(MerkleVdsError::ProofMismatch);
            }
            return Ok(());
        }
        return Ok(());
    }

    if first_size == second_size {
        if first_hash != second_hash {
            return Err(MerkleVdsError::ProofMismatch);
        }
        if let Some(proof) = proof {
            if !proof.hashes.is_empty() {
                return Err(MerkleVdsError::Invalid(
                    "consistency proof must be empty when tree size is unchanged".to_string(),
                ));
            }
        }
        return Ok(());
    }

    if first_size > second_size {
        return Err(MerkleVdsError::Invalid(
            "first Merkle tree size must not exceed second tree size".to_string(),
        ));
    }

    let proof = proof.ok_or_else(|| {
        MerkleVdsError::Invalid(
            "a non-empty tree growth transition requires a consistency proof".to_string(),
        )
    })?;

    proof.verify_sha256(first_size, first_root, second_size, second_root)
}

/// Compute the RFC 9162 Merkle Tree Hash for an ordered list of leaf inputs.
///
/// This is exposed as a protocol-mechanics primitive for future log adapters and tests.
/// The leaf input is domain-separated with 0x00 and internal nodes with 0x01.
#[must_use]
pub fn merkle_tree_hash_sha256(entries: &[Vec<u8>]) -> String {
    encode_hex(&merkle_tree_hash_bytes(entries))
}

fn merkle_tree_hash_bytes(entries: &[Vec<u8>]) -> [u8; 32] {
    match entries.len() {
        0 => Sha256::digest(b"").into(),
        1 => {
            let mut hasher = Sha256::new();
            hasher.update([0x00]);
            hasher.update(&entries[0]);
            hasher.finalize().into()
        },
        n => {
            let mut power = 1usize << (usize::BITS - 1 - n.leading_zeros());
            if power == n {
                power >>= 1;
            }
            let left = merkle_tree_hash_bytes(&entries[..power]);
            let right = merkle_tree_hash_bytes(&entries[power..]);
            merkle_parent_sha256(&left, &right)
        }
    }
}

fn merkle_parent_sha256(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update([0x01]);
    hasher.update(left);
    hasher.update(right);
    hasher.finalize().into()
}

fn decode_hex<const N: usize>(value: &str) -> Result<[u8; N], MerkleVdsError> {
    if value.len() != N * 2 {
        return Err(MerkleVdsError::Invalid(format!(
            "expected {N}-byte lowercase hexadecimal value"
        )));
    }
    let bytes = value.as_bytes();
    let mut output = [0_u8; N];
    for index in 0..N {
        let high = decode_nibble(bytes[index * 2])?;
        let low = decode_nibble(bytes[index * 2 + 1])?;
        output[index] = (high << 4) | low;
    }
    Ok(output)
}

fn decode_nibble(value: u8) -> Result<u8, MerkleVdsError> {
    match value {
        b'0'..=b'9' => Ok(value - b'0'),
        b'a'..=b'f' => Ok(value - b'a' + 10),
        _ => Err(MerkleVdsError::Invalid(
            "Merkle hashes must use lowercase hexadecimal".to_string(),
        )),
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MerkleVdsError {
    Invalid(String),
    ProofMismatch,
}

impl std::fmt::Display for MerkleVdsError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Invalid(message) => write!(formatter, "invalid Merkle VDS input: {message}"),
            Self::ProofMismatch => {
                write!(
                    formatter,
                    "Merkle consistency proof does not match advertised roots"
                )
            }
        }
    }
}

impl std::error::Error for MerkleVdsError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn consistency_proof(m: usize, entries: &[Vec<u8>]) -> Vec<String> {
        let mut output = Vec::new();

        fn subproof(m: usize, entries: &[Vec<u8>], complete: bool, output: &mut Vec<String>) {
            let n = entries.len();
            if m == n {
                if !complete {
                    output.push(merkle_tree_hash_sha256(entries));
                }
                return;
            }

            let mut power = 1usize << (usize::BITS - 1 - n.leading_zeros());
            if power == n {
                power >>= 1;
            }

            if m <= power {
                subproof(m, &entries[..power], complete, output);
                output.push(merkle_tree_hash_sha256(&entries[power..]));
            } else {
                subproof(m - power, &entries[power..], false, output);
                output.push(merkle_tree_hash_sha256(&entries[..power]));
            }
        }

        subproof(m, entries, true, &mut output);
        output
    }

    fn entries(count: usize) -> Vec<Vec<u8>> {
        (0..count)
            .map(|index| format!("leaf-{index}").into_bytes())
            .collect()
    }

    #[test]
    fn accepts_many_rfc9162_consistency_shapes() {
        for second_size in 2..64 {
            let data = entries(second_size);
            let second_root = merkle_tree_hash_sha256(&data);

            for first_size in 1..second_size {
                let proof = consistency_proof(first_size, &data);
                let proof = MerkleConsistencyProofV1::new(proof).expect("proof");
                let first_root = merkle_tree_hash_sha256(&data[..first_size].to_vec());

                proof
                    .verify_sha256(
                        first_size as u64,
                        &first_root,
                        second_size as u64,
                        &second_root,
                    )
                    .expect("consistency proof");
            }
        }
    }

    #[test]
    fn rejects_corrupted_path_hash() {
        let data = entries(7);
        let mut proof = consistency_proof(3, &data);
        proof[0] = "00".repeat(32);
        let proof = MerkleConsistencyProofV1::new(proof).expect("shape-valid proof");
        let first_root = merkle_tree_hash_sha256(&data[..3].to_vec());
        let second_root = merkle_tree_hash_sha256(&data);

        assert!(matches!(
            proof.verify_sha256(3, &first_root, 7, &second_root),
            Err(MerkleVdsError::ProofMismatch)
        ));
    }

    #[test]
    fn rejects_corrupted_advertised_root() {
        let data = entries(7);
        let proof = MerkleConsistencyProofV1::new(consistency_proof(3, &data)).expect("proof");
        let second_root = merkle_tree_hash_sha256(&data);
        let wrong_first_root = "11".repeat(32);

        assert!(matches!(
            proof.verify_sha256(3, &wrong_first_root, 7, &second_root),
            Err(MerkleVdsError::ProofMismatch)
        ));
    }

    #[test]
    fn rejects_wrong_size_relationship() {
        let data = entries(4);
        let proof = MerkleConsistencyProofV1::new(consistency_proof(2, &data)).expect("proof");
        let root = merkle_tree_hash_sha256(&data);

        assert!(matches!(
            proof.verify_sha256(4, &root, 4, &root),
            Err(MerkleVdsError::Invalid(_))
        ));
    }

    #[test]
    fn c2sp_empty_tree_start_requires_empty_root_and_proof() {
        let empty_root = merkle_tree_hash_sha256(&[]);
        let data = entries(5);
        let second_root = merkle_tree_hash_sha256(&data);

        verify_append_only_sha256(0, &empty_root, 5, &second_root, None).expect("empty-tree start");

        let bad_root = "11".repeat(32);
        assert!(matches!(
            verify_append_only_sha256(0, &bad_root, 5, &second_root, None),
            Err(MerkleVdsError::ProofMismatch)
        ));

        let nonempty = MerkleConsistencyProofV1::new(vec!["22".repeat(32)]).expect("shape-valid");
        assert!(matches!(
            verify_append_only_sha256(0, &empty_root, 5, &second_root, Some(&nonempty),),
            Err(MerkleVdsError::Invalid(_))
        ));
    }

    #[test]
    fn unchanged_tree_head_requires_equal_roots() {
        let data = entries(5);
        let root = merkle_tree_hash_sha256(&data);
        let empty = MerkleConsistencyProofV1::empty();

        verify_append_only_sha256(5, &root, 5, &root, Some(&empty)).expect("unchanged tree");

        let wrong = "33".repeat(32);
        assert!(matches!(
            verify_append_only_sha256(5, &root, 5, &wrong, Some(&empty)),
            Err(MerkleVdsError::ProofMismatch)
        ));
    }

    #[test]
    fn rejects_deserialized_malformed_consistency_proof() {
        let malformed = serde_json::json!({
            "hashes": ["zz"]
        });
        let proof: MerkleConsistencyProofV1 =
            serde_json::from_value(malformed).expect("deserialize");
        let root = merkle_tree_hash_sha256(&entries(2));
        assert!(matches!(
            verify_append_only_sha256(
                1,
                &merkle_tree_hash_sha256(&entries(1)),
                2,
                &root,
                Some(&proof),
            ),
            Err(MerkleVdsError::Invalid(_))
        ));
    }

    #[test]
    fn accepts_inclusion_proofs_for_many_tree_shapes() {
        fn inclusion_path(index: usize, entries: &[Vec<u8>]) -> Vec<String> {
            let n = entries.len();
            if n <= 1 {
                return Vec::new();
            }
            let mut power = 1usize << (usize::BITS - 1 - n.leading_zeros());
            if power == n {
                power >>= 1;
            }
            if index < power {
                let mut path = inclusion_path(index, &entries[..power]);
                path.push(merkle_tree_hash_sha256(&entries[power..]));
                path
            } else {
                let mut path = inclusion_path(index - power, &entries[power..]);
                path.push(merkle_tree_hash_sha256(&entries[..power]));
                path
            }
        }

        for tree_size in 1..64 {
            let data = (0..tree_size)
                .map(|index| format!("leaf-{index}").into_bytes())
                .collect::<Vec<_>>();
            let root = merkle_tree_hash_sha256(&data);
            for index in 0..tree_size {
                let leaf_hash = merkle_leaf_hash_sha256(&data[index]);
                let proof = MerkleInclusionProofV1::new(
                    index as u64,
                    leaf_hash,
                    inclusion_path(index, &data),
                )
                .expect("inclusion proof");
                proof.verify_sha256(tree_size as u64, &root).expect("inclusion");
            }
        }
    }

    #[test]
    fn rejects_inclusion_leaf_out_of_range_and_wrong_root() {
        let data = entries(5);
        let root = merkle_tree_hash_sha256(&data);
        let leaf_hash = merkle_leaf_hash_sha256(&data[2]);
        let proof = MerkleInclusionProofV1::new(5, leaf_hash.clone(), Vec::new())
            .expect("shape-valid out-of-range proof");
        assert!(matches!(
            proof.verify_sha256(5, &root),
            Err(MerkleVdsError::Invalid(_))
        ));

        let valid_path = {
            let mut path = Vec::new();
            path.push(merkle_tree_hash_sha256(&data[3..]));
            path
        };
        let proof = MerkleInclusionProofV1::new(
            2,
            leaf_hash,
            valid_path,
        )
        .expect("shape-valid proof");
        assert!(matches!(
            proof.verify_sha256(5, &"11".repeat(32)),
            Err(MerkleVdsError::ProofMismatch)
        ));
    }

    #[test]
    fn rejects_oversized_consistency_proof() {
        let oversized = vec!["00".repeat(32); MAX_MERKLE_CONSISTENCY_PROOF_HASHES + 1];
        assert!(matches!(
            MerkleConsistencyProofV1::new(oversized),
            Err(MerkleVdsError::Invalid(message)) if message.contains("63 hashes")
        ));
    }

    #[test]
    fn rejects_empty_proof_and_malformed_hash() {
        assert!(matches!(
            MerkleConsistencyProofV1::new(Vec::new()),
            Err(MerkleVdsError::Invalid(_))
        ));
        assert!(matches!(
            MerkleConsistencyProofV1::new(vec!["00".to_string()]),
            Err(MerkleVdsError::Invalid(_))
        ));
    }
}
