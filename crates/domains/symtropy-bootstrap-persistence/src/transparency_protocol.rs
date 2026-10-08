// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Protocol-shaped Merkle proof bindings for future transparency wire adapters.
//!
//! These types deliberately model the semantic fields carried by RFC 9942 proof
//! content without implementing CBOR/COSE serialization. Their purpose is to keep
//! tree-size context attached to the proof before a future wire adapter is introduced.

use serde::{Deserialize, Serialize};

use super::transparency_vds::{
    MerkleConsistencyProofV1, MerkleInclusionProofV1, MerkleVdsError,
};

/// Semantic binding for the RFC 9942 consistency-proof content tuple:
/// [older tree size, newer tree size, consistency path].
///
/// The hash path remains represented in the repository's canonical lowercase
/// hexadecimal form. A future CBOR adapter must encode these hashes as bstr values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rfc9942ConsistencyProofContentV1 {
    older_tree_size: u64,
    newer_tree_size: u64,
    consistency_path: MerkleConsistencyProofV1,
}

impl Rfc9942ConsistencyProofContentV1 {
    pub fn new(
        older_tree_size: u64,
        newer_tree_size: u64,
        consistency_path: MerkleConsistencyProofV1,
    ) -> Result<Self, MerkleVdsError> {
        consistency_path.validate_basic()?;
        if older_tree_size == 0 || older_tree_size >= newer_tree_size {
            return Err(MerkleVdsError::Invalid(
                "RFC 9942 consistency content requires 0 < older size < newer size".to_string(),
            ));
        }
        Ok(Self {
            older_tree_size,
            newer_tree_size,
            consistency_path,
        })
    }

    #[must_use]
    pub const fn older_tree_size(&self) -> u64 {
        self.older_tree_size
    }

    #[must_use]
    pub const fn newer_tree_size(&self) -> u64 {
        self.newer_tree_size
    }

    #[must_use]
    pub fn consistency_path(&self) -> &MerkleConsistencyProofV1 {
        &self.consistency_path
    }

    pub fn validate_basic(&self) -> Result<(), MerkleVdsError> {
        Self::new(
            self.older_tree_size,
            self.newer_tree_size,
            self.consistency_path.clone(),
        )
        .map(|_| ())
    }

    /// Verify using the sizes carried by this proof content object.
    pub fn verify_sha256(
        &self,
        older_root: &str,
        newer_root: &str,
    ) -> Result<(), MerkleVdsError> {
        self.validate_basic()?;
        self.consistency_path.verify_sha256(
            self.older_tree_size,
            older_root,
            self.newer_tree_size,
            newer_root,
        )
    }
}

/// Binds RFC 9942 consistency content to the exact older and newer tree roots
/// supplied by the surrounding witness/log evidence.
///
/// The newer root is the value a signed consistency proof's detached payload
/// must authenticate. Keeping it beside the proof content prevents a future
/// adapter from accidentally verifying the path against a different root.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rfc9942ConsistencyProofBindingV1 {
    content: Rfc9942ConsistencyProofContentV1,
    older_root_hash: String,
    newer_root_hash: String,
}

impl Rfc9942ConsistencyProofBindingV1 {
    pub fn new(
        content: Rfc9942ConsistencyProofContentV1,
        older_root_hash: impl Into<String>,
        newer_root_hash: impl Into<String>,
    ) -> Result<Self, MerkleVdsError> {
        let value = Self {
            content,
            older_root_hash: older_root_hash.into(),
            newer_root_hash: newer_root_hash.into(),
        };
        value.validate_basic()?;
        Ok(value)
    }

    #[must_use]
    pub fn content(&self) -> &Rfc9942ConsistencyProofContentV1 {
        &self.content
    }

    #[must_use]
    pub fn older_root_hash(&self) -> &str {
        &self.older_root_hash
    }

    #[must_use]
    pub fn newer_root_hash(&self) -> &str {
        &self.newer_root_hash
    }

