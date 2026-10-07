                    MerkleConsistencyProofV1::new(consistency_proof(1, &entries))
                        .expect("w2 consistency proof"),
                ),
            )
            .expect("w2 evidence"),
        ];

        let accepted = state
            .verify_for_new_transition_with_witness_evidence(
                &log,
                &policy,
                &checkpoint,
                &evidence,
                "bootstrap",
                1,
                3,
                &"55".repeat(32),
            )
            .expect("both different VDS frontiers should verify");

        assert_eq!(accepted.witness_evidence().len(), 2);
        assert_ne!(
            accepted.witness_evidence()[0].vds_consistency_proof(),
            accepted.witness_evidence()[1].vds_consistency_proof()
        );
        assert_eq!(
            accepted.witness_evidence()[0]
                .retained_vds_tree_head()
                .expect("w1 retained VDS")
                .tree_size(),
            2
        );
        assert_eq!(
            accepted.witness_evidence()[1]
                .retained_vds_tree_head()
                .expect("w2 retained VDS")
                .tree_size(),
            1
        );
    }

    #[test]
    fn witness_evidence_rejects_false_retained_vds_frontier() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let mut state = TransparencyWitnessSetV1::new(&policy).expect("state");
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let first = accepted_first_checkpoint(&keys, &state, &policy);
        state.commit_after_durable_append(first).expect("first commit");

        let entries = vec![b"leaf-0".to_vec(), b"leaf-1".to_vec()];
        let root = crate::transparency_vds::merkle_tree_hash_sha256(&entries);
        let genesis = transparency_genesis_digest("log-1", 1, &policy.commitment());
        let unsigned = TransparencyCheckpointUnsignedV1::new(
            "log-1",
            1,
            2,
            "bootstrap",
            1,
            2,
            &"88".repeat(32),
            2,
            &root,
            &genesis,
            policy.commitment(),
        )
        .expect("checkpoint");
        let signature = keys.log.sign(&unsigned.signing_digest());
        let checkpoint = unsigned
            .into_signed(encode_hex(signature.as_ref()))
            .expect("signed checkpoint");
        let witness_signature = witnessed_signatures(&keys, &checkpoint, &[0])[0].clone();
        let false_frontier = TransparencyVdsTreeHeadV1::empty();
        let evidence = TransparencyWitnessEvidenceV1 {
            witness_signature,
            vds_consistency_proof: Some(MerkleConsistencyProofV1::empty()),
            retained_vds_tree_head: Some(false_frontier),
        };

        let error = state
            .verify_candidate_with_witness_evidence(
                &log,
                &policy,
                &checkpoint,
                &[evidence],
                "bootstrap",
                1,
                2,
                &"88".repeat(32),
            )
            .expect_err("false retained VDS frontier must reject");
        assert!(matches!(
            error,
            TransparencyError::VdsPredecessorMismatch { witness_id }
                if witness_id == "w1"
        ));
    }

    #[test]
    fn inclusion_evidence_binds_entry_to_signed_checkpoint() {
        let keys = TestKeys::new();
        let policy = policy(&keys);
        let log = TransparencyLogAuthorityV1::from_public_key_hex("log-1", 1, keys.log_public())
            .expect("log");
        let entries = vec![
            b"statement-0".to_vec(),
            b"statement-1".to_vec(),
            b"statement-2".to_vec(),
        ];
        let root = crate::transparency_vds::merkle_tree_hash_sha256(&entries);
        let unsigned = TransparencyCheckpointUnsignedV1::new(
            "log-1",