use std::fmt;

pub const HORIZON: u8 = 10;
pub const CALLS_PER_SCENARIO: usize = 60_000;

const STRATUM_0: HazardProfile = HazardProfile {
    berth: 0.12,
    diversion: 0.03,
    cancellation: 0.02,
};
const STRATUM_1: HazardProfile = HazardProfile {
    berth: 0.07,
    diversion: 0.06,
    cancellation: 0.04,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Cause {
    Berth,
    Diversion,
    Cancellation,
}

impl Cause {
    fn index(self) -> usize {
        match self {
            Self::Berth => 0,
            Self::Diversion => 1,
            Self::Cancellation => 2,
        }
    }
}

impl fmt::Display for Cause {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Berth => write!(f, "BERTH"),
            Self::Diversion => write!(f, "DIVERSION"),
            Self::Cancellation => write!(f, "CANCELLATION"),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CensorReason {
    Administrative,
    ReportingLag,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ScenarioKind {
    IndependentAdministrative,
    MeasuredStratumDependent,
    OutcomeDependentReportingLag,
    PositivityFailure,
}

impl ScenarioKind {
    pub fn name(self) -> &'static str {
        match self {
            Self::IndependentAdministrative => "INDEPENDENT_ADMIN",
            Self::MeasuredStratumDependent => "MEASURED_STRATUM_DEPENDENT",
            Self::OutcomeDependentReportingLag => "OUTCOME_DEPENDENT_REPORTING_LAG",
            Self::PositivityFailure => "POSITIVITY_FAILURE",
        }
    }

    fn seed(self) -> u64 {
        match self {
            Self::IndependentAdministrative => 0x0001_2345_6789,
            Self::MeasuredStratumDependent => 0x23_4567_89ab,
            Self::OutcomeDependentReportingLag => 0x34_5678_9abc,
            Self::PositivityFailure => 0x45_6789_abcd,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct HazardProfile {
    berth: f64,
    diversion: f64,
    cancellation: f64,
}

impl HazardProfile {
    fn total(self) -> f64 {
        self.berth + self.diversion + self.cancellation
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct LatentCall {
    stratum: u8,
    cause: Option<Cause>,
    event_tick: Option<u8>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ObservedCall {
    pub stratum: u8,
    pub duration_tick: u8,
    pub outcome: Option<Cause>,
    pub censor_reason: Option<CensorReason>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Incidence {
    pub berth: f64,
    pub diversion: f64,
    pub cancellation: f64,
}

impl Incidence {
    fn zero() -> Self {
        Self {
            berth: 0.0,
            diversion: 0.0,
            cancellation: 0.0,
        }
    }

    fn add_cause(&mut self, cause: Cause, value: f64) {
        match cause {
            Cause::Berth => self.berth += value,
            Cause::Diversion => self.diversion += value,
            Cause::Cancellation => self.cancellation += value,
        }
    }

    pub fn max_abs_error(self, other: Self) -> f64 {
        (self.berth - other.berth)
            .abs()
            .max((self.diversion - other.diversion).abs())
            .max((self.cancellation - other.cancellation).abs())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IncidenceBounds {
    pub lower: Incidence,
    pub upper: Incidence,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Qualification {
    IdentifiableUnderDeclaredMechanism,
    SensitivityRequiredNotIdentified,
    NotEvaluable,
}

impl Qualification {
    pub fn label(self) -> &'static str {
        match self {
            Self::IdentifiableUnderDeclaredMechanism => "IDENTIFIABLE_UNDER_DECLARED_MECHANISM",
            Self::SensitivityRequiredNotIdentified => "SENSITIVITY_REQUIRED_NOT_IDENTIFIED",
            Self::NotEvaluable => "NOT_EVALUABLE",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioReport {
    pub scenario: String,
    pub calls: usize,
    pub true_empirical: Incidence,
    pub analytic_truth: Incidence,
    pub pooled_aalen_johansen: Incidence,
    pub standardized_stratum_aalen_johansen: Incidence,
    pub known_g_conditional_ipcw: Option<Incidence>,
    pub observed_event_share: Incidence,
    pub unresolved_fraction: f64,
    pub administrative_censored_calls: usize,
    pub reporting_lag_censored_calls: usize,
    pub partial_identification_bounds: IncidenceBounds,
    pub pooled_aj_error: f64,
    pub standardized_aj_error: f64,
    pub conditional_ipcw_error: Option<f64>,
    pub minimum_censoring_survival: Option<f64>,
    pub maximum_ipcw_weight: Option<f64>,
    pub stratum_1_true_berths: usize,
    pub stratum_1_observed_berths: usize,
    pub qualification: Qualification,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LagSensitivityStep {
    pub reporting_delay_ticks: u8,
    pub true_berth_cif: f64,
    pub observed_berth_cif: f64,
    pub berth_underestimation: f64,
    pub unresolved_fraction: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct RobustnessReport {
    pub scenarios: Vec<ScenarioReport>,
    pub reporting_delay_sweep: Vec<LagSensitivityStep>,
}

#[derive(Clone, Copy, Debug)]
struct SplitMix64 {
    state: u64,
}

impl SplitMix64 {
    fn new(seed: u64) -> Self {
        Self { state: seed }
    }

    fn next_u64(&mut self) -> u64 {
        self.state = self.state.wrapping_add(0x9E37_79B9_7F4A_7C15);
        let mut value = self.state;
        value = (value ^ (value >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
        value = (value ^ (value >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
        value ^ (value >> 31)
    }

    fn next_f64(&mut self) -> f64 {
        let mantissa = self.next_u64() >> 11;
        mantissa as f64 / 9_007_199_254_740_992.0
    }
}

fn profile(stratum: u8) -> HazardProfile {
    if stratum == 0 { STRATUM_0 } else { STRATUM_1 }
}

fn sample_latent_event(stratum: u8, rng: &mut SplitMix64) -> LatentCall {
    let hazard = profile(stratum);
    let total = hazard.total();

    for tick in 1..=HORIZON {
        let draw = rng.next_f64();
        if draw < total {
            let cause_draw = rng.next_f64() * total;
            let cause = if cause_draw < hazard.berth {
                Cause::Berth
            } else if cause_draw < hazard.berth + hazard.diversion {
                Cause::Diversion
            } else {
                Cause::Cancellation
            };
            return LatentCall {
                stratum,
                cause: Some(cause),
                event_tick: Some(tick),
            };
        }
    }

    LatentCall {
        stratum,
        cause: None,
        event_tick: None,
    }
}

fn sample_administrative_censor_tick(probability_per_tick: f64, rng: &mut SplitMix64) -> u8 {
    for tick in 1..=HORIZON {
        if rng.next_f64() < probability_per_tick {
            return tick;
        }
    }
    HORIZON + 1
}

fn censor(stratum: u8, duration_tick: u8, reason: CensorReason) -> ObservedCall {
    ObservedCall {
        stratum,
        duration_tick,
        outcome: None,
        censor_reason: Some(reason),
    }
}

fn event(stratum: u8, duration_tick: u8, cause: Cause) -> ObservedCall {
    ObservedCall {
        stratum,
        duration_tick,
        outcome: Some(cause),
        censor_reason: None,
    }
}

fn observe_call(scenario: ScenarioKind, latent: LatentCall, rng: &mut SplitMix64) -> ObservedCall {
    match scenario {
        ScenarioKind::IndependentAdministrative | ScenarioKind::MeasuredStratumDependent => {
            let censor_probability = match scenario {
                ScenarioKind::IndependentAdministrative => 0.10,
                ScenarioKind::MeasuredStratumDependent if latent.stratum == 0 => 0.03,
                ScenarioKind::MeasuredStratumDependent => 0.22,
                _ => unreachable!(),
            };
            let censor_tick = sample_administrative_censor_tick(censor_probability, rng);

            // Event wins an exact event/censor tick tie. This tie convention
            // is part of the frozen discrete-time benchmark protocol.
            if let (Some(cause), Some(event_tick)) = (latent.cause, latent.event_tick) {
                if event_tick <= censor_tick {
                    return event(latent.stratum, event_tick, cause);
                }
            }

            censor(
                latent.stratum,
                censor_tick.min(HORIZON),
                CensorReason::Administrative,
            )
        }
        ScenarioKind::OutcomeDependentReportingLag => match (latent.cause, latent.event_tick) {
            (Some(cause), Some(event_tick)) => {
                let delay = if cause == Cause::Berth { 4 } else { 0 };
                if event_tick.saturating_add(delay) <= HORIZON {
                    event(latent.stratum, event_tick, cause)
                } else {
                    censor(latent.stratum, HORIZON, CensorReason::ReportingLag)
                }
            }
            _ => censor(latent.stratum, HORIZON, CensorReason::Administrative),
        },
        ScenarioKind::PositivityFailure => match (latent.cause, latent.event_tick) {
            (Some(cause), Some(event_tick)) => {
                let delay = if latent.stratum == 1 && cause == Cause::Berth {
                    HORIZON + 1
                } else {
                    0
                };
                if event_tick.saturating_add(delay) <= HORIZON {
                    event(latent.stratum, event_tick, cause)
                } else {
                    censor(latent.stratum, HORIZON, CensorReason::ReportingLag)
                }
            }
            _ => censor(latent.stratum, HORIZON, CensorReason::Administrative),
        },
    }
}

fn analytic_stratum_cif(stratum: u8) -> Incidence {
    let hazard = profile(stratum);
    let total = hazard.total();
    let survival_after_horizon = (1.0 - total).powi(i32::from(HORIZON));
    let cumulative_event_mass = 1.0 - survival_after_horizon;

    Incidence {
        berth: hazard.berth / total * cumulative_event_mass,
        diversion: hazard.diversion / total * cumulative_event_mass,
        cancellation: hazard.cancellation / total * cumulative_event_mass,
    }
}

fn analytic_population_cif() -> Incidence {
    let first = analytic_stratum_cif(0);
    let second = analytic_stratum_cif(1);
    Incidence {
        berth: (first.berth + second.berth) / 2.0,
        diversion: (first.diversion + second.diversion) / 2.0,
        cancellation: (first.cancellation + second.cancellation) / 2.0,
    }
}

fn truth_cif(latent: &[LatentCall]) -> Incidence {
    let mut result = Incidence::zero();
    let total = latent.len() as f64;
    for call in latent {
        if let Some(cause) = call.cause {
            result.add_cause(cause, 1.0 / total);
        }
    }
    result
}

fn aj_cif(observed: &[ObservedCall]) -> Incidence {
    let mut survival = 1.0_f64;
    let mut cif = Incidence::zero();
    let mut at_risk = observed.len();

    for tick in 1..=HORIZON {
        let mut events = [0_usize; 3];
        let mut censored = 0_usize;

        for call in observed.iter().filter(|call| call.duration_tick == tick) {
            if let Some(cause) = call.outcome {
                events[cause.index()] += 1;
            } else {
                censored += 1;
            }
        }

        let total_events = events.iter().sum::<usize>();
        if total_events + censored > at_risk {
            // This cannot occur for a structurally valid cohort.
            return Incidence {
                berth: f64::NAN,
                diversion: f64::NAN,
                cancellation: f64::NAN,
            };
        }

        if at_risk > 0 {
            let survival_before = survival;
            survival *= (at_risk - total_events) as f64 / at_risk as f64;
            for cause in [Cause::Berth, Cause::Diversion, Cause::Cancellation] {
                cif.add_cause(
                    cause,
                    survival_before * events[cause.index()] as f64 / at_risk as f64,
                );
            }
        }
        at_risk -= total_events + censored;
    }

    cif
}

fn selected(observed: &[ObservedCall], stratum: u8) -> Vec<ObservedCall> {
    observed
        .iter()
        .copied()
        .filter(|call| call.stratum == stratum)
        .collect()
}

fn standardize_stratum_aj(observed: &[ObservedCall]) -> Incidence {
    let first = aj_cif(&selected(observed, 0));
    let second = aj_cif(&selected(observed, 1));
    Incidence {
        berth: (first.berth + second.berth) / 2.0,
        diversion: (first.diversion + second.diversion) / 2.0,
        cancellation: (first.cancellation + second.cancellation) / 2.0,
    }
}

fn known_g_ipcw(
    observed: &[ObservedCall],
    scenario: ScenarioKind,
) -> Option<(Incidence, f64, f64)> {
    if !matches!(
        scenario,
        ScenarioKind::IndependentAdministrative | ScenarioKind::MeasuredStratumDependent
    ) {
        return None;
    }

    let mut incidence = Incidence::zero();
    let mut minimum_g = 1.0_f64;
    let mut maximum_weight = 1.0_f64;
    let denominator = observed.len() as f64;

    for call in observed {
        let Some(cause) = call.outcome else {
            continue;
        };
        let q: f64 = match scenario {
            ScenarioKind::IndependentAdministrative => 0.10,
            ScenarioKind::MeasuredStratumDependent if call.stratum == 0 => 0.03,
            ScenarioKind::MeasuredStratumDependent => 0.22,
            _ => unreachable!(),
        };

        // With event-priority at tied ticks, G(t|X)=P(C >= t|X).
        let g = (1.0 - q).powi(i32::from(call.duration_tick) - 1);
        if g <= 0.0 {
            return None;
        }
        minimum_g = minimum_g.min(g);
        maximum_weight = maximum_weight.max(1.0 / g);
        incidence.add_cause(cause, 1.0 / (g * denominator));
    }

    // Report the weakest theoretical observation support through the horizon,
    // not merely the minimum G among events that happened to be observed.
    let minimum_horizon_g = match scenario {
        ScenarioKind::IndependentAdministrative => (1.0_f64 - 0.10).powi(i32::from(HORIZON) - 1),
        ScenarioKind::MeasuredStratumDependent => (1.0_f64 - 0.22).powi(i32::from(HORIZON) - 1),
        _ => unreachable!(),
    };

    Some((incidence, minimum_horizon_g.min(minimum_g), maximum_weight))
}

fn observed_cause_share(observed: &[ObservedCall]) -> Incidence {
    let mut result = Incidence::zero();
    let denominator = observed.len() as f64;
    for call in observed {
        if let Some(cause) = call.outcome {
            result.add_cause(cause, 1.0 / denominator);
        }
    }
    result
}

fn unresolved_fraction(observed: &[ObservedCall]) -> f64 {
    observed
        .iter()
        .filter(|call| call.outcome.is_none())
        .count() as f64
        / observed.len() as f64
}

fn partial_bounds(observed: &[ObservedCall]) -> IncidenceBounds {
    let lower = observed_cause_share(observed);
    let unresolved = unresolved_fraction(observed);
    IncidenceBounds {
        lower,
        upper: Incidence {
            berth: (lower.berth + unresolved).min(1.0),
            diversion: (lower.diversion + unresolved).min(1.0),
            cancellation: (lower.cancellation + unresolved).min(1.0),
        },
    }
}

fn generate_scenario(scenario: ScenarioKind, count: usize) -> (Vec<LatentCall>, Vec<ObservedCall>) {
    // Separate deterministic streams prevent event-dependent PRNG draw
    // counts from coupling the latent event and censoring processes.
    let mut latent_rng = SplitMix64::new(scenario.seed() ^ 0xA076_1D64_78BD_642F);
    let mut censor_rng = SplitMix64::new(scenario.seed() ^ 0xE703_7ED1_A0B4_28DB);
    let mut latent = Vec::with_capacity(count);
    let mut observed = Vec::with_capacity(count);

    for index in 0..count {
        let stratum = (index % 2) as u8;
        let truth = sample_latent_event(stratum, &mut latent_rng);
        let record = observe_call(scenario, truth, &mut censor_rng);
        latent.push(truth);
        observed.push(record);
    }

    (latent, observed)
}

fn observe_with_berth_reporting_delay(latent: LatentCall, berth_delay_ticks: u8) -> ObservedCall {
    match (latent.cause, latent.event_tick) {
        (Some(Cause::Berth), Some(event_tick))
            if event_tick.saturating_add(berth_delay_ticks) > HORIZON =>
        {
            censor(latent.stratum, HORIZON, CensorReason::ReportingLag)
        }
        (Some(cause), Some(event_tick)) => event(latent.stratum, event_tick, cause),
        _ => censor(latent.stratum, HORIZON, CensorReason::Administrative),
    }
}

fn evaluate_reporting_delay_sweep() -> Vec<LagSensitivityStep> {
    // Hold the latent cohort fixed across all delay values. Only publication
    // latency changes, so the sensitivity curve has no cohort-composition noise.
    let scenario = ScenarioKind::OutcomeDependentReportingLag;
    let mut latent_rng = SplitMix64::new(scenario.seed() ^ 0xA076_1D64_78BD_642F);
    let latent: Vec<LatentCall> = (0..CALLS_PER_SCENARIO)
        .map(|index| sample_latent_event((index % 2) as u8, &mut latent_rng))
        .collect();
    let truth = truth_cif(&latent);
    let delays = [0_u8, 1, 2, 3, 4, 5, 6, 8, 10];

    delays
        .into_iter()
        .map(|delay| {
            let observed: Vec<ObservedCall> = latent
                .iter()
                .copied()
                .map(|call| observe_with_berth_reporting_delay(call, delay))
                .collect();
            let estimate = aj_cif(&observed);
            LagSensitivityStep {
                reporting_delay_ticks: delay,
                true_berth_cif: truth.berth,
                observed_berth_cif: estimate.berth,
                berth_underestimation: truth.berth - estimate.berth,
                unresolved_fraction: unresolved_fraction(&observed),
            }
        })
        .collect()
}

fn evaluate_scenario(scenario: ScenarioKind) -> ScenarioReport {
    let (latent, observed) = generate_scenario(scenario, CALLS_PER_SCENARIO);
    let full_truth = truth_cif(&latent);
    let analytic = analytic_population_cif();
    let pooled = aj_cif(&observed);
    let standardized = standardize_stratum_aj(&observed);
    let ipcw = known_g_ipcw(&observed, scenario);
    let bounds = partial_bounds(&observed);
    let administrative_censored_calls = observed
        .iter()
        .filter(|call| call.censor_reason == Some(CensorReason::Administrative))
        .count();
    let reporting_lag_censored_calls = observed
        .iter()
        .filter(|call| call.censor_reason == Some(CensorReason::ReportingLag))
        .count();

    let stratum_1_true_berths = latent
        .iter()
        .filter(|call| call.stratum == 1 && call.cause == Some(Cause::Berth))
        .count();
    let stratum_1_observed_berths = observed
        .iter()
        .filter(|call| call.stratum == 1 && call.outcome == Some(Cause::Berth))
        .count();

    let qualification = match scenario {
        ScenarioKind::IndependentAdministrative | ScenarioKind::MeasuredStratumDependent => {
            Qualification::IdentifiableUnderDeclaredMechanism
        }
        ScenarioKind::OutcomeDependentReportingLag => {
            Qualification::SensitivityRequiredNotIdentified
        }
        ScenarioKind::PositivityFailure => Qualification::NotEvaluable,
    };

    ScenarioReport {
        scenario: scenario.name().to_string(),
        calls: observed.len(),
        true_empirical: full_truth,
        analytic_truth: analytic,
        pooled_aalen_johansen: pooled,
        standardized_stratum_aalen_johansen: standardized,
        known_g_conditional_ipcw: ipcw.map(|item| item.0),
        observed_event_share: observed_cause_share(&observed),
        unresolved_fraction: unresolved_fraction(&observed),
        administrative_censored_calls,
        reporting_lag_censored_calls,
        partial_identification_bounds: bounds,
        pooled_aj_error: pooled.max_abs_error(full_truth),
        standardized_aj_error: standardized.max_abs_error(full_truth),
        conditional_ipcw_error: ipcw.map(|item| item.0.max_abs_error(full_truth)),
        minimum_censoring_survival: ipcw.map(|item| item.1),
        maximum_ipcw_weight: ipcw.map(|item| item.2),
        stratum_1_true_berths,
        stratum_1_observed_berths,
        qualification,
    }
}

pub fn evaluate_reference_scenarios() -> RobustnessReport {
    RobustnessReport {
        scenarios: [
            ScenarioKind::IndependentAdministrative,
            ScenarioKind::MeasuredStratumDependent,
            ScenarioKind::OutcomeDependentReportingLag,
            ScenarioKind::PositivityFailure,
        ]
        .into_iter()
        .map(evaluate_scenario)
        .collect(),
        reporting_delay_sweep: evaluate_reporting_delay_sweep(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn report_for<'a>(report: &'a RobustnessReport, name: &str) -> &'a ScenarioReport {
        report
            .scenarios
            .iter()
            .find(|scenario| scenario.scenario == name)
            .expect("scenario exists")
    }

    fn close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:.8}, expected {expected:.8}, tolerance {tolerance:.8}"
        );
    }

    fn contains(bounds: IncidenceBounds, truth: Incidence) -> bool {
        truth.berth >= bounds.lower.berth
            && truth.berth <= bounds.upper.berth
            && truth.diversion >= bounds.lower.diversion
            && truth.diversion <= bounds.upper.diversion
            && truth.cancellation >= bounds.lower.cancellation
            && truth.cancellation <= bounds.upper.cancellation
    }

    #[test]
    fn generator_analytic_truth_is_pinned() {
        let truth = analytic_population_cif();
        close(truth.berth, 0.4721162404244379, 1e-12);
        close(truth.diversion, 0.22363400862210214, 1e-12);
        close(truth.cancellation, 0.14908933908140143, 1e-12);
    }

    #[test]
    fn independent_administrative_censoring_preserves_pooled_aj() {
        let report = evaluate_reference_scenarios();
        let independent = report_for(&report, "INDEPENDENT_ADMIN");
        assert!(
            independent.pooled_aj_error < 0.015,
            "pooled A-J error {}",
            independent.pooled_aj_error
        );
        assert!(
            independent.conditional_ipcw_error.unwrap() < 0.015,
            "IPCW error {:?}",
            independent.conditional_ipcw_error
        );
        assert_eq!(
            independent.qualification,
            Qualification::IdentifiableUnderDeclaredMechanism
        );
    }

    #[test]
    fn measured_stratum_dependent_censoring_breaks_pooled_aj_but_is_adjustable() {
        let report = evaluate_reference_scenarios();
        let conditional = report_for(&report, "MEASURED_STRATUM_DEPENDENT");
        assert!(
            conditional.pooled_aalen_johansen.berth - conditional.true_empirical.berth > 0.02,
            "pooled A-J did not expose the pre-registered stratum-mix bias"
        );
        assert!(
            conditional.standardized_aj_error < 0.015,
            "stratified A-J error {}",
            conditional.standardized_aj_error
        );
        assert!(
            conditional.conditional_ipcw_error.unwrap() < 0.015,
            "conditional IPCW error {:?}",
            conditional.conditional_ipcw_error
        );
        assert!(
            conditional.maximum_ipcw_weight.unwrap() > 1.0
                && conditional.minimum_censoring_survival.unwrap() > 0.0
        );
    }

    #[test]
    fn outcome_dependent_reporting_delay_is_not_given_a_calibrated_label() {
        let report = evaluate_reference_scenarios();
        let dependent = report_for(&report, "OUTCOME_DEPENDENT_REPORTING_LAG");
        assert!(
            dependent.true_empirical.berth - dependent.pooled_aalen_johansen.berth > 0.05,
            "outcome-dependent lag did not create the expected naive A-J bias"
        );
        assert_eq!(
            dependent.qualification,
            Qualification::SensitivityRequiredNotIdentified
        );
        assert!(contains(
            dependent.partial_identification_bounds,
            dependent.true_empirical
        ));
        assert!(dependent.known_g_conditional_ipcw.is_none());
    }

    #[test]
    fn positivity_failure_is_not_extrapolated_with_infinite_weights() {
        let report = evaluate_reference_scenarios();
        let positivity = report_for(&report, "POSITIVITY_FAILURE");
        assert_eq!(positivity.stratum_1_observed_berths, 0);
        assert!(positivity.stratum_1_true_berths > 0);
        assert_eq!(positivity.qualification, Qualification::NotEvaluable);
        assert!(positivity.known_g_conditional_ipcw.is_none());
        assert!(contains(
            positivity.partial_identification_bounds,
            positivity.true_empirical
        ));
    }

    #[test]
    fn observed_only_bounds_include_each_full_truth_incidence() {
        let report = evaluate_reference_scenarios();
        for scenario in &report.scenarios {
            assert!(contains(
                scenario.partial_identification_bounds,
                scenario.true_empirical
            ));
        }
    }

    #[test]
    fn administrative_and_reporting_lag_censors_remain_distinct() {
        let report = evaluate_reference_scenarios();
        let independent = report_for(&report, "INDEPENDENT_ADMIN");
        let measured = report_for(&report, "MEASURED_STRATUM_DEPENDENT");
        let delayed = report_for(&report, "OUTCOME_DEPENDENT_REPORTING_LAG");
        let positivity = report_for(&report, "POSITIVITY_FAILURE");

        assert_eq!(independent.administrative_censored_calls, 22_094);
        assert_eq!(independent.reporting_lag_censored_calls, 0);
        assert_eq!(measured.administrative_censored_calls, 22_600);
        assert_eq!(measured.reporting_lag_censored_calls, 0);
        assert_eq!(delayed.administrative_censored_calls, 9_267);
        assert_eq!(delayed.reporting_lag_censored_calls, 5_824);
        assert_eq!(positivity.administrative_censored_calls, 9_376);
        assert_eq!(positivity.reporting_lag_censored_calls, 10_501);
    }

    #[test]
    fn event_wins_same_tick_censor_tie() {
        let observed = [
            ObservedCall {
                stratum: 0,
                duration_tick: 2,
                outcome: Some(Cause::Berth),
                censor_reason: None,
            },
            ObservedCall {
                stratum: 1,
                duration_tick: 2,
                outcome: None,
                censor_reason: Some(CensorReason::Administrative),
            },
        ];
        let estimate = aj_cif(&observed);
        close(estimate.berth, 0.5, 1e-12);
    }

    #[test]
    fn reporting_delay_sweep_uses_one_latent_cohort_and_exposes_growing_bias() {
        let report = evaluate_reference_scenarios();
        assert_eq!(report.reporting_delay_sweep.len(), 9);

        let truth = report.reporting_delay_sweep[0].true_berth_cif;
        close(
            report.reporting_delay_sweep[0].observed_berth_cif,
            truth,
            1e-12,
        );
        close(
            report.reporting_delay_sweep[0].berth_underestimation,
            0.0,
            1e-12,
        );

        for window in report.reporting_delay_sweep.windows(2) {
            assert_eq!(window[0].true_berth_cif, window[1].true_berth_cif);
            assert!(
                window[1].berth_underestimation >= window[0].berth_underestimation,
                "berth underestimation did not grow with reporting delay"
            );
            assert!(
                window[1].unresolved_fraction >= window[0].unresolved_fraction,
                "unresolved fraction did not grow with reporting delay"
            );
        }

        let four_tick = report
            .reporting_delay_sweep
            .iter()
            .find(|step| step.reporting_delay_ticks == 4)
            .expect("four-tick delay exists");
        close(four_tick.berth_underestimation, 0.09706666666666669, 1e-12);

        let ten_tick = report
            .reporting_delay_sweep
            .iter()
            .find(|step| step.reporting_delay_ticks == 10)
            .expect("ten-tick delay exists");
        close(ten_tick.observed_berth_cif, 0.0, 1e-12);
    }

    #[test]
    fn seeded_generator_is_replay_deterministic() {
        let first = evaluate_reference_scenarios();
        let second = evaluate_reference_scenarios();
        assert_eq!(first, second);
    }
}