    pub fn validate_basic(&self) -> Result<(), MerkleVdsError> {
        self.content.validate_basic()?;
        if !is_sha256_hex(&self.older_root_hash) || !is_sha256_hex(&self.newer_root_hash) {
            return Err(MerkleVdsError::Invalid(
                "RFC 9942 proof roots must be lowercase SHA-256".to_string(),
            ));
        }
        Ok(())
    }

    pub fn verify_sha256(&self) -> Result<(), MerkleVdsError> {
        self.validate_basic()?;
        self.content
            .verify_sha256(&self.older_root_hash, &self.newer_root_hash)
    }
}

/// Binds RFC 9942 inclusion content to the exact tree root that a signed
/// inclusion receipt proves.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rfc9942InclusionProofBindingV1 {
    content: Rfc9942InclusionProofContentV1,
    root_hash: String,
}

impl Rfc9942InclusionProofBindingV1 {
    pub fn new(
        content: Rfc9942InclusionProofContentV1,
        root_hash: impl Into<String>,
    ) -> Result<Self, MerkleVdsError> {
        let value = Self {
            content,
            root_hash: root_hash.into(),
        };
        value.validate_basic()?;
        Ok(value)
    }

    #[must_use]
    pub fn content(&self) -> &Rfc9942InclusionProofContentV1 {
        &self.content
    }

    #[must_use]
    pub fn root_hash(&self) -> &str {
        &self.root_hash
    }

    pub fn validate_basic(&self) -> Result<(), MerkleVdsError> {
        self.content.validate_basic()?;
        if !is_sha256_hex(&self.root_hash) {
            return Err(MerkleVdsError::Invalid(
                "RFC 9942 proof root must be lowercase SHA-256".to_string(),
            ));
        }
        Ok(())
    }

    pub fn verify_sha256(&self) -> Result<(), MerkleVdsError> {
        self.validate_basic()?;
        self.content.verify_sha256(&self.root_hash)
    }
}

/// Semantic binding for the RFC 9942 inclusion-proof content tuple:
/// [tree size, leaf index, inclusion path].
///
/// The leaf hash remains part of the repository's semantic proof object.
/// A future CBOR adapter must encode the path hashes as bstr values.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Rfc9942InclusionProofContentV1 {
    tree_size: u64,
    proof: MerkleInclusionProofV1,
}

impl Rfc9942InclusionProofContentV1 {
    pub fn new(
        tree_size: u64,
        proof: MerkleInclusionProofV1,
    ) -> Result<Self, MerkleVdsError> {
        proof.validate_basic()?;
        if tree_size == 0 || proof.leaf_index() >= tree_size {
            return Err(MerkleVdsError::Invalid(
                "RFC 9942 inclusion content requires leaf index < tree size".to_string(),
            ));
        }
        Ok(Self { tree_size, proof })
    }

    #[must_use]
    pub const fn tree_size(&self) -> u64 {
        self.tree_size
    }

    #[must_use]
    pub fn proof(&self) -> &MerkleInclusionProofV1 {
        &self.proof
    }

    pub fn validate_basic(&self) -> Result<(), MerkleVdsError> {
        Self::new(self.tree_size, self.proof.clone()).map(|_| ())
    }

    /// Verify using the tree size carried by this proof content object.
    pub fn verify_sha256(&self, root_hash: &str) -> Result<(), MerkleVdsError> {
        self.validate_basic()?;
        self.proof.verify_sha256(self.tree_size, root_hash)
    }
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::transparency_vds::{
        merkle_leaf_hash_sha256, merkle_tree_hash_sha256,
    };

    fn entries(count: usize) -> Vec<Vec<u8>> {
        (0..count)
            .map(|index| format!("leaf-{index}").into_bytes())
            .collect()
    }

