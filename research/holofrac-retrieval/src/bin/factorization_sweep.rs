use holofrac_spatial_retrieval::run_factorization_profile;

fn minimum_entity_count(fact_count: usize) -> usize {
    let mut count = 2_usize;
    while count.saturating_mul(count - 1) < fact_count {
        count += 1;
    }
    count
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    let dimension = 1_024_usize;
    println!("# HoloFrac factorized role-codebook sweep v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!("# dimension={dimension}");
    println!(
        "fact_count,reuse_regime,entity_count,unique_role_values,reuse_ratio,index_build_us,duplicated_role_slot_bytes,factorized_role_slot_bytes,slot_reference_bytes,role_slot_bytes_saved,holistic_vector_bytes,retained_payload_bytes_ex_map_keys"
    );

    for fact_count in [100_usize, 1_000, 5_000] {
        let high_reuse_entities = minimum_entity_count(fact_count);
        let medium_reuse_entities = (fact_count / 4).max(high_reuse_entities + 1);
        let regimes = [
            ("high", high_reuse_entities),
            ("medium", medium_reuse_entities),
            ("low", fact_count + 1),
        ];

        for (regime, entity_count) in regimes {
            let profile = run_factorization_profile(
                fact_count,
                entity_count,
                dimension,
                0x484f_4c4f_434f_4445,
            )?;
            println!(
                "{},{},{},{},{:.6},{:.3},{},{},{},{},{},{}",
                profile.fact_count,
                regime,
                profile.entity_count,
                profile.unique_role_codebook_entries,
                profile.role_slot_reuse_ratio,
                profile.index_build_micros,
                profile.duplicated_role_slot_payload_bytes,
                profile.factorized_role_slot_payload_bytes,
                profile.role_slot_reference_bytes,
                profile.factorized_role_slot_bytes_saved,
                profile.holistic_vector_payload_bytes,
                profile.retained_payload_bytes_excluding_map_and_key_overhead
            );
        }
    }
    Ok(())
}
