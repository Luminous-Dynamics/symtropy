use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const VALIDATOR_VERSION: &str = "0.1.0";
const MAX_STOCKS: usize = 12;
const MAX_PRODUCTS: usize = 6;
const MAX_DEMAND_PER_PRODUCT: u32 = 12;
const MAX_DIMENSION_MM: u64 = 100_000;
const MAX_SEARCH_NODES: u64 = 150_000;
const MAX_COMPLETE_PLANS: usize = 5_000;
const REQUIRED_SCENARIOS: [&str; 3] = [
    "VALUE_LENGTH_OFFCUT_TRADEOFF",
    "MULTI_STOCK_DEMAND_CAP",
    "KERF_EDGE_AND_UNUSABLE_RESIDUAL",
];

type Check<T> = Result<T, String>;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Corpus {
    suite_id: String,
    suite_version: String,
    manifest_kind: String,
    claim_ceiling: String,
    units: Units,
    scenarios: Vec<Scenario>,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Units {
    stock_length: String,
    kerf: String,
    value: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct Scenario {
    scenario_id: String,
    evidence_class: String,
    stock_lengths_mm: Vec<u64>,
    kerf_mm: u64,
    minimum_reusable_offcut_mm: u64,
    minimum_pieces_required: u32,
    products: Vec<ProductSpec>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct ProductSpec {
    product_id: String,
    length_mm: u64,
    demand_units: u32,
    unit_value_microcredits: u64,
    application: Application,
    grade_evidence_state: GradeEvidenceState,
    inspection_pass: bool,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum Application {
    Structural,
    NonStructural,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum GradeEvidenceState {
    Qualified,
    Observed,
    Estimated,
    Missing,
    Unknown,
}

impl ProductSpec {
    fn eligible_for_demand(&self) -> bool {
        match self.application {
            Application::Structural => {
                self.grade_evidence_state == GradeEvidenceState::Qualified && self.inspection_pass
            }
            Application::NonStructural => true,
        }
    }
}

#[derive(Debug, Clone)]
struct Pattern {
    counts: Vec<u32>,
    product_length_mm: u64,
    piece_count: u32,
    kerf_loss_mm: u64,
    reusable_offcut_mm: u64,
    unusable_residual_mm: u64,
    unprocessed_stock_length_mm: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct StockPatternReceipt {
    stock_index: usize,
    stock_length_mm: u64,
    product_counts: BTreeMap<String, u32>,
    product_length_mm: u64,
    piece_count: u32,
    kerf_loss_mm: u64,
    reusable_offcut_mm: u64,
    unusable_residual_mm: u64,
    unprocessed_stock_length_mm: u64,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
struct PlanReceipt {
    product_counts: BTreeMap<String, u32>,
    sale_value_microcredits: u64,
    product_length_mm: u64,
    fulfilled_demand_units: u32,
    reusable_offcut_length_mm: u64,
    unusable_residual_length_mm: u64,
    kerf_loss_mm: u64,
    unrecoverable_loss_length_mm: u64,
    unprocessed_stock_length_mm: u64,
    stock_patterns: Vec<StockPatternReceipt>,
}

#[derive(Debug, Clone, Serialize)]
struct GreedyBaselineReceipt {
    strategy: &'static str,
    meets_minimum_pieces: bool,
    failure_reason: Option<String>,
    plan: PlanReceipt,
}

#[derive(Debug, Clone, Serialize)]
struct ScenarioReceipt {
    scenario_id: String,
    status: &'static str,
    eligible_product_ids: Vec<String>,
    excluded_structural_product_ids: Vec<String>,
    search_nodes: u64,
    feasible_plan_count: usize,
    pareto_plan_count: usize,
    baseline_is_covered_by_pareto_frontier: bool,
    greedy_baseline: GreedyBaselineReceipt,
    pareto_plans: Vec<PlanReceipt>,
}

#[derive(Debug, Clone)]
struct PartialPlan {
    product_counts: Vec<u32>,
    sale_value_microcredits: u64,
    product_length_mm: u64,
    fulfilled_demand_units: u32,
    reusable_offcut_length_mm: u64,
    unusable_residual_length_mm: u64,
    kerf_loss_mm: u64,
    unprocessed_stock_length_mm: u64,
    stock_patterns: Vec<StockPatternReceipt>,
}

impl PartialPlan {
    fn empty(product_count: usize) -> Self {
        Self {
            product_counts: vec![0; product_count],
            sale_value_microcredits: 0,
            product_length_mm: 0,
            fulfilled_demand_units: 0,
            reusable_offcut_length_mm: 0,
            unusable_residual_length_mm: 0,
            kerf_loss_mm: 0,
            unprocessed_stock_length_mm: 0,
            stock_patterns: Vec::new(),
        }
    }

    fn into_receipt(self, products: &[ProductSpec]) -> PlanReceipt {
        let product_counts = products
            .iter()
            .enumerate()
            .map(|(index, product)| (product.product_id.clone(), self.product_counts[index]))
            .collect();
        let unrecoverable_loss_length_mm = self
            .kerf_loss_mm
            .checked_add(self.unusable_residual_length_mm)
            .expect("loss sum was checked during evaluation");
        PlanReceipt {
            product_counts,
            sale_value_microcredits: self.sale_value_microcredits,
            product_length_mm: self.product_length_mm,
            fulfilled_demand_units: self.fulfilled_demand_units,
            reusable_offcut_length_mm: self.reusable_offcut_length_mm,
            unusable_residual_length_mm: self.unusable_residual_length_mm,
            kerf_loss_mm: self.kerf_loss_mm,
            unrecoverable_loss_length_mm,
            unprocessed_stock_length_mm: self.unprocessed_stock_length_mm,
            stock_patterns: self.stock_patterns,
        }
    }
}

#[derive(Debug)]
struct SearchBudget {
    nodes: u64,
    limit: u64,
}

impl SearchBudget {
    fn tick(&mut self, context: &str) -> Check<()> {
        self.nodes = self
            .nodes
            .checked_add(1)
            .ok_or_else(|| "search-node counter overflow".to_owned())?;
        if self.nodes > self.limit {
            return Err(format!(
                "search budget exhausted in {context}: nodes={} limit={}; no partial optimum returned",
                self.nodes, self.limit
            ));
        }
        Ok(())
    }
}

fn validate_scenario(scenario: &Scenario) -> Check<()> {
    if scenario.scenario_id.trim().is_empty() {
        return Err("scenario_id must not be empty".to_owned());
    }
    if scenario.evidence_class != "synthetic" {
        return Err(format!(
            "{}: v0.1 accepts only synthetic fixtures",
            scenario.scenario_id
        ));
    }
    if scenario.stock_lengths_mm.is_empty() || scenario.stock_lengths_mm.len() > MAX_STOCKS {
        return Err(format!(
            "{}: stock count must be within 1..={MAX_STOCKS}",
            scenario.scenario_id
        ));
    }
    if scenario.products.is_empty() || scenario.products.len() > MAX_PRODUCTS {
        return Err(format!(
            "{}: product type count must be within 1..={MAX_PRODUCTS}",
            scenario.scenario_id
        ));
    }
    if scenario.kerf_mm > MAX_DIMENSION_MM
        || scenario.minimum_reusable_offcut_mm > MAX_DIMENSION_MM
    {
        return Err(format!(
            "{}: kerf or offcut threshold exceeds the dimension bound",
            scenario.scenario_id
        ));
    }
    if scenario.minimum_pieces_required == 0 {
        return Err(format!(
            "{}: minimum_pieces_required must be positive",
            scenario.scenario_id
        ));
    }
    if scenario
        .stock_lengths_mm
        .iter()
        .any(|length| *length == 0 || *length > MAX_DIMENSION_MM)
    {
        return Err(format!(
            "{}: every stock length must be within 1..={MAX_DIMENSION_MM} mm",
            scenario.scenario_id
        ));
    }

    let mut product_ids = BTreeSet::new();
    let mut eligible_demand = 0_u32;
    for product in &scenario.products {
        if product.product_id.trim().is_empty() {
            return Err(format!(
                "{}: product_id must not be empty",
                scenario.scenario_id
            ));
        }
        if !product_ids.insert(product.product_id.clone()) {
            return Err(format!(
                "{}: duplicate product_id {}",
                scenario.scenario_id, product.product_id
            ));
        }
        if product.length_mm == 0 || product.length_mm > MAX_DIMENSION_MM {
            return Err(format!(
                "{}.{}: product length must be within 1..={MAX_DIMENSION_MM} mm",
                scenario.scenario_id, product.product_id
            ));
        }
        if product.demand_units == 0 || product.demand_units > MAX_DEMAND_PER_PRODUCT {
            return Err(format!(
                "{}.{}: demand must be within 1..={MAX_DEMAND_PER_PRODUCT}",
                scenario.scenario_id, product.product_id
            ));
        }
        if product.eligible_for_demand() {
            eligible_demand = eligible_demand
                .checked_add(product.demand_units)
                .ok_or_else(|| "eligible demand count overflow".to_owned())?;
        }
    }
    if scenario.minimum_pieces_required > eligible_demand {
        return Err(format!(
            "{}: minimum pieces exceed all eligible product demand",
            scenario.scenario_id
        ));
    }
    Ok(())
}

fn make_pattern(
    stock_length_mm: u64,
    kerf_mm: u64,
    minimum_reusable_offcut_mm: u64,
    counts: &[u32],
    products: &[ProductSpec],
) -> Check<Pattern> {
    let mut product_length_mm = 0_u64;
    let mut piece_count = 0_u32;
    for (index, quantity) in counts.iter().enumerate() {
        if *quantity == 0 {
            continue;
        }
        let piece_length = products[index]
            .length_mm
            .checked_mul(u64::from(*quantity))
            .ok_or_else(|| "product length multiplication overflow".to_owned())?;
        product_length_mm = product_length_mm
            .checked_add(piece_length)
            .ok_or_else(|| "pattern product length overflow".to_owned())?;
        piece_count = piece_count
            .checked_add(*quantity)
            .ok_or_else(|| "pattern piece count overflow".to_owned())?;
    }

    if piece_count == 0 {
        return Ok(Pattern {
            counts: counts.to_vec(),
            product_length_mm: 0,
            piece_count: 0,
            kerf_loss_mm: 0,
            reusable_offcut_mm: 0,
            unusable_residual_mm: 0,
            unprocessed_stock_length_mm: stock_length_mm,
        });
    }

    let cuts = u64::from(piece_count - 1);
    let kerf_loss_mm = kerf_mm
        .checked_mul(cuts)
        .ok_or_else(|| "kerf loss multiplication overflow".to_owned())?;
    let used = product_length_mm
        .checked_add(kerf_loss_mm)
        .ok_or_else(|| "pattern total use overflow".to_owned())?;
    if used > stock_length_mm {
        return Err("internal error: generated infeasible cut pattern".to_owned());
    }
    let remainder = stock_length_mm - used;
    let (reusable_offcut_mm, unusable_residual_mm) = if remainder == 0 {
        (0, 0)
    } else if remainder >= minimum_reusable_offcut_mm {
        (remainder, 0)
    } else {
        (0, remainder)
    };
    Ok(Pattern {
        counts: counts.to_vec(),
        product_length_mm,
        piece_count,
        kerf_loss_mm,
        reusable_offcut_mm,
        unusable_residual_mm,
        unprocessed_stock_length_mm: 0,
    })
}

fn enumerate_patterns(
    stock_length_mm: u64,
    kerf_mm: u64,
    minimum_reusable_offcut_mm: u64,
    remaining_demand: &[u32],
    eligible: &[bool],
    products: &[ProductSpec],
    budget: &mut SearchBudget,
) -> Check<Vec<Pattern>> {
    fn recurse(
        index: usize,
        stock_length_mm: u64,
        kerf_mm: u64,
        minimum_reusable_offcut_mm: u64,
        remaining_demand: &[u32],
        eligible: &[bool],
        products: &[ProductSpec],
        counts: &mut [u32],
        output: &mut Vec<Pattern>,
        budget: &mut SearchBudget,
    ) -> Check<()> {
        budget.tick("pattern enumeration")?;
        if index == products.len() {
            output.push(make_pattern(
                stock_length_mm,
                kerf_mm,
                minimum_reusable_offcut_mm,
                counts,
                products,
            )?);
            return Ok(());
        }

        if !eligible[index] || remaining_demand[index] == 0 {
            counts[index] = 0;
            return recurse(
                index + 1,
                stock_length_mm,
                kerf_mm,
                minimum_reusable_offcut_mm,
                remaining_demand,
                eligible,
                products,
                counts,
                output,
                budget,
            );
        }

        let product = &products[index];
        let maximum_from_stock = (stock_length_mm / product.length_mm) as u32;
        let maximum_quantity = remaining_demand[index].min(maximum_from_stock);
        for quantity in 0..=maximum_quantity {
            let added_length = product
                .length_mm
                .checked_mul(u64::from(quantity))
                .ok_or_else(|| "candidate product length overflow".to_owned())?;
            let mut product_length = 0_u64;
            let mut piece_count = 0_u32;
            for prior in 0..index {
                product_length = product_length
                    .checked_add(
                        products[prior]
                            .length_mm
                            .checked_mul(u64::from(counts[prior]))
                            .ok_or_else(|| "candidate product length overflow".to_owned())?,
                    )
                    .ok_or_else(|| "candidate product length overflow".to_owned())?;
                piece_count = piece_count
                    .checked_add(counts[prior])
                    .ok_or_else(|| "candidate piece count overflow".to_owned())?;
            }
            product_length = product_length
                .checked_add(added_length)
                .ok_or_else(|| "candidate product length overflow".to_owned())?;
            piece_count = piece_count
                .checked_add(quantity)
                .ok_or_else(|| "candidate piece count overflow".to_owned())?;
            let kerf_loss = kerf_mm
                .checked_mul(u64::from(piece_count.saturating_sub(1)))
                .ok_or_else(|| "candidate kerf overflow".to_owned())?;
            let used = product_length
                .checked_add(kerf_loss)
                .ok_or_else(|| "candidate used length overflow".to_owned())?;
            if used > stock_length_mm {
                break;
            }

            counts[index] = quantity;
            recurse(
                index + 1,
                stock_length_mm,
                kerf_mm,
                minimum_reusable_offcut_mm,
                remaining_demand,
                eligible,
                products,
                counts,
                output,
                budget,
            )?;
        }
        counts[index] = 0;
        Ok(())
    }

    let mut output = Vec::new();
    let mut counts = vec![0_u32; products.len()];
    recurse(
        0,
        stock_length_mm,
        kerf_mm,
        minimum_reusable_offcut_mm,
        remaining_demand,
        eligible,
        products,
        &mut counts,
        &mut output,
        budget,
    )?;
    Ok(output)
}

fn apply_pattern(
    partial: &PartialPlan,
    pattern: &Pattern,
    stock_index: usize,
    stock_length_mm: u64,
    products: &[ProductSpec],
) -> Check<PartialPlan> {
    let mut next = partial.clone();
    let mut pattern_counts = BTreeMap::new();
    for (index, quantity) in pattern.counts.iter().enumerate() {
        let product = &products[index];
        pattern_counts.insert(product.product_id.clone(), *quantity);
        if *quantity == 0 {
            continue;
        }
        if !product.eligible_for_demand() {
            return Err(format!(
                "internal error: ineligible product {} entered a pattern",
                product.product_id
            ));
        }
        next.product_counts[index] = next.product_counts[index]
            .checked_add(*quantity)
            .ok_or_else(|| "cumulative demand count overflow".to_owned())?;
        next.fulfilled_demand_units = next
            .fulfilled_demand_units
            .checked_add(*quantity)
            .ok_or_else(|| "fulfilled demand count overflow".to_owned())?;
        let value = product
            .unit_value_microcredits
            .checked_mul(u64::from(*quantity))
            .ok_or_else(|| "sale value multiplication overflow".to_owned())?;
        next.sale_value_microcredits = next
            .sale_value_microcredits
            .checked_add(value)
            .ok_or_else(|| "sale value sum overflow".to_owned())?;
    }
    next.product_length_mm = next
        .product_length_mm
        .checked_add(pattern.product_length_mm)
        .ok_or_else(|| "cumulative product length overflow".to_owned())?;
    next.reusable_offcut_length_mm = next
        .reusable_offcut_length_mm
        .checked_add(pattern.reusable_offcut_mm)
        .ok_or_else(|| "cumulative offcut length overflow".to_owned())?;
    next.unusable_residual_length_mm = next
        .unusable_residual_length_mm
        .checked_add(pattern.unusable_residual_mm)
        .ok_or_else(|| "cumulative unusable residual overflow".to_owned())?;
    next.kerf_loss_mm = next
        .kerf_loss_mm
        .checked_add(pattern.kerf_loss_mm)
        .ok_or_else(|| "cumulative kerf loss overflow".to_owned())?;
    next.unprocessed_stock_length_mm = next
        .unprocessed_stock_length_mm
        .checked_add(pattern.unprocessed_stock_length_mm)
        .ok_or_else(|| "cumulative unprocessed length overflow".to_owned())?;
    next.stock_patterns.push(StockPatternReceipt {
        stock_index,
        stock_length_mm,
        product_counts: pattern_counts,
        product_length_mm: pattern.product_length_mm,
        piece_count: pattern.piece_count,
        kerf_loss_mm: pattern.kerf_loss_mm,
        reusable_offcut_mm: pattern.reusable_offcut_mm,
        unusable_residual_mm: pattern.unusable_residual_mm,
        unprocessed_stock_length_mm: pattern.unprocessed_stock_length_mm,
    });
    Ok(next)
}

fn run_greedy_baseline(
    scenario: &Scenario,
    eligible: &[bool],
) -> Check<PartialPlan> {
    // Greedy comparator: at each cut, choose the highest declared synthetic
    // value-per-length product that still fits and has unmet demand. This is
    // intentionally simple, deterministic and not an assertion of industry
    // best practice.
    let mut order: Vec<usize> = (0..scenario.products.len()).collect();
    order.sort_by(|left, right| {
        let left_product = &scenario.products[*left];
        let right_product = &scenario.products[*right];
        let left_cross = u128::from(right_product.unit_value_microcredits)
            * u128::from(left_product.length_mm);
        let right_cross = u128::from(left_product.unit_value_microcredits)
            * u128::from(right_product.length_mm);
        left_cross.cmp(&right_cross).then_with(|| {
            left_product.product_id.cmp(&right_product.product_id)
        })
    });

    let mut partial = PartialPlan::empty(scenario.products.len());
    for (stock_index, stock_length_mm) in scenario.stock_lengths_mm.iter().copied().enumerate() {
        let mut counts = vec![0_u32; scenario.products.len()];
        let mut product_length_mm = 0_u64;
        let mut piece_count = 0_u32;
        loop {
            let mut selected = None;
            for index in &order {
                let product = &scenario.products[*index];
                if !eligible[*index]
                    || partial.product_counts[*index]
                        .checked_add(counts[*index])
                        .is_none_or(|count| count >= product.demand_units)
                {
                    continue;
                }
                let candidate_length = product_length_mm
                    .checked_add(product.length_mm)
                    .ok_or_else(|| "greedy candidate length overflow".to_owned())?;
                let candidate_piece_count = piece_count
                    .checked_add(1)
                    .ok_or_else(|| "greedy candidate piece count overflow".to_owned())?;
                let candidate_kerf = scenario
                    .kerf_mm
                    .checked_mul(u64::from(candidate_piece_count - 1))
                    .ok_or_else(|| "greedy candidate kerf overflow".to_owned())?;
                let candidate_used = candidate_length
                    .checked_add(candidate_kerf)
                    .ok_or_else(|| "greedy candidate used length overflow".to_owned())?;
                if candidate_used <= stock_length_mm {
                    selected = Some((*index, candidate_length, candidate_piece_count));
                    break;
                }
            }
            let Some((index, new_length, new_piece_count)) = selected else {
                break;
            };
            counts[index] = counts[index]
                .checked_add(1)
                .ok_or_else(|| "greedy SKU count overflow".to_owned())?;
            product_length_mm = new_length;
            piece_count = new_piece_count;
        }

        let pattern = make_pattern(
            stock_length_mm,
            scenario.kerf_mm,
            scenario.minimum_reusable_offcut_mm,
            &counts,
            &scenario.products,
        )?;
        partial = apply_pattern(
            &partial,
            &pattern,
            stock_index,
            stock_length_mm,
            &scenario.products,
        )?;
    }
    Ok(partial)
}

fn weakly_dominates(other: &PlanReceipt, candidate: &PlanReceipt) -> bool {
    other.sale_value_microcredits >= candidate.sale_value_microcredits
        && other.product_length_mm >= candidate.product_length_mm
        && other.reusable_offcut_length_mm >= candidate.reusable_offcut_length_mm
        && other.unrecoverable_loss_length_mm <= candidate.unrecoverable_loss_length_mm
}

fn plan_key(plan: &PartialPlan) -> (Vec<u32>, u64, u64, u64, u64, u64) {
    (
        plan.product_counts.clone(),
        plan.sale_value_microcredits,
        plan.product_length_mm,
        plan.reusable_offcut_length_mm,
        plan.kerf_loss_mm,
        plan.unusable_residual_length_mm,
    )
}

fn is_dominated(candidate: &PlanReceipt, other: &PlanReceipt) -> bool {
    let candidate_loss = candidate.unrecoverable_loss_length_mm;
    let other_loss = other.unrecoverable_loss_length_mm;
    let no_worse = other.sale_value_microcredits >= candidate.sale_value_microcredits
        && other.product_length_mm >= candidate.product_length_mm
        && other.reusable_offcut_length_mm >= candidate.reusable_offcut_length_mm
        && other_loss <= candidate_loss;
    let strictly_better = other.sale_value_microcredits > candidate.sale_value_microcredits
        || other.product_length_mm > candidate.product_length_mm
        || other.reusable_offcut_length_mm > candidate.reusable_offcut_length_mm
        || other_loss < candidate_loss;
    no_worse && strictly_better
}

fn check_conservation(
    stock_lengths_mm: &[u64],
    plan: &PlanReceipt,
) -> Check<()> {
    if plan.stock_patterns.len() != stock_lengths_mm.len() {
        return Err("plan stock-pattern count does not match stock inventory".to_owned());
    }
    let mut total_input = 0_u64;
    let mut total_accounted = 0_u64;
    for (index, pattern) in plan.stock_patterns.iter().enumerate() {
        if pattern.stock_index != index || pattern.stock_length_mm != stock_lengths_mm[index] {
            return Err("stock patterns are out of order or have changed source lengths".to_owned());
        }
        let accounted = pattern
            .product_length_mm
            .checked_add(pattern.kerf_loss_mm)
            .and_then(|sum| sum.checked_add(pattern.reusable_offcut_mm))
            .and_then(|sum| sum.checked_add(pattern.unusable_residual_mm))
            .and_then(|sum| sum.checked_add(pattern.unprocessed_stock_length_mm))
            .ok_or_else(|| "stock conservation total overflow".to_owned())?;
        if accounted != pattern.stock_length_mm {
            return Err(format!(
                "stock {index} does not close: input={} mm, accounted={accounted} mm",
                pattern.stock_length_mm
            ));
        }
        total_input = total_input
            .checked_add(pattern.stock_length_mm)
            .ok_or_else(|| "input stock sum overflow".to_owned())?;
        total_accounted = total_accounted
            .checked_add(accounted)
            .ok_or_else(|| "accounted stock sum overflow".to_owned())?;
    }
    if total_input != total_accounted {
        return Err("aggregate stock conservation failed".to_owned());
    }
    Ok(())
}

fn evaluate_scenario_with_budget(
    scenario: &Scenario,
    budget_limit: u64,
) -> Check<ScenarioReceipt> {
    validate_scenario(scenario)?;
    let eligible: Vec<bool> = scenario
        .products
        .iter()
        .map(ProductSpec::eligible_for_demand)
        .collect();
    let eligible_product_ids: Vec<String> = scenario
        .products
        .iter()
        .zip(&eligible)
        .filter(|(_, ok)| **ok)
        .map(|(product, _)| product.product_id.clone())
        .collect();
    let excluded_structural_product_ids: Vec<String> = scenario
        .products
        .iter()
        .filter(|product| !product.eligible_for_demand())
        .map(|product| product.product_id.clone())
        .collect();

    let mut budget = SearchBudget {
        nodes: 0,
        limit: budget_limit,
    };
    let greedy_partial = run_greedy_baseline(scenario, &eligible)?;
    let greedy_meets_minimum = greedy_partial.fulfilled_demand_units
        >= scenario.minimum_pieces_required;
    let greedy_failure = if greedy_meets_minimum {
        None
    } else {
        Some(format!(
            "greedy baseline fulfilled {} pieces, below required minimum {}",
            greedy_partial.fulfilled_demand_units, scenario.minimum_pieces_required
        ))
    };
    let greedy_plan = greedy_partial.clone().into_receipt(&scenario.products);
    check_conservation(&scenario.stock_lengths_mm, &greedy_plan)?;
    let greedy_baseline = GreedyBaselineReceipt {
        strategy: "highest_synthetic_value_per_length_first_fit",
        meets_minimum_pieces: greedy_meets_minimum,
        failure_reason: greedy_failure,
        plan: greedy_plan,
    };
    let mut raw_plans: Vec<PartialPlan> = Vec::new();

    fn search(
        stock_index: usize,
        scenario: &Scenario,
        eligible: &[bool],
        partial: &PartialPlan,
        raw_plans: &mut Vec<PartialPlan>,
        budget: &mut SearchBudget,
    ) -> Check<()> {
        budget.tick("plan search")?;
        if stock_index == scenario.stock_lengths_mm.len() {
            if partial.fulfilled_demand_units >= scenario.minimum_pieces_required {
                check_conservation(
                    &scenario.stock_lengths_mm,
                    &partial.clone().into_receipt(&scenario.products),
                )?;
                raw_plans.push(partial.clone());
                if raw_plans.len() > MAX_COMPLETE_PLANS {
                    return Err(format!(
                        "{}: complete-plan limit {MAX_COMPLETE_PLANS} exceeded; no partial optimum returned",
                        scenario.scenario_id
                    ));
                }
            }
            return Ok(());
        }

        let remaining_demand: Vec<u32> = scenario
            .products
            .iter()
            .enumerate()
            .map(|(index, product)| {
                if !eligible[index] {
                    0
                } else {
                    product.demand_units.saturating_sub(partial.product_counts[index])
                }
            })
            .collect();
        let stock_length = scenario.stock_lengths_mm[stock_index];
        let patterns = enumerate_patterns(
            stock_length,
            scenario.kerf_mm,
            scenario.minimum_reusable_offcut_mm,
            &remaining_demand,
            eligible,
            &scenario.products,
            budget,
        )?;
        for pattern in &patterns {
            let next = apply_pattern(
                partial,
                pattern,
                stock_index,
                stock_length,
                &scenario.products,
            )?;
            search(
                stock_index + 1,
                scenario,
                eligible,
                &next,
                raw_plans,
                budget,
            )?;
        }
        Ok(())
    }

    search(
        0,
        scenario,
        &eligible,
        &PartialPlan::empty(scenario.products.len()),
        &mut raw_plans,
        &mut budget,
    )?;

    if raw_plans.is_empty() {
        return Err(format!(
            "{}: no feasible plan satisfies minimum_pieces_required={} under the supplied stock, demand and evidence gates",
            scenario.scenario_id, scenario.minimum_pieces_required
        ));
    }

    let mut unique: BTreeMap<
        (Vec<u32>, u64, u64, u64, u64, u64),
        PartialPlan,
    > = BTreeMap::new();
    for plan in raw_plans {
        unique.entry(plan_key(&plan)).or_insert(plan);
    }
    let feasible_plan_count = unique.len();
    let plans: Vec<PlanReceipt> = unique
        .into_values()
        .map(|plan| plan.into_receipt(&scenario.products))
        .collect();

    let mut frontier: Vec<PlanReceipt> = Vec::new();
    for candidate in &plans {
        let dominated = plans.iter().any(|other| is_dominated(candidate, other));
        if !dominated {
            frontier.push(candidate.clone());
        }
    }
    frontier.sort_by(|a, b| {
        a.sale_value_microcredits
            .cmp(&b.sale_value_microcredits)
            .then(a.product_length_mm.cmp(&b.product_length_mm))
            .then(a.reusable_offcut_length_mm.cmp(&b.reusable_offcut_length_mm))
            .then(a.unrecoverable_loss_length_mm.cmp(&b.unrecoverable_loss_length_mm))
            .then_with(|| a.product_counts.iter().cmp(b.product_counts.iter()))
    });

    for plan in &frontier {
        check_conservation(&scenario.stock_lengths_mm, plan)?;
        for product in &scenario.products {
            let count = plan.product_counts.get(&product.product_id).copied().unwrap_or(0);
            if count > product.demand_units {
                return Err(format!(
                    "{}: demand cap exceeded for {}",
                    scenario.scenario_id, product.product_id
                ));
            }
            if !product.eligible_for_demand() && count != 0 {
                return Err(format!(
                    "{}: ineligible structural product {} entered frontier",
                    scenario.scenario_id, product.product_id
                ));
            }
        }
    }

    let baseline_is_covered_by_pareto_frontier = greedy_baseline.meets_minimum_pieces
        && frontier
            .iter()
            .any(|plan| weakly_dominates(plan, &greedy_baseline.plan));
    if greedy_baseline.meets_minimum_pieces && !baseline_is_covered_by_pareto_frontier {
        return Err(format!(
            "{}: internal reference error: Pareto frontier does not dominate or match feasible greedy baseline",
            scenario.scenario_id
        ));
    }

    Ok(ScenarioReceipt {
        scenario_id: scenario.scenario_id.clone(),
        status: "pass",
        eligible_product_ids,
        excluded_structural_product_ids,
        search_nodes: budget.nodes,
        feasible_plan_count,
        pareto_plan_count: frontier.len(),
        baseline_is_covered_by_pareto_frontier,
        greedy_baseline,
        pareto_plans: frontier,
    })
}

fn evaluate_corpus(bytes: &[u8], source_commit: &str) -> Check<Value> {
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }
    let corpus: Corpus = serde_json::from_slice(bytes)
        .map_err(|error| format!("invalid corpus JSON or schema: {error}"))?;
    if corpus.suite_id != "wood-cut-plan-reference"
        || corpus.suite_version != "0.1.0"
        || corpus.manifest_kind != "synthetic-cut-plan-fixtures"
    {
        return Err("unsupported suite ID, version or manifest kind".to_owned());
    }
    if corpus.claim_ceiling.trim().is_empty() {
        return Err("claim_ceiling must not be empty".to_owned());
    }
    if corpus.units.stock_length != "millimetre"
        || corpus.units.kerf != "millimetre"
        || corpus.units.value != "synthetic_microcredit"
    {
        return Err("unsupported or ambiguous corpus units".to_owned());
    }
    if corpus.scenarios.len() != REQUIRED_SCENARIOS.len() {
        return Err(format!(
            "expected exactly {} scenarios, got {}",
            REQUIRED_SCENARIOS.len(),
            corpus.scenarios.len()
        ));
    }
    let mut ids = BTreeSet::new();
    let mut receipts = Vec::new();
    for (index, scenario) in corpus.scenarios.iter().enumerate() {
        if !ids.insert(scenario.scenario_id.clone()) {
            return Err(format!("duplicate scenario ID {}", scenario.scenario_id));
        }
        if scenario.scenario_id != REQUIRED_SCENARIOS[index] {
            return Err(format!(
                "scenario {} must be {}, got {}",
                index, REQUIRED_SCENARIOS[index], scenario.scenario_id
            ));
        }
        receipts.push(evaluate_scenario_with_budget(scenario, MAX_SEARCH_NODES)?);
    }
    let hash = Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(json!({
        "validator": "wood-cut-plan",
        "validator_version": VALIDATOR_VERSION,
        "result": "pass",
        "scenario_count": receipts.len(),
        "corpus_sha256": hash,
        "source_commit": source_commit,
        "scenarios": receipts
    }))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 || args[1] != "--source-commit" {
        eprintln!("usage: wood-cut-plan <corpus.json> --source-commit <40-character-commit-sha>");
        return ExitCode::from(2);
    }
    let bytes = match fs::read(&args[0]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read corpus: {error}");
            return ExitCode::FAILURE;
        }
    };
    match evaluate_corpus(&bytes, &args[2]) {
        Ok(receipt) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt).expect("JSON receipt serializes")
            );
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("wood-cut-plan rejected: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../../../docs/tech/wood-cut-plan-corpus-v0.1.json");
    const SOURCE_COMMIT: &str = "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa";

    fn parsed_corpus() -> Corpus {
        serde_json::from_str(CORPUS).expect("frozen corpus parses")
    }

    #[test]
    fn frozen_corpus_produces_a_receipt_for_all_scenarios() {
        let receipt = evaluate_corpus(CORPUS.as_bytes(), SOURCE_COMMIT)
            .expect("frozen reference corpus evaluates");
        assert_eq!(receipt["result"], "pass");
        assert_eq!(receipt["scenario_count"], 3);
        let scenarios = receipt["scenarios"].as_array().expect("scenario list");
        assert!(scenarios.iter().all(|scenario| scenario["status"] == "pass"));
        assert_eq!(scenarios[0]["greedy_baseline"]["plan"]["sale_value_microcredits"], 1500);
        assert_eq!(scenarios[1]["greedy_baseline"]["plan"]["sale_value_microcredits"], 2060);
        assert_eq!(scenarios[2]["greedy_baseline"]["plan"]["sale_value_microcredits"], 50);
        assert!(scenarios.iter().all(|scenario| {
            scenario["greedy_baseline"]["meets_minimum_pieces"] == true
                && scenario["baseline_is_covered_by_pareto_frontier"] == true
        }));
    }

    #[test]
    fn frozen_reference_frontier_sizes_match_independent_fixture_oracle() {
        let corpus = parsed_corpus();
        let expected = [6_usize, 21_usize, 2_usize];
        for (scenario, expected_count) in corpus.scenarios.iter().zip(expected) {
            let receipt = evaluate_scenario_with_budget(scenario, MAX_SEARCH_NODES)
                .expect("scenario evaluates");
            assert_eq!(
                receipt.pareto_plan_count,
                expected_count,
                "unexpected Pareto frontier size for {}",
                scenario.scenario_id
            );
        }
    }

    #[test]
    fn ineligible_structural_demand_is_never_fulfilled() {
        let corpus = parsed_corpus();
        let scenario = &corpus.scenarios[0];
        let receipt = evaluate_scenario_with_budget(scenario, MAX_SEARCH_NODES)
            .expect("scenario evaluates");
        assert_eq!(
            receipt.excluded_structural_product_ids,
            vec!["unverified_premium".to_owned()]
        );
        for plan in receipt.pareto_plans {
            assert_eq!(
                plan.product_counts.get("unverified_premium"),
                Some(&0)
            );
        }
    }

    #[test]
    fn all_frontier_plans_conserve_every_stock_piece() {
        let corpus = parsed_corpus();
        for scenario in &corpus.scenarios {
            let receipt = evaluate_scenario_with_budget(scenario, MAX_SEARCH_NODES)
                .expect("scenario evaluates");
            for plan in &receipt.pareto_plans {
                check_conservation(&scenario.stock_lengths_mm, plan)
                    .expect("each stock item is accounted for exactly");
            }
        }
    }

    #[test]
    fn demand_caps_are_never_exceeded() {
        let corpus = parsed_corpus();
        for scenario in &corpus.scenarios {
            let receipt = evaluate_scenario_with_budget(scenario, MAX_SEARCH_NODES)
                .expect("scenario evaluates");
            for plan in &receipt.pareto_plans {
                for product in &scenario.products {
                    assert!(
                        plan.product_counts[&product.product_id] <= product.demand_units,
                        "demand cap violated for {}",
                        product.product_id
                    );
                }
            }
        }
    }

    #[test]
    fn exact_fit_dominates_two_short_pieces_that_create_kerf_and_waste() {
        let corpus = parsed_corpus();
        let receipt = evaluate_scenario_with_budget(&corpus.scenarios[2], MAX_SEARCH_NODES)
            .expect("edge scenario evaluates");
        assert!(receipt.pareto_plans.iter().any(|plan| {
            plan.product_counts.get("exact_fit") == Some(&1)
        }));
        assert!(!receipt.pareto_plans.iter().any(|plan| {
            plan.product_counts.get("half_stock") == Some(&2)
        }));
    }

    #[test]
    fn no_feasible_cut_fails_closed() {
        let scenario: Scenario = serde_json::from_value(json!({
            "scenario_id": "IMPOSSIBLE",
            "evidence_class": "synthetic",
            "stock_lengths_mm": [100],
            "kerf_mm": 4,
            "minimum_reusable_offcut_mm": 10,
            "minimum_pieces_required": 1,
            "products": [{
                "product_id": "too_long",
                "length_mm": 101,
                "demand_units": 1,
                "unit_value_microcredits": 1000,
                "application": "non_structural",
                "grade_evidence_state": "unknown",
                "inspection_pass": false
            }]
        })).expect("scenario schema parses");
        let error = evaluate_scenario_with_budget(&scenario, MAX_SEARCH_NODES)
            .expect_err("infeasible request must fail");
        assert!(error.contains("no feasible plan"));
    }

    #[test]
    fn exhausted_search_budget_never_returns_a_partial_frontier() {
        let corpus = parsed_corpus();
        let error = evaluate_scenario_with_budget(&corpus.scenarios[0], 0)
            .expect_err("zero-node budget must reject");
        assert!(error.contains("search budget exhausted"));
        assert!(error.contains("no partial optimum returned"));
    }

    #[test]
    fn receipt_is_deterministic_for_identical_bytes_and_source_revision() {
        let first = evaluate_corpus(CORPUS.as_bytes(), SOURCE_COMMIT)
            .expect("first evaluation");
        let second = evaluate_corpus(CORPUS.as_bytes(), SOURCE_COMMIT)
            .expect("second evaluation");
        assert_eq!(
            serde_json::to_vec(&first).expect("serialize first"),
            serde_json::to_vec(&second).expect("serialize second")
        );
    }

    #[test]
    fn unknown_corpus_keys_are_rejected() {
        let mut value: Value = serde_json::from_str(CORPUS).expect("valid corpus");
        value["invented_default"] = json!(0);
        let bytes = serde_json::to_vec(&value).expect("mutated corpus serializes");
        let error = evaluate_corpus(&bytes, SOURCE_COMMIT)
            .expect_err("unknown key must fail");
        assert!(error.contains("unknown field"));
    }
}