    fn consistency_proof(m: usize, entries: &[Vec<u8>]) -> Vec<String> {
        fn subproof(
            m: usize,
            entries: &[Vec<u8>],
            complete: bool,
            output: &mut Vec<String>,
        ) {
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

        let mut output = Vec::new();
        subproof(m, entries, true, &mut output);
        output
    }

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

    #[test]
    fn consistency_content_is_self_bound_to_both_tree_sizes() {
        let old_entries = entries(3);
        let new_entries = entries(7);
        let old_root = merkle_tree_hash_sha256(&old_entries);
        let new_root = merkle_tree_hash_sha256(&new_entries);
        let path = MerkleConsistencyProofV1::new(consistency_proof(3, &new_entries))
            .expect("proof");
        let content =
            Rfc9942ConsistencyProofContentV1::new(3, 7, path).expect("content");

        content
            .verify_sha256(&old_root, &new_root)
            .expect("bound consistency proof");

        assert!(matches!(
            Rfc9942ConsistencyProofContentV1::new(
                7,
                3,
                content.consistency_path().clone(),
            ),
            Err(MerkleVdsError::Invalid(_))
        ));
    }

    #[test]
    fn inclusion_content_is_self_bound_to_tree_size() {
        let data = entries(5);
        let root = merkle_tree_hash_sha256(&data);
        let proof = MerkleInclusionProofV1::new(
            2,
            merkle_leaf_hash_sha256(&data[2]),
            inclusion_path(2, &data),
        )
        .expect("proof");
        let content = Rfc9942InclusionProofContentV1::new(5, proof).expect("content");

        content.verify_sha256(&root).expect("bound inclusion proof");

        assert!(matches!(
            Rfc9942InclusionProofContentV1::new(2, content.proof().clone()),
            Err(MerkleVdsError::Invalid(_))
        ));
    }

    #[test]
    fn consistency_binding_rejects_root_mismatch() {
        let old_entries = entries(3);
        let new_entries = entries(7);
        let old_root = merkle_tree_hash_sha256(&old_entries);
        let new_root = merkle_tree_hash_sha256(&new_entries);
        let path = MerkleConsistencyProofV1::new(consistency_proof(3, &new_entries))
            .expect("proof");
        let content =
            Rfc9942ConsistencyProofContentV1::new(3, 7, path).expect("content");
        let binding =
            Rfc9942ConsistencyProofBindingV1::new(content, old_root.clone(), new_root.clone())
                .expect("binding");

        binding.verify_sha256().expect("bound roots");

        let swapped =
            Rfc9942ConsistencyProofBindingV1::new(
                binding.content().clone(),
                old_root,
                "11".repeat(32),
            )
            .expect("shape-valid root");
        assert!(matches!(
            swapped.verify_sha256(),
            Err(MerkleVdsError::ProofMismatch)
        ));
    }

    #[test]
    fn inclusion_binding_rejects_root_mismatch() {
        let data = entries(5);
        let root = merkle_tree_hash_sha256(&data);
        let proof = MerkleInclusionProofV1::new(
            2,
            merkle_leaf_hash_sha256(&data[2]),
            inclusion_path(2, &data),
        )
        .expect("proof");
        let content = Rfc9942InclusionProofContentV1::new(5, proof).expect("content");
        let binding =
            Rfc9942InclusionProofBindingV1::new(content, root).expect("binding");

        binding.verify_sha256().expect("bound root");

        let wrong = Rfc9942InclusionProofBindingV1::new(
            binding.content().clone(),
            "22".repeat(32),
        )
        .expect("shape-valid root");
        assert!(matches!(
            wrong.verify_sha256(),
            Err(MerkleVdsError::ProofMismatch)
        ));
    }

    #[test]
    fn malformed_deserialized_binding_fails_closed() {
        let path = MerkleConsistencyProofV1::empty();
        let malformed = Rfc9942ConsistencyProofContentV1 {
            older_tree_size: 4,
            newer_tree_size: 4,
            consistency_path: path,
        };
        assert!(matches!(
            malformed.validate_basic(),
            Err(MerkleVdsError::Invalid(_))
        ));
    }
}
