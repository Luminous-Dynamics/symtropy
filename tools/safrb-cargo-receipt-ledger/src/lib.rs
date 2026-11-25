#![forbid(unsafe_code)]

//! Synthetic-only, time-indexed cargo receipt and fuel-availability ledger.
//! This is an event/accounting contract, not a port, tank, pipeline or refinery solver.

use std::collections::{BTreeMap, BTreeSet};

pub const SCHEMA_VERSION: &str = "0.1.0";
pub const HORIZON_HOURS: u32 = 72;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReceiptEvidence {
    Verified,
    Missing,
    Conflicting,
}
impl ReceiptEvidence {
    pub const fn as_str(self) -> &'static str {
        match self { Self::Verified => "verified", Self::Missing => "missing", Self::Conflicting => "conflicting" }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AssayDisposition {
    Passed,
    Failed,
    Pending,
    Conflicting,
}
impl AssayDisposition {
    pub const fn as_str(self) -> &'static str {
        match self { Self::Passed => "passed", Self::Failed => "failed", Self::Pending => "pending", Self::Conflicting => "conflicting" }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AuthorityDisposition {
    Authorized,
    Withheld,
    Conflicting,
}
impl AuthorityDisposition {
    pub const fn as_str(self) -> &'static str {
        match self { Self::Authorized => "authorized", Self::Withheld => "withheld", Self::Conflicting => "conflicting" }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoManifest {
    pub cargo_id: String,
    pub supplier_id: String,
    pub terminal_id: String,
    pub product: String,
    pub grade: String,
    pub planned_quantity_grams: u64,
    pub declared_eta_hour: u32,
    pub known_at_hour: u32,
    pub common_cause_group: String,
    pub documentary_status: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PhysicalReceipt {
    pub receipt_event_id: String,
    pub cargo_id: String,
    pub batch_id: String,
    pub terminal_id: String,
    pub product: String,
    pub grade: String,
    pub received_at_hour: u32,
    pub received_lower_grams: u64,
    pub actual_received_grams: u64,
    pub received_upper_grams: u64,
    pub evidence: ReceiptEvidence,
    pub evidence_ref: String,
    pub evidence_digest: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QualityAssessment {
    pub assay_event_id: String,
    pub batch_id: String,
    pub assessed_at_hour: u32,
    pub disposition: AssayDisposition,
    pub released_lower_grams: u64,
    pub sample_chain_ref: String,
    pub evidence_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct AccessAuthorization {
    pub authority_event_id: String,
    pub batch_id: String,
    pub valid_from_hour: u32,
    /// Exclusive end of the authority window.
    pub valid_until_hour: u32,
    pub disposition: AuthorityDisposition,
    pub encumbered_grams: u64,
    pub max_transfer_grams: u64,
    pub authority_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferRoute {
    pub route_id: String,
    pub capacity_grams_per_hour: u64,
    pub transit_delay_hours: u32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TransferReceipt {
    pub transfer_event_id: String,
    pub batch_id: String,
    pub route_id: String,
    /// The loading window is [load_start_hour, dispatch_hour).
    pub load_start_hour: u32,
    pub dispatch_hour: u32,
    pub arrival_hour: u32,
    pub source_quantity_grams: u64,
    pub delivered_quantity_grams: u64,
    pub declared_route_loss_grams: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GeneratorFuelIssue {
    pub issue_event_id: String,
    pub transfer_event_id: String,
    pub issued_at_hour: u32,
    pub issued_quantity_grams: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CargoCase {
    pub case_id: String,
    pub manifests: Vec<CargoManifest>,
    pub receipts: Vec<PhysicalReceipt>,
    pub quality_assessments: Vec<QualityAssessment>,
    pub authorizations: Vec<AccessAuthorization>,
    pub routes: Vec<TransferRoute>,
    pub transfers: Vec<TransferReceipt>,
    pub fuel_issues: Vec<GeneratorFuelIssue>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HourlyLedger {
    pub hour_index: u32,
    pub admitted_lower_grams: u64,
    pub terminal_inventory_grams: u64,
    pub terminal_accessible_grams: u64,
    pub in_transit_grams: u64,
    pub delivered_to_generator_grams: u64,
    pub generator_available_grams: u64,
    pub fuel_issued_grams: u64,
    pub route_loss_grams: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BatchBalance {
    pub batch_id: String,
    pub received_lower_grams: u64,
    pub admitted_grams: u64,
    pub quarantined_or_unreleased_grams: u64,
    pub unverified_grams: u64,
    pub terminal_closing_grams: u64,
    pub transferred_source_grams: u64,
    pub delivered_grams: u64,
    pub route_loss_grams: u64,
    pub generator_closing_grams: u64,
    pub fuel_issued_grams: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Projection {
    pub hourly: Vec<HourlyLedger>,
    pub batches: Vec<BatchBalance>,
}

fn nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() { Err(format!("{label} must be non-empty")) } else { Ok(()) }
}

fn add(total: &mut u64, value: u64, label: &str) -> Result<(), String> {
    *total = total.checked_add(value).ok_or_else(|| format!("{label} overflow"))?;
    Ok(())
}

fn event_id(ids: &mut BTreeSet<String>, value: &str) -> Result<(), String> {
    nonempty(value, "event ID")?;
    if !ids.insert(value.to_owned()) { return Err(format!("duplicate or replayed event ID: {value}")); }
    Ok(())
}

fn receipt_admitted(receipt: &PhysicalReceipt, assay: Option<&QualityAssessment>) -> u64 {
    if receipt.evidence != ReceiptEvidence::Verified { return 0; }
    match assay {
        Some(a) if a.disposition == AssayDisposition::Passed => a.released_lower_grams,
        _ => 0,
    }
}

pub fn validate_and_project(case: &CargoCase) -> Result<Projection, String> {
    nonempty(&case.case_id, "case ID")?;
    let mut cargo_ids = BTreeMap::new();
    for manifest in &case.manifests {
        nonempty(&manifest.cargo_id, "cargo ID")?;
        nonempty(&manifest.supplier_id, "supplier ID")?;
        nonempty(&manifest.terminal_id, "terminal ID")?;
        nonempty(&manifest.product, "product")?;
        nonempty(&manifest.grade, "grade")?;
        nonempty(&manifest.common_cause_group, "common-cause group")?;
        nonempty(&manifest.documentary_status, "documentary status")?;
        if manifest.planned_quantity_grams == 0 { return Err("manifest planned quantity must be positive".into()); }
        if manifest.known_at_hour >= HORIZON_HOURS { return Err("manifest known_at_hour must be inside the 72-hour horizon".into()); }
        if cargo_ids.insert(manifest.cargo_id.as_str(), manifest).is_some() {
            return Err(format!("duplicate cargo manifest: {}", manifest.cargo_id));
        }
    }
    let mut ids = BTreeSet::new();
    let mut receipts = BTreeMap::new();
    for receipt in &case.receipts {
        event_id(&mut ids, &receipt.receipt_event_id)?;
        let manifest = cargo_ids.get(receipt.cargo_id.as_str()).ok_or_else(|| format!("receipt references unknown cargo {}", receipt.cargo_id))?;
        if receipt.terminal_id != manifest.terminal_id || receipt.product != manifest.product || receipt.grade != manifest.grade {
            return Err(format!("receipt identity/product/grade mismatch for batch {}", receipt.batch_id));
        }
        nonempty(&receipt.batch_id, "batch ID")?;
        if receipt.received_lower_grams > receipt.actual_received_grams || receipt.actual_received_grams > receipt.received_upper_grams {
            return Err(format!("receipt quantity interval does not contain actual quantity for {}", receipt.batch_id));
        }
        if receipt.evidence == ReceiptEvidence::Verified {
            nonempty(&receipt.evidence_ref, "verified receipt evidence reference")?;
            nonempty(&receipt.evidence_digest, "verified receipt evidence digest")?;
        }
        if receipts.insert(receipt.batch_id.as_str(), receipt).is_some() {
            return Err(format!("duplicate batch receipt: {}", receipt.batch_id));
        }
    }

    let mut assays = BTreeMap::new();
    for assay in &case.quality_assessments {
        event_id(&mut ids, &assay.assay_event_id)?;
        let receipt = receipts.get(assay.batch_id.as_str()).ok_or_else(|| format!("assay references unknown batch {}", assay.batch_id))?;
        if assay.assessed_at_hour < receipt.received_at_hour { return Err(format!("assay predates physical receipt for {}", assay.batch_id)); }
        if assay.released_lower_grams > receipt.received_lower_grams { return Err(format!("assay release exceeds conservative receipt lower bound for {}", assay.batch_id)); }
        if assay.disposition == AssayDisposition::Passed {
            nonempty(&assay.sample_chain_ref, "passed assay sample-chain reference")?;
            nonempty(&assay.evidence_ref, "passed assay evidence reference")?;
        } else if assay.released_lower_grams != 0 {
            return Err(format!("non-passing assay cannot release stock for {}", assay.batch_id));
        }
        if assays.insert(assay.batch_id.as_str(), assay).is_some() { return Err(format!("duplicate assay for batch {}", assay.batch_id)); }
    }

    let mut authorizations = BTreeMap::new();
    for authorization in &case.authorizations {
        event_id(&mut ids, &authorization.authority_event_id)?;
        let receipt = receipts.get(authorization.batch_id.as_str()).ok_or_else(|| format!("authority references unknown batch {}", authorization.batch_id))?;
        let assay = assays.get(authorization.batch_id.as_str()).ok_or_else(|| format!("authority precedes/omits assay for {}", authorization.batch_id))?;
        if authorization.valid_from_hour < assay.assessed_at_hour || authorization.valid_from_hour < receipt.received_at_hour ||
            authorization.valid_until_hour <= authorization.valid_from_hour {
            return Err(format!("invalid authority time window for {}", authorization.batch_id));
        }
        if authorization.disposition == AuthorityDisposition::Authorized {
            if assay.disposition != AssayDisposition::Passed || receipt.evidence != ReceiptEvidence::Verified {
                return Err(format!("authority cannot admit unverified or non-passing stock for {}", authorization.batch_id));
            }
            nonempty(&authorization.authority_ref, "authority evidence reference")?;
            if authorization.encumbered_grams > assay.released_lower_grams ||
                authorization.max_transfer_grams > assay.released_lower_grams - authorization.encumbered_grams {
                return Err(format!("authority exceeds released and unencumbered stock for {}", authorization.batch_id));
            }
        } else if authorization.max_transfer_grams != 0 {
            return Err(format!("withheld/conflicting authority cannot grant transfer capacity for {}", authorization.batch_id));
        }
        if authorizations.insert(authorization.batch_id.as_str(), authorization).is_some() {
            return Err(format!("duplicate authority record for batch {}", authorization.batch_id));
        }
    }

    let mut routes = BTreeMap::new();
    for route in &case.routes {
        nonempty(&route.route_id, "route ID")?;
        if route.capacity_grams_per_hour == 0 { return Err(format!("route {} has zero capacity", route.route_id)); }
        if routes.insert(route.route_id.as_str(), route).is_some() { return Err(format!("duplicate route {}", route.route_id)); }
    }

    let mut transfers_by_batch: BTreeMap<&str, Vec<&TransferReceipt>> = BTreeMap::new();
    let mut transfers_by_id = BTreeMap::new();
    let mut route_hour_usage: BTreeMap<(String, u32), u64> = BTreeMap::new();
    let mut transferred_by_batch: BTreeMap<&str, u64> = BTreeMap::new();
    for transfer in &case.transfers {
        event_id(&mut ids, &transfer.transfer_event_id)?;
        let receipt = receipts.get(transfer.batch_id.as_str()).ok_or_else(|| format!("transfer references unknown batch {}", transfer.batch_id))?;
        let assay = assays.get(transfer.batch_id.as_str()).ok_or_else(|| format!("transfer lacks assay for {}", transfer.batch_id))?;
        let authority = authorizations.get(transfer.batch_id.as_str()).ok_or_else(|| format!("transfer lacks authority for {}", transfer.batch_id))?;
        let route = routes.get(transfer.route_id.as_str()).ok_or_else(|| format!("transfer references unknown route {}", transfer.route_id))?;
        if receipt.evidence != ReceiptEvidence::Verified || assay.disposition != AssayDisposition::Passed ||
            authority.disposition != AuthorityDisposition::Authorized {
            return Err(format!("transfer attempts to promote unverified, unadmitted or unauthorized batch {}", transfer.batch_id));
        }
        if transfer.load_start_hour < receipt.received_at_hour || transfer.load_start_hour < assay.assessed_at_hour ||
            transfer.load_start_hour < authority.valid_from_hour || transfer.dispatch_hour <= transfer.load_start_hour ||
            transfer.dispatch_hour >= authority.valid_until_hour {
            return Err(format!("transfer occurs outside receipt/assay/authority chronology for {}", transfer.batch_id));
        }
        if transfer.arrival_hour < transfer.dispatch_hour.saturating_add(route.transit_delay_hours) {
            return Err(format!("transfer arrives before declared transit delay for {}", transfer.transfer_event_id));
        }
        if transfer.delivered_quantity_grams.checked_add(transfer.declared_route_loss_grams) != Some(transfer.source_quantity_grams) {
            return Err(format!("transfer mass balance fails for {}", transfer.transfer_event_id));
        }
        let duration = transfer.dispatch_hour - transfer.load_start_hour;
        let quotient = transfer.source_quantity_grams / u64::from(duration);
        let remainder = transfer.source_quantity_grams % u64::from(duration);
        for offset in 0..duration {
            let portion = quotient + if u64::from(offset) < remainder { 1 } else { 0 };
            let slot = route_hour_usage.entry((transfer.route_id.clone(), transfer.load_start_hour + offset)).or_insert(0);
            add(slot, portion, "route hourly throughput")?;
            if *slot > route.capacity_grams_per_hour {
                return Err(format!("route capacity exceeded on {} at hour {}", transfer.route_id, transfer.load_start_hour + offset));
            }
        }
        add(transferred_by_batch.entry(transfer.batch_id.as_str()).or_insert(0), transfer.source_quantity_grams, "batch transferred quantity")?;
        if transfers_by_id.insert(transfer.transfer_event_id.as_str(), transfer).is_some() {
            return Err(format!("duplicate transfer event {}", transfer.transfer_event_id));
        }
        transfers_by_batch.entry(transfer.batch_id.as_str()).or_default().push(transfer);
        // Keep a stable reference to the source receipt for subsequent checks.
        let _ = receipt;
    }

    for (batch_id, total_source) in &transferred_by_batch {
        let receipt = receipts.get(batch_id).ok_or_else(|| format!("missing receipt for {batch_id}"))?;
        let assay = assays.get(batch_id).ok_or_else(|| format!("missing assay for {batch_id}"))?;
        let authority = authorizations.get(batch_id).ok_or_else(|| format!("missing authority for {batch_id}"))?;
        let admitted = receipt_admitted(receipt, Some(assay));
        let accessible = admitted.saturating_sub(authority.encumbered_grams);
        if *total_source > accessible || *total_source > authority.max_transfer_grams {
            return Err(format!("transfers exceed accessible/authorized batch quantity for {batch_id}"));
        }
    }

    let mut issued_by_transfer: BTreeMap<&str, u64> = BTreeMap::new();
    for issue in &case.fuel_issues {
        event_id(&mut ids, &issue.issue_event_id)?;
        let transfer = transfers_by_id.get(issue.transfer_event_id.as_str()).ok_or_else(|| format!("fuel issue references unknown transfer {}", issue.transfer_event_id))?;
        if issue.issued_at_hour < transfer.arrival_hour { return Err(format!("fuel issue precedes generator receipt for {}", issue.issue_event_id)); }
        add(issued_by_transfer.entry(issue.transfer_event_id.as_str()).or_insert(0), issue.issued_quantity_grams, "generator fuel issue")?;
    }
    for (transfer_id, issued) in &issued_by_transfer {
        let transfer = transfers_by_id.get(transfer_id).ok_or_else(|| format!("missing transfer {transfer_id}"))?;
        if *issued > transfer.delivered_quantity_grams {
            return Err(format!("generator issue exceeds delivered quantity for {transfer_id}"));
        }
    }

    let mut hourly = Vec::with_capacity(HORIZON_HOURS as usize);
    for hour in 0..HORIZON_HOURS {
        let mut row = HourlyLedger {
            hour_index: hour, admitted_lower_grams: 0, terminal_inventory_grams: 0,
            terminal_accessible_grams: 0, in_transit_grams: 0, delivered_to_generator_grams: 0,
            generator_available_grams: 0, fuel_issued_grams: 0, route_loss_grams: 0,
        };
        for (batch_id, receipt) in &receipts {
            if receipt.received_at_hour > hour { continue; }
            let assay = assays.get(batch_id);
            let admitted = match assay {
                Some(value) if value.assessed_at_hour <= hour => receipt_admitted(receipt, Some(value)),
                _ => 0,
            };
            let mut dispatched = 0;
            for transfer in transfers_by_batch.get(batch_id).into_iter().flatten() {
                if transfer.dispatch_hour <= hour { add(&mut dispatched, transfer.source_quantity_grams, "hourly dispatched stock")?; }
                if transfer.dispatch_hour <= hour && transfer.arrival_hour > hour { add(&mut row.in_transit_grams, transfer.source_quantity_grams, "hourly in-transit stock")?; }
                if transfer.arrival_hour <= hour {
                    add(&mut row.delivered_to_generator_grams, transfer.delivered_quantity_grams, "hourly generator delivery")?;
                    add(&mut row.route_loss_grams, transfer.declared_route_loss_grams, "hourly route loss")?;
                }
            }
            let terminal = admitted.checked_sub(dispatched).ok_or_else(|| format!("terminal stock underflow for {batch_id} at hour {hour}"))?;
            add(&mut row.admitted_lower_grams, admitted, "hourly admitted stock")?;
            add(&mut row.terminal_inventory_grams, terminal, "hourly terminal stock")?;
            if let Some(auth) = authorizations.get(batch_id) {
                if auth.disposition == AuthorityDisposition::Authorized && auth.valid_from_hour <= hour && hour < auth.valid_until_hour {
                    let accessible = admitted.saturating_sub(auth.encumbered_grams).saturating_sub(dispatched)
                        .min(auth.max_transfer_grams.saturating_sub(dispatched));
                    add(&mut row.terminal_accessible_grams, accessible, "hourly accessible stock")?;
                }
            }
        }
        for issue in &case.fuel_issues {
            if issue.issued_at_hour <= hour { add(&mut row.fuel_issued_grams, issue.issued_quantity_grams, "hourly fuel issue")?; }
        }
        row.generator_available_grams = row.delivered_to_generator_grams.checked_sub(row.fuel_issued_grams)
            .ok_or_else(|| format!("generator issue exceeds delivered fuel at hour {hour}"))?;
        let accounted = row.terminal_inventory_grams
            .checked_add(row.in_transit_grams).and_then(|v| v.checked_add(row.delivered_to_generator_grams))
            .and_then(|v| v.checked_add(row.route_loss_grams))
            .ok_or_else(|| "hourly inventory conservation overflow".to_string())?;
        if accounted != row.admitted_lower_grams {
            return Err(format!("hourly mass balance fails at hour {hour}: admitted {} != accounted {accounted}", row.admitted_lower_grams));
        }
        hourly.push(row);
    }

    let mut batches = Vec::with_capacity(receipts.len());
    for (batch_id, receipt) in &receipts {
        let assay = assays.get(batch_id);
        let admitted = receipt_admitted(receipt, assay.copied());
        let quarantined = if receipt.evidence == ReceiptEvidence::Verified { receipt.received_lower_grams - admitted } else { 0 };
        let unverified = if receipt.evidence == ReceiptEvidence::Verified { 0 } else { receipt.received_lower_grams };
        let mut transferred = 0;
        let mut delivered = 0;
        let mut loss = 0;
        for transfer in transfers_by_batch.get(batch_id).into_iter().flatten() {
            add(&mut transferred, transfer.source_quantity_grams, "batch transfer total")?;
            add(&mut delivered, transfer.delivered_quantity_grams, "batch delivery total")?;
            add(&mut loss, transfer.declared_route_loss_grams, "batch route loss total")?;
        }
        let mut issued = 0;
        for issue in &case.fuel_issues {
            if transfers_by_id.get(issue.transfer_event_id.as_str()).is_some_and(|transfer| transfer.batch_id == *batch_id) {
                add(&mut issued, issue.issued_quantity_grams, "batch issue total")?;
            }
        }
        let terminal_close = admitted.checked_sub(transferred).ok_or_else(|| format!("negative terminal closing stock for {batch_id}"))?;
        if transferred != delivered.saturating_add(loss) || delivered < issued {
            return Err(format!("batch balance mismatch for {batch_id}"));
        }
        batches.push(BatchBalance {
            batch_id: (*batch_id).to_owned(), received_lower_grams: receipt.received_lower_grams,
            admitted_grams: admitted, quarantined_or_unreleased_grams: quarantined,
            unverified_grams: unverified, terminal_closing_grams: terminal_close,
            transferred_source_grams: transferred, delivered_grams: delivered,
            route_loss_grams: loss, generator_closing_grams: delivered - issued, fuel_issued_grams: issued,
        });
    }
    Ok(Projection { hourly, batches })
}

fn q(value: &str) -> String {
    let mut out = String::from("\"");
    for ch in value.chars() {
        match ch {
            '\"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c if c.is_control() => out.push(' '),
            c => out.push(c),
        }
    }
    out.push('\"');
    out
}

impl CargoCase {
    /// Planner projection at hour-zero: manifest facts only. No receipts, assays,
    /// access decisions, transfers, fuel issues or oracle-derived inventory are exposed.
    pub fn planner_json(&self) -> String {
        let manifests = self.manifests.iter().filter(|m| m.known_at_hour == 0).map(|m| format!(
            "{{\"cargoId\":{},\"supplierId\":{},\"terminalId\":{},\"product\":{},\"grade\":{},\"plannedQuantityGrams\":{},\"declaredEtaHour\":{},\"knownAtHour\":{},\"commonCauseGroup\":{},\"documentaryStatus\":{}}}",
            q(&m.cargo_id), q(&m.supplier_id), q(&m.terminal_id), q(&m.product), q(&m.grade),
            m.planned_quantity_grams, m.declared_eta_hour, m.known_at_hour, q(&m.common_cause_group), q(&m.documentary_status)
        )).collect::<Vec<_>>().join(",");
        format!("{{\"schemaVersion\":\"0.1.0\",\"recordKind\":\"planner_observation\",\"caseId\":{},\"horizonHours\":72,\"evidenceMaturity\":\"synthetic_fixture\",\"manifests\":[{}]}}", q(&self.case_id), manifests)
    }

    pub fn oracle_json(&self, projection: &Projection) -> String {
        let receipts = self.receipts.iter().map(|r| format!(
            "{{\"receiptEventId\":{},\"cargoId\":{},\"batchId\":{},\"terminalId\":{},\"product\":{},\"grade\":{},\"receivedAtHour\":{},\"receivedLowerGrams\":{},\"actualReceivedGrams\":{},\"receivedUpperGrams\":{},\"evidence\":{},\"evidenceRef\":{},\"evidenceDigest\":{}}}",
            q(&r.receipt_event_id), q(&r.cargo_id), q(&r.batch_id), q(&r.terminal_id), q(&r.product), q(&r.grade),
            r.received_at_hour, r.received_lower_grams, r.actual_received_grams, r.received_upper_grams,
            q(r.evidence.as_str()), q(&r.evidence_ref), q(&r.evidence_digest)
        )).collect::<Vec<_>>().join(",");
        let assays = self.quality_assessments.iter().map(|a| format!(
            "{{\"assayEventId\":{},\"batchId\":{},\"assessedAtHour\":{},\"disposition\":{},\"releasedLowerGrams\":{},\"sampleChainRef\":{},\"evidenceRef\":{}}}",
            q(&a.assay_event_id), q(&a.batch_id), a.assessed_at_hour, q(a.disposition.as_str()),
            a.released_lower_grams, q(&a.sample_chain_ref), q(&a.evidence_ref)
        )).collect::<Vec<_>>().join(",");
        let auth = self.authorizations.iter().map(|a| format!(
            "{{\"authorityEventId\":{},\"batchId\":{},\"validFromHour\":{},\"validUntilHour\":{},\"disposition\":{},\"encumberedGrams\":{},\"maxTransferGrams\":{},\"authorityRef\":{}}}",
            q(&a.authority_event_id), q(&a.batch_id), a.valid_from_hour, a.valid_until_hour,
            q(a.disposition.as_str()), a.encumbered_grams, a.max_transfer_grams, q(&a.authority_ref)
        )).collect::<Vec<_>>().join(",");
        let routes = self.routes.iter().map(|r| format!(
            "{{\"routeId\":{},\"capacityGramsPerHour\":{},\"transitDelayHours\":{}}}",
            q(&r.route_id), r.capacity_grams_per_hour, r.transit_delay_hours
        )).collect::<Vec<_>>().join(",");
        let transfers = self.transfers.iter().map(|t| format!(
            "{{\"transferEventId\":{},\"batchId\":{},\"routeId\":{},\"loadStartHour\":{},\"dispatchHour\":{},\"arrivalHour\":{},\"sourceQuantityGrams\":{},\"deliveredQuantityGrams\":{},\"declaredRouteLossGrams\":{}}}",
            q(&t.transfer_event_id), q(&t.batch_id), q(&t.route_id), t.load_start_hour, t.dispatch_hour, t.arrival_hour,
            t.source_quantity_grams, t.delivered_quantity_grams, t.declared_route_loss_grams
        )).collect::<Vec<_>>().join(",");
        let issues = self.fuel_issues.iter().map(|i| format!(
            "{{\"issueEventId\":{},\"transferEventId\":{},\"issuedAtHour\":{},\"issuedQuantityGrams\":{}}}",
            q(&i.issue_event_id), q(&i.transfer_event_id), i.issued_at_hour, i.issued_quantity_grams
        )).collect::<Vec<_>>().join(",");
        let hourly = projection.hourly.iter().map(|h| format!(
            "{{\"hourIndex\":{},\"admittedLowerGrams\":{},\"terminalInventoryGrams\":{},\"terminalAccessibleGrams\":{},\"inTransitGrams\":{},\"deliveredToGeneratorGrams\":{},\"generatorAvailableGrams\":{},\"fuelIssuedGrams\":{},\"routeLossGrams\":{}}}",
            h.hour_index, h.admitted_lower_grams, h.terminal_inventory_grams, h.terminal_accessible_grams,
            h.in_transit_grams, h.delivered_to_generator_grams, h.generator_available_grams, h.fuel_issued_grams, h.route_loss_grams
        )).collect::<Vec<_>>().join(",");
        let balances = projection.batches.iter().map(|b| format!(
            "{{\"batchId\":{},\"receivedLowerGrams\":{},\"admittedGrams\":{},\"quarantinedOrUnreleasedGrams\":{},\"unverifiedGrams\":{},\"terminalClosingGrams\":{},\"transferredSourceGrams\":{},\"deliveredGrams\":{},\"routeLossGrams\":{},\"generatorClosingGrams\":{},\"fuelIssuedGrams\":{}}}",
            q(&b.batch_id), b.received_lower_grams, b.admitted_grams, b.quarantined_or_unreleased_grams,
            b.unverified_grams, b.terminal_closing_grams, b.transferred_source_grams, b.delivered_grams,
            b.route_loss_grams, b.generator_closing_grams, b.fuel_issued_grams
        )).collect::<Vec<_>>().join(",");
        format!(
            "{{\"schemaVersion\":\"0.1.0\",\"recordKind\":\"oracle_truth\",\"caseId\":{},\"horizonHours\":72,\"evidenceMaturity\":\"synthetic_fixture\",\"receipts\":[{}],\"qualityAssessments\":[{}],\"authorizations\":[{}],\"routes\":[{}],\"transfers\":[{}],\"fuelIssues\":[{}],\"batchBalances\":[{}],\"hourlyTrajectory\":[{}]}}",
            q(&self.case_id), receipts, assays, auth, routes, transfers, issues, balances, hourly
        )
    }
}

pub fn report_json(cases: &[CargoCase], projections: &[Projection]) -> String {
    let mut manifests = 0_u64;
    let mut receipts = 0_u64;
    let mut admitted = 0_u64;
    let mut delivered = 0_u64;
    let mut issued = 0_u64;
    for (case, projection) in cases.iter().zip(projections) {
        manifests += case.manifests.len() as u64;
        receipts += case.receipts.len() as u64;
        for batch in &projection.batches {
            admitted += batch.admitted_grams;
            delivered += batch.delivered_grams;
            issued += batch.fuel_issued_grams;
        }
    }
    format!(
        "{{\"reportVersion\":\"0.1.0\",\"evidenceMaturity\":\"synthetic_fixture\",\"caseCount\":{},\"manifestCount\":{},\"receiptCount\":{},\"admittedLowerGrams\":{},\"deliveredToGeneratorGrams\":{},\"fuelIssuedGrams\":{},\"plannerOracleFilesSeparate\":true,\"horizonHours\":72}}",
        cases.len(), manifests, receipts, admitted, delivered, issued
    )
}

fn make_manifest(cargo_id: &str, eta: u32) -> CargoManifest {
    CargoManifest {
        cargo_id: cargo_id.into(), supplier_id: "synthetic-supplier-a".into(),
        terminal_id: "synthetic-terminal-a".into(), product: "diesel".into(), grade: "ULSD-10ppm".into(),
        planned_quantity_grams: 1_000_000, declared_eta_hour: eta, known_at_hour: 0,
        common_cause_group: "synthetic-port-supplier".into(), documentary_status: "declared_only".into(),
    }
}
fn receipt(cargo: &str, batch: &str, at: u32, lower: u64, actual: u64, upper: u64, evidence: ReceiptEvidence) -> PhysicalReceipt {
    let verified = evidence == ReceiptEvidence::Verified;
    PhysicalReceipt {
        receipt_event_id: format!("receipt-{batch}"), cargo_id: cargo.into(), batch_id: batch.into(),
        terminal_id: "synthetic-terminal-a".into(), product: "diesel".into(), grade: "ULSD-10ppm".into(),
        received_at_hour: at, received_lower_grams: lower, actual_received_grams: actual, received_upper_grams: upper,
        evidence, evidence_ref: if verified { format!("synthetic-receipt-record-{batch}") } else { String::new() },
        evidence_digest: if verified { format!("sha256:synthetic-{batch}") } else { String::new() },
    }
}
fn assay(batch: &str, at: u32, disposition: AssayDisposition, release: u64) -> QualityAssessment {
    let passed = disposition == AssayDisposition::Passed;
    QualityAssessment {
        assay_event_id: format!("assay-{batch}"), batch_id: batch.into(), assessed_at_hour: at,
        disposition, released_lower_grams: release,
        sample_chain_ref: if passed { format!("sample-chain-{batch}") } else { String::new() },
        evidence_ref: if passed { format!("synthetic-assay-{batch}") } else { String::new() },
    }
}
fn authorization(batch: &str, from: u32, until: u32, disposition: AuthorityDisposition, encumbered: u64, max_transfer: u64) -> AccessAuthorization {
    let authorized = disposition == AuthorityDisposition::Authorized;
    AccessAuthorization {
        authority_event_id: format!("authority-{batch}"), batch_id: batch.into(),
        valid_from_hour: from, valid_until_hour: until, disposition, encumbered_grams: encumbered,
        max_transfer_grams: max_transfer, authority_ref: if authorized { format!("synthetic-authority-{batch}") } else { String::new() },
    }
}
fn transfer(id: &str, batch: &str, route: &str, load_start: u32, dispatch: u32, arrival: u32, source: u64, delivered: u64) -> TransferReceipt {
    TransferReceipt {
        transfer_event_id: id.into(), batch_id: batch.into(), route_id: route.into(),
        load_start_hour: load_start, dispatch_hour: dispatch, arrival_hour: arrival,
        source_quantity_grams: source, delivered_quantity_grams: delivered,
        declared_route_loss_grams: source - delivered,
    }
}
fn base_case(case_id: &str, eta: u32) -> CargoCase {
    CargoCase {
        case_id: case_id.into(), manifests: vec![make_manifest(&format!("{case_id}-cargo"), eta)],
        receipts: Vec::new(), quality_assessments: Vec::new(), authorizations: Vec::new(),
        routes: Vec::new(), transfers: Vec::new(), fuel_issues: Vec::new(),
    }
}

pub fn synthetic_cases() -> Vec<CargoCase> {
    let mut cases = Vec::new();

    let mut on_time = base_case("verified-on-time", 4);
    on_time.receipts.push(receipt("verified-on-time-cargo", "batch-on-time", 4, 880_000, 900_000, 920_000, ReceiptEvidence::Verified));
    on_time.quality_assessments.push(assay("batch-on-time", 5, AssayDisposition::Passed, 850_000));
    on_time.authorizations.push(authorization("batch-on-time", 6, 60, AuthorityDisposition::Authorized, 50_000, 800_000));
    on_time.routes.push(TransferRoute { route_id: "line-a".into(), capacity_grams_per_hour: 250_000, transit_delay_hours: 2 });
    on_time.transfers.push(transfer("transfer-on-time", "batch-on-time", "line-a", 6, 8, 10, 400_000, 390_000));
    on_time.fuel_issues.push(GeneratorFuelIssue { issue_event_id: "issue-on-time".into(), transfer_event_id: "transfer-on-time".into(), issued_at_hour: 12, issued_quantity_grams: 100_000 });
    cases.push(on_time);

    let mut partial = base_case("partial-late-receipt", 2);
    partial.receipts.push(receipt("partial-late-receipt-cargo", "batch-partial", 12, 380_000, 400_000, 420_000, ReceiptEvidence::Verified));
    partial.quality_assessments.push(assay("batch-partial", 13, AssayDisposition::Passed, 350_000));
    partial.authorizations.push(authorization("batch-partial", 14, 60, AuthorityDisposition::Authorized, 50_000, 250_000));
    partial.routes.push(TransferRoute { route_id: "line-b".into(), capacity_grams_per_hour: 100_000, transit_delay_hours: 4 });
    partial.transfers.push(transfer("transfer-partial", "batch-partial", "line-b", 16, 18, 22, 200_000, 195_000));
    partial.fuel_issues.push(GeneratorFuelIssue { issue_event_id: "issue-partial".into(), transfer_event_id: "transfer-partial".into(), issued_at_hour: 24, issued_quantity_grams: 100_000 });
    cases.push(partial);

    let mut unverified = base_case("unverified-receipt", 3);
    unverified.receipts.push(receipt("unverified-receipt-cargo", "batch-unverified", 6, 490_000, 500_000, 510_000, ReceiptEvidence::Missing));
    cases.push(unverified);

    let mut failed_quality = base_case("quality-rejected", 1);
    failed_quality.receipts.push(receipt("quality-rejected-cargo", "batch-rejected", 2, 790_000, 800_000, 810_000, ReceiptEvidence::Verified));
    failed_quality.quality_assessments.push(assay("batch-rejected", 3, AssayDisposition::Failed, 0));
    failed_quality.authorizations.push(authorization("batch-rejected", 4, 50, AuthorityDisposition::Withheld, 0, 0));
    cases.push(failed_quality);

    let mut withheld = base_case("authority-withheld", 1);
    withheld.receipts.push(receipt("authority-withheld-cargo", "batch-withheld", 2, 590_000, 600_000, 610_000, ReceiptEvidence::Verified));
    withheld.quality_assessments.push(assay("batch-withheld", 3, AssayDisposition::Passed, 550_000));
    withheld.authorizations.push(authorization("batch-withheld", 4, 60, AuthorityDisposition::Withheld, 0, 0));
    cases.push(withheld);

    let mut after = base_case("after-horizon", 96);
    after.receipts.push(receipt("after-horizon-cargo", "batch-after", 80, 690_000, 700_000, 710_000, ReceiptEvidence::Verified));
    after.quality_assessments.push(assay("batch-after", 81, AssayDisposition::Passed, 650_000));
    after.authorizations.push(authorization("batch-after", 82, 110, AuthorityDisposition::Authorized, 0, 600_000));
    after.routes.push(TransferRoute { route_id: "line-late".into(), capacity_grams_per_hour: 100_000, transit_delay_hours: 3 });
    after.transfers.push(transfer("transfer-after", "batch-after", "line-late", 82, 84, 90, 100_000, 98_000));
    after.fuel_issues.push(GeneratorFuelIssue { issue_event_id: "issue-after".into(), transfer_event_id: "transfer-after".into(), issued_at_hour: 92, issued_quantity_grams: 50_000 });
    cases.push(after);

    let mut constrained = base_case("route-capacity-constrained", 1);
    constrained.receipts.push(receipt("route-capacity-constrained-cargo", "batch-route", 1, 590_000, 600_000, 610_000, ReceiptEvidence::Verified));
    constrained.quality_assessments.push(assay("batch-route", 2, AssayDisposition::Passed, 550_000));
    constrained.authorizations.push(authorization("batch-route", 3, 60, AuthorityDisposition::Authorized, 50_000, 400_000));
    constrained.routes.push(TransferRoute { route_id: "line-c".into(), capacity_grams_per_hour: 50_000, transit_delay_hours: 1 });
    constrained.transfers.push(transfer("transfer-route-1", "batch-route", "line-c", 4, 6, 7, 100_000, 99_000));
    constrained.transfers.push(transfer("transfer-route-2", "batch-route", "line-c", 6, 8, 9, 100_000, 99_000));
    constrained.fuel_issues.push(GeneratorFuelIssue { issue_event_id: "issue-route".into(), transfer_event_id: "transfer-route-1".into(), issued_at_hour: 8, issued_quantity_grams: 40_000 });
    cases.push(constrained);

    let mut pending = base_case("assay-pending", 1);
    pending.receipts.push(receipt("assay-pending-cargo", "batch-pending", 1, 490_000, 500_000, 510_000, ReceiptEvidence::Verified));
    pending.quality_assessments.push(assay("batch-pending", 4, AssayDisposition::Pending, 0));
    cases.push(pending);

    let mut conflict = base_case("conflicting-receipt", 1);
    conflict.receipts.push(receipt("conflicting-receipt-cargo", "batch-conflict", 2, 490_000, 500_000, 510_000, ReceiptEvidence::Conflicting));
    cases.push(conflict);

    cases
}

#[cfg(test)]
mod tests {
    use super::*;

    fn find(case_id: &str) -> CargoCase {
        synthetic_cases().into_iter().find(|case| case.case_id == case_id).expect("fixture exists")
    }

    #[test]
    fn all_synthetic_cases_validate_and_reconcile() {
        let cases = synthetic_cases();
        assert_eq!(cases.len(), 9);
        for case in &cases {
            let projection = validate_and_project(case).expect("valid synthetic case");
            assert_eq!(projection.hourly.len(), HORIZON_HOURS as usize);
            for row in &projection.hourly {
                assert_eq!(
                    row.admitted_lower_grams,
                    row.terminal_inventory_grams + row.in_transit_grams
                        + row.delivered_to_generator_grams + row.route_loss_grams
                );
                assert_eq!(row.delivered_to_generator_grams, row.generator_available_grams + row.fuel_issued_grams);
            }
        }
    }

    #[test]
    fn unverified_and_conflicting_receipts_never_become_admitted_stock() {
        for id in ["unverified-receipt", "conflicting-receipt"] {
            let case = find(id);
            let projection = validate_and_project(&case).expect("case remains evaluable");
            assert!(projection.hourly.iter().all(|row| row.admitted_lower_grams == 0));
        }
    }

    #[test]
    fn a_late_receipt_is_not_backdated_into_earlier_service_hours() {
        let case = find("partial-late-receipt");
        let projection = validate_and_project(&case).expect("valid case");
        assert!(projection.hourly[..12].iter().all(|row| row.admitted_lower_grams == 0));
        assert_eq!(projection.hourly[12].admitted_lower_grams, 350_000);
        assert!(projection.hourly[..22].iter().all(|row| row.delivered_to_generator_grams == 0));
        assert_eq!(projection.hourly[22].delivered_to_generator_grams, 195_000);
    }

    #[test]
    fn arrival_at_horizon_end_is_outside_the_half_open_service_window() {
        let mut case = find("after-horizon");
        case.receipts[0].received_at_hour = HORIZON_HOURS;
        case.quality_assessments[0].assessed_at_hour = HORIZON_HOURS + 1;
        case.authorizations[0].valid_from_hour = HORIZON_HOURS + 2;
        case.transfers[0].load_start_hour = HORIZON_HOURS + 2;
        case.transfers[0].dispatch_hour = HORIZON_HOURS + 4;
        case.transfers[0].arrival_hour = HORIZON_HOURS + 7;
        case.fuel_issues[0].issued_at_hour = HORIZON_HOURS + 8;
        let projection = validate_and_project(&case).expect("valid boundary case");
        assert!(projection.hourly.iter().all(|row| row.admitted_lower_grams == 0));
        assert!(projection.hourly.iter().all(|row| row.delivered_to_generator_grams == 0));
    }

    #[test]
    fn replayed_event_ids_are_rejected() {
        let mut case = find("verified-on-time");
        case.fuel_issues.push(GeneratorFuelIssue {
            issue_event_id: "transfer-on-time".into(),
            transfer_event_id: "transfer-on-time".into(),
            issued_at_hour: 12,
            issued_quantity_grams: 1,
        });
        assert!(validate_and_project(&case).is_err());
    }

    #[test]
    fn product_or_grade_substitution_is_rejected() {
        let mut case = find("verified-on-time");
        case.receipts[0].grade = "wrong-grade".into();
        assert!(validate_and_project(&case).is_err());
    }

    #[test]
    fn loading_capacity_is_enforced_per_route_hour() {
        let mut case = find("route-capacity-constrained");
        case.transfers[1].load_start_hour = 4;
        case.transfers[1].dispatch_hour = 6;
        case.transfers[1].arrival_hour = 7;
        assert!(validate_and_project(&case).is_err());
    }

    #[test]
    fn no_transfer_can_precede_quality_and_authority() {
        let mut case = find("verified-on-time");
        case.transfers[0].load_start_hour = 4;
        case.transfers[0].dispatch_hour = 5;
        case.transfers[0].arrival_hour = 7;
        assert!(validate_and_project(&case).is_err());
    }

    #[test]
    fn fuel_issue_cannot_precede_generator_delivery() {
        let mut case = find("verified-on-time");
        case.fuel_issues[0].issued_at_hour = 9;
        assert!(validate_and_project(&case).is_err());
    }

    #[test]
    fn interval_widening_cannot_increase_admitted_quantity() {
        let mut case = find("verified-on-time");
        let before = validate_and_project(&case).expect("baseline").batches[0].admitted_grams;
        case.receipts[0].received_lower_grams -= 100_000;
        case.quality_assessments[0].released_lower_grams -= 100_000;
        case.authorizations[0].max_transfer_grams = 700_000;
        let after = validate_and_project(&case).expect("widened interval").batches[0].admitted_grams;
        assert!(after <= before);
    }

    #[test]
    fn transfer_and_generator_issue_balances_reconcile() {
        let projection = validate_and_project(&find("verified-on-time")).expect("valid case");
        let balance = projection.batches.first().expect("batch balance");
        assert_eq!(balance.transferred_source_grams, balance.delivered_grams + balance.route_loss_grams);
        assert_eq!(balance.delivered_grams, balance.generator_closing_grams + balance.fuel_issued_grams);
    }
}
