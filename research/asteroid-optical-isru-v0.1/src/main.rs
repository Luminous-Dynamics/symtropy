use asteroid_optical_isru_reference::{validate_corpus, BenchmarkCorpus};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let corpus: BenchmarkCorpus =
        serde_json::from_str(include_str!("../fixtures/cases-v0.1.json"))?;
    validate_corpus(&corpus)?;

    println!("ASTEROID-OPTICAL-ISRU-001 corpus audit");
    println!("schema: {}", corpus.schema_version);
    println!("sources: {}", corpus.source_records.len());
    println!("cases: {}", corpus.cases.len());
    println!("result: PASS_CONTRACT_ONLY");
    println!("claim ceiling: {}", corpus.claim_ceiling);

    for case in &corpus.cases {
        println!(
            "{}\t{}\t{}",
            case.id, case.evidence_class, case.expected_disposition
        );
    }

    Ok(())
}
