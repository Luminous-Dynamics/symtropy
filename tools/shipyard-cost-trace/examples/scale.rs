use shipyard_cost_trace::{
    EventKind, EvidencePermission, LedgerEvent, QuantityAggregationSemantics, QuantityEvent, Stage,
    audit,
};
use std::time::Instant;

const CONFIG: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";
const SAMPLE_COUNT: usize = 21;

fn workload(size: usize) -> Vec<LedgerEvent> {
    let mut events = Vec::with_capacity(size);
    for index in 0..size {
        let event_id = format!("event-{index:06}");
        let related_event_ids = if index == 0 {
            Vec::new()
        } else {
            vec![format!("event-{:06}", index - 1)]
        };
        events.push(LedgerEvent {
            event_id,
            build_id: "scale-fixture-001".to_owned(),
            configuration_sha256: CONFIG.to_owned(),
            task_id: "fabrication-block-a".to_owned(),
            stage: Stage::Observed,
            source_ref: format!("synthetic:event-{index:06}"),
            source_permission: EvidencePermission::PublicOrPermitted,
            source_permission_ref: "fixture-rights:public-test-data".to_owned(),
            related_event_ids,
            kind: EventKind::Quantity(QuantityEvent {
                quantity_key: "labor.fitter.hours".to_owned(),
                quantity: 1.0,
                aggregation_semantics: QuantityAggregationSemantics::AdditiveContribution,
                unit: "hour".to_owned(),
                rate: None,
            }),
        });
    }

    // Reverse creation order to exercise iterative lineage traversal on a long chain.
    events.reverse();
    events
}

fn percentile(sorted: &[u128], numerator: usize, denominator: usize) -> u128 {
    let last = sorted.len() - 1;
    let index = (last * numerator).div_ceil(denominator);
    sorted[index.min(last)]
}

fn main() {
    println!(
        "schema,event_count,samples,median_audit_ns,p95_audit_ns,quantity_axes,observed_units"
    );
    for size in [20_usize, 200, 2_000, 20_000] {
        let events = workload(size);
        let mut samples = Vec::with_capacity(SAMPLE_COUNT);
        let mut axes = 0;
        let mut observed = 0.0;

        for _ in 0..SAMPLE_COUNT {
            let start = Instant::now();
            let report = audit(&events).expect("valid synthetic scale fixture");
            samples.push(start.elapsed().as_nanos());
            axes = report.quantity_reconciliations.len();
            observed = report.quantity_reconciliations[0]
                .observed
                .expect("observed quantity in scale fixture");
            assert_eq!(report.event_count, size);
        }

        samples.sort_unstable();
        println!(
            "shipyard-cost-trace-v0,{size},{SAMPLE_COUNT},{},{},{axes},{observed}",
            percentile(&samples, 50, 100),
            percentile(&samples, 95, 100),
        );
    }

    println!("Synthetic release-mode characterization only; not an industrial throughput claim.");
}
