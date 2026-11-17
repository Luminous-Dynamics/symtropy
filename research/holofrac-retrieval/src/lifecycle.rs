//! Synthetic, evidence-carrying component lifecycle ledger for HoloFrac.
//!
//! This fixture tests append-only identity, lifecycle transitions, bitemporal
//! availability and material mass accounting. It is not a real EPD/LCA model
//! and does not certify recoverability or reuse permission.

pub const LIFECYCLE_SCHEMA_VERSION: u32 = 1;
pub const LIFECYCLE_FIXTURE_GENERATOR: &str = "holofrac-component-lifecycle-v0";
const YEAR_MICROS: i64 = 31_557_600_000_000;
const DAY_MICROS: i64 = 86_400_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleState {
    Designed,
    InService,
    Removed,
    Recovered,
    Recycled,
    Disposed,
}

impl LifecycleState {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Designed => "designed",
            Self::InService => "in_service",
            Self::Removed => "removed",
            Self::Recovered => "recovered",
            Self::Recycled => "recycled",
            Self::Disposed => "disposed",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentCondition {
    Good,
    Degraded,
    Damaged,
    Repaired,
}

impl ComponentCondition {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Good => "good",
            Self::Degraded => "degraded",
            Self::Damaged => "damaged",
            Self::Repaired => "repaired",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum LifecycleAction {
    Install,
    Inspect,
    Repair,
    Remove,
    Recover,
    Reuse,
    Recycle,
    Dispose,
}

impl LifecycleAction {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Install => "install",
            Self::Inspect => "inspect",
            Self::Repair => "repair",
            Self::Remove => "remove",
            Self::Recover => "recover",
            Self::Reuse => "reuse",
            Self::Recycle => "recycle",
            Self::Dispose => "dispose",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentPassport {
    pub component_id: String,
    pub material_id: String,
    pub passport_id: String,
    /// Synthetic source identifier in this fixture; not a cryptographic digest.
    pub source_geometry_id: String,
    pub connection_type: String,
    pub initial_mass_kg: f64,
    pub initial_use_class: String,
    pub initial_condition: ComponentCondition,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentLifecycleEvent {
    pub event_id: u64,
    pub component_id: String,
    pub revision: u32,
    pub prior_event_id: Option<u64>,
    pub action: LifecycleAction,
    pub state_before: LifecycleState,
    pub state_after: LifecycleState,
    pub condition_before: ComponentCondition,
    pub condition_after: ComponentCondition,
    pub use_class_after: String,
    pub effective_at_micros: i64,
    pub recorded_at_micros: i64,
    pub mass_before_kg: f64,
    pub mass_after_kg: f64,
    pub mass_loss_kg: f64,
    pub recycled_mass_kg: f64,
    pub disposed_mass_kg: f64,
    /// Synthetic supporting record ID; never interpreted as a real certification receipt.
    pub supporting_record_id: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ComponentLifecycleLedger {
    pub passport: ComponentPassport,
    /// Append-only sequence ordered by source revision.
    pub events: Vec<ComponentLifecycleEvent>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LifecycleSnapshot {
    pub decision_cutoff_micros: i64,
    pub revision: u32,
    pub last_event_id: Option<u64>,
    pub state: LifecycleState,
    pub condition: ComponentCondition,
    pub use_class: String,
    pub mass_kg: f64,
    pub visible_event_count: usize,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct LifecycleAccounting {
    pub initial_mass_kg: f64,
    pub active_mass_kg: f64,
    pub recycled_mass_kg: f64,
    pub disposed_mass_kg: f64,
    pub loss_mass_kg: f64,
    pub residual_kg: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LifecycleFixtureConfig {
    pub seed: u64,
    pub initial_mass_kg: f64,
    pub first_recovery_loss_fraction: f64,
    pub second_recovery_loss_fraction: f64,
    pub late_final_recording_delay_years: i64,
}

impl Default for LifecycleFixtureConfig {
    fn default() -> Self {
        Self {
            seed: 0x484f_4c4f_4c49_4645,
            initial_mass_kg: 100.0,
            first_recovery_loss_fraction: 0.05,
            second_recovery_loss_fraction: 0.05,
            late_final_recording_delay_years: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct LifecycleFixtureManifest {
    pub schema_version: u32,
    pub generator: &'static str,
    pub config: LifecycleFixtureConfig,
    /// Stable regression identifier only; not a cryptographic digest.
    pub digest_non_security: u64,
    pub component_count: usize,
    pub event_count: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct LifecycleFixture {
    pub manifest: LifecycleFixtureManifest,
    pub ledger: ComponentLifecycleLedger,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LifecycleError {
    InvalidPassport,
    InvalidMass,
    InvalidLossFraction,
    InvalidRecordingDelay,
    DuplicateEventId(u64),
    ComponentIdentityMismatch(u64),
    RevisionDiscontinuity(u64),
    ParentEventMismatch(u64),
    InvalidEventTime(u64),
    InvalidEventRecord(u64),
    InvalidTransition(u64),
    MassDiscontinuity(u64),
    MassImbalance(u64),
    ManifestMismatch,
}

impl std::fmt::Display for LifecycleError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidPassport => f.write_str("component passport contains an empty required field"),
            Self::InvalidMass => f.write_str("component or event mass must be finite and non-negative"),
            Self::InvalidLossFraction => f.write_str("recovery loss fractions must lie in [0, 1)"),
            Self::InvalidRecordingDelay => f.write_str("late recording delay must be non-negative"),
            Self::DuplicateEventId(id) => write!(f, "duplicate lifecycle event ID: {id}"),
            Self::ComponentIdentityMismatch(id) => {
                write!(f, "event {id} references a different component identity")
            }
            Self::RevisionDiscontinuity(id) => {
                write!(f, "event {id} has a discontinuous source revision")
            }
            Self::ParentEventMismatch(id) => {
                write!(f, "event {id} has an invalid prior-event reference")
            }
            Self::InvalidEventTime(id) => {
                write!(f, "event {id} has invalid effective/recorded chronology")
            }
            Self::InvalidEventRecord(id) => {
                write!(f, "event {id} has an empty supporting-record identifier")
            }
            Self::InvalidTransition(id) => {
                write!(f, "event {id} attempts an unsupported lifecycle transition")
            }
            Self::MassDiscontinuity(id) => {
                write!(f, "event {id} mass_before does not equal the prior mass_after")
            }
            Self::MassImbalance(id) => {
                write!(f, "event {id} does not conserve component mass across active/loss/recovery flows")
            }
            Self::ManifestMismatch => {
                f.write_str("lifecycle fixture manifest does not match its declared contents")
            }
        }
    }
}

impl std::error::Error for LifecycleError {}

fn mass_close(left: f64, right: f64) -> bool {
    let tolerance = 1.0e-9 * left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= tolerance
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn expected_transition(event: &ComponentLifecycleEvent) -> bool {
    let stable_mass = mass_close(event.mass_before_kg, event.mass_after_kg)
        && mass_close(event.mass_loss_kg, 0.0)
        && mass_close(event.recycled_mass_kg, 0.0)
        && mass_close(event.disposed_mass_kg, 0.0);
    let stable_condition = event.condition_before == event.condition_after;

    match event.action {
        LifecycleAction::Install => {
            event.state_before == LifecycleState::Designed
                && event.state_after == LifecycleState::InService
                && stable_condition
                && stable_mass
        }
        LifecycleAction::Inspect => {
            event.state_before == LifecycleState::InService
                && event.state_after == LifecycleState::InService
                && matches!(
                    (event.condition_before, event.condition_after),
                    (ComponentCondition::Good, ComponentCondition::Good)
                        | (ComponentCondition::Good, ComponentCondition::Degraded)
                        | (ComponentCondition::Good, ComponentCondition::Damaged)
                        | (ComponentCondition::Repaired, ComponentCondition::Repaired)
                        | (ComponentCondition::Repaired, ComponentCondition::Degraded)
                        | (ComponentCondition::Repaired, ComponentCondition::Damaged)
                        | (ComponentCondition::Degraded, ComponentCondition::Degraded)
                        | (ComponentCondition::Damaged, ComponentCondition::Damaged)
                )
                && stable_mass
        }
        LifecycleAction::Repair => {
            event.state_before == LifecycleState::InService
                && event.state_after == LifecycleState::InService
                && matches!(
                    event.condition_before,
                    ComponentCondition::Degraded | ComponentCondition::Damaged
                )
                && event.condition_after == ComponentCondition::Repaired
                && stable_mass
        }
        LifecycleAction::Remove => {
            event.state_before == LifecycleState::InService
                && event.state_after == LifecycleState::Removed
                && stable_condition
                && stable_mass
        }
        LifecycleAction::Recover => {
            event.state_before == LifecycleState::Removed
                && event.state_after == LifecycleState::Recovered
                && stable_condition
                && event.mass_after_kg > 0.0
                && mass_close(
                    event.mass_before_kg,
                    event.mass_after_kg + event.mass_loss_kg,
                )
                && mass_close(event.recycled_mass_kg, 0.0)
                && mass_close(event.disposed_mass_kg, 0.0)
        }
        LifecycleAction::Reuse => {
            event.state_before == LifecycleState::Recovered
                && event.state_after == LifecycleState::InService
                && event.condition_before != ComponentCondition::Damaged
                && stable_condition
                && stable_mass
        }
        LifecycleAction::Recycle => {
            event.state_before == LifecycleState::Recovered
                && event.state_after == LifecycleState::Recycled
                && event.mass_after_kg == 0.0
                && mass_close(event.mass_loss_kg, 0.0)
                && mass_close(event.recycled_mass_kg, event.mass_before_kg)
                && mass_close(event.disposed_mass_kg, 0.0)
        }
        LifecycleAction::Dispose => {
            event.state_before == LifecycleState::Recovered
                && event.state_after == LifecycleState::Disposed
                && event.mass_after_kg == 0.0
                && mass_close(event.mass_loss_kg, 0.0)
                && mass_close(event.recycled_mass_kg, 0.0)
                && mass_close(event.disposed_mass_kg, event.mass_before_kg)
        }
    }
}

impl ComponentLifecycleLedger {
    pub fn validate(&self) -> Result<(), LifecycleError> {
        let passport = &self.passport;
        if [
            passport.component_id.as_str(),
            passport.material_id.as_str(),
            passport.passport_id.as_str(),
            passport.source_geometry_id.as_str(),
            passport.connection_type.as_str(),
            passport.initial_use_class.as_str(),
        ]
        .iter()
        .any(|value| value.trim().is_empty())
        {
            return Err(LifecycleError::InvalidPassport);
        }
        if !finite_nonnegative(passport.initial_mass_kg) || passport.initial_mass_kg == 0.0 {
            return Err(LifecycleError::InvalidMass);
        }

        let mut event_ids = std::collections::HashSet::with_capacity(self.events.len());
        let mut expected_revision = 1_u32;
        let mut prior_event_id = None;
        let mut state = LifecycleState::Designed;
        let mut condition = passport.initial_condition;
        let mut use_class = passport.initial_use_class.clone();
        let mut current_mass = passport.initial_mass_kg;
        let mut previous_effective = 0_i64;
        let mut previous_recorded = 0_i64;

        for event in &self.events {
            if !event_ids.insert(event.event_id) {
                return Err(LifecycleError::DuplicateEventId(event.event_id));
            }
            if event.component_id != passport.component_id {
                return Err(LifecycleError::ComponentIdentityMismatch(event.event_id));
            }
            if event.revision != expected_revision {
                return Err(LifecycleError::RevisionDiscontinuity(event.event_id));
            }
            if event.prior_event_id != prior_event_id {
                return Err(LifecycleError::ParentEventMismatch(event.event_id));
            }
            if event.recorded_at_micros < event.effective_at_micros
                || event.effective_at_micros < previous_effective
                || event.recorded_at_micros < previous_recorded
            {
                return Err(LifecycleError::InvalidEventTime(event.event_id));
            }
            if event.supporting_record_id.trim().is_empty() {
                return Err(LifecycleError::InvalidEventRecord(event.event_id));
            }

            for mass in [
                event.mass_before_kg,
                event.mass_after_kg,
                event.mass_loss_kg,
                event.recycled_mass_kg,
                event.disposed_mass_kg,
            ] {
                if !finite_nonnegative(mass) {
                    return Err(LifecycleError::InvalidMass);
                }
            }
            if event.use_class_after.trim().is_empty() {
                return Err(LifecycleError::InvalidEventRecord(event.event_id));
            }
            if event.state_before != state
                || event.condition_before != condition
                || !mass_close(event.mass_before_kg, current_mass)
            {
                return Err(LifecycleError::MassDiscontinuity(event.event_id));
            }
            if !mass_close(
                event.mass_before_kg,
                event.mass_after_kg
                    + event.mass_loss_kg
                    + event.recycled_mass_kg
                    + event.disposed_mass_kg,
            ) {
                return Err(LifecycleError::MassImbalance(event.event_id));
            }
            if !expected_transition(event) {
                return Err(LifecycleError::InvalidTransition(event.event_id));
            }

            if event.action != LifecycleAction::Reuse && event.use_class_after != use_class {
                return Err(LifecycleError::InvalidTransition(event.event_id));
            }

            state = event.state_after;
            condition = event.condition_after;
            use_class = event.use_class_after.clone();
            current_mass = event.mass_after_kg;
            prior_event_id = Some(event.event_id);
            expected_revision += 1;
            previous_effective = event.effective_at_micros;
            previous_recorded = event.recorded_at_micros;
        }

        Ok(())
    }

    pub fn snapshot_as_of(
        &self,
        decision_cutoff_micros: i64,
    ) -> Result<LifecycleSnapshot, LifecycleError> {
        self.validate()?;

        let mut snapshot = LifecycleSnapshot {
            decision_cutoff_micros,
            revision: 0,
            last_event_id: None,
            state: LifecycleState::Designed,
            condition: self.passport.initial_condition,
            use_class: self.passport.initial_use_class.clone(),
            mass_kg: self.passport.initial_mass_kg,
            visible_event_count: 0,
        };

        for event in self
            .events
            .iter()
            .take_while(|event| event.recorded_at_micros <= decision_cutoff_micros)
        {
            snapshot.revision = event.revision;
            snapshot.last_event_id = Some(event.event_id);
            snapshot.state = event.state_after;
            snapshot.condition = event.condition_after;
            snapshot.use_class = event.use_class_after.clone();
            snapshot.mass_kg = event.mass_after_kg;
            snapshot.visible_event_count += 1;
        }

        Ok(snapshot)
    }

    pub fn accounting_as_of(
        &self,
        decision_cutoff_micros: i64,
    ) -> Result<LifecycleAccounting, LifecycleError> {
        let snapshot = self.snapshot_as_of(decision_cutoff_micros)?;
        let mut recycled_mass_kg = 0.0;
        let mut disposed_mass_kg = 0.0;
        let mut loss_mass_kg = 0.0;

        for event in self
            .events
            .iter()
            .take(snapshot.visible_event_count)
        {
            recycled_mass_kg += event.recycled_mass_kg;
            disposed_mass_kg += event.disposed_mass_kg;
            loss_mass_kg += event.mass_loss_kg;
        }

        let residual_kg = self.passport.initial_mass_kg
            - snapshot.mass_kg
            - recycled_mass_kg
            - disposed_mass_kg
            - loss_mass_kg;

        Ok(LifecycleAccounting {
            initial_mass_kg: self.passport.initial_mass_kg,
            active_mass_kg: snapshot.mass_kg,
            recycled_mass_kg,
            disposed_mass_kg,
            loss_mass_kg,
            residual_kg,
        })
    }
}

fn feed_u64(hash: &mut u64, value: u64) {
    for byte in value.to_le_bytes() {
        *hash ^= u64::from(byte);
        *hash = hash.wrapping_mul(0x100_0000_01b3);
    }
}

fn feed_text(hash: &mut u64, value: &str) {
    for byte in value.as_bytes() {
        *hash ^= u64::from(*byte);
        *hash = hash.wrapping_mul(0x100_0000_01b3);
    }
}

fn stable_digest(config: &LifecycleFixtureConfig, ledger: &ComponentLifecycleLedger) -> u64 {
    let mut hash = 0xcbf2_9ce4_8422_2325_u64;
    feed_u64(&mut hash, LIFECYCLE_SCHEMA_VERSION as u64);
    feed_u64(&mut hash, config.seed);
    feed_u64(&mut hash, config.initial_mass_kg.to_bits());
    feed_u64(&mut hash, config.first_recovery_loss_fraction.to_bits());
    feed_u64(&mut hash, config.second_recovery_loss_fraction.to_bits());
    feed_u64(&mut hash, config.late_final_recording_delay_years as u64);
    feed_text(&mut hash, &ledger.passport.component_id);
    feed_text(&mut hash, &ledger.passport.material_id);
    feed_text(&mut hash, &ledger.passport.passport_id);
    feed_text(&mut hash, &ledger.passport.source_geometry_id);
    feed_text(&mut hash, &ledger.passport.connection_type);
    feed_text(&mut hash, &ledger.passport.initial_use_class);
    feed_u64(&mut hash, ledger.passport.initial_mass_kg.to_bits());

    for event in &ledger.events {
        feed_u64(&mut hash, event.event_id);
        feed_u64(&mut hash, event.revision as u64);
        feed_u64(&mut hash, event.prior_event_id.unwrap_or(u64::MAX));
        feed_text(&mut hash, event.component_id.as_str());
        feed_text(&mut hash, event.action.as_str());
        feed_text(&mut hash, event.state_before.as_str());
        feed_text(&mut hash, event.state_after.as_str());
        feed_text(&mut hash, event.condition_before.as_str());
        feed_text(&mut hash, event.condition_after.as_str());
        feed_u64(&mut hash, event.effective_at_micros as u64);
        feed_u64(&mut hash, event.recorded_at_micros as u64);
        feed_u64(&mut hash, event.mass_before_kg.to_bits());
        feed_u64(&mut hash, event.mass_after_kg.to_bits());
        feed_u64(&mut hash, event.mass_loss_kg.to_bits());
        feed_u64(&mut hash, event.recycled_mass_kg.to_bits());
        feed_u64(&mut hash, event.disposed_mass_kg.to_bits());
        feed_text(&mut hash, &event.supporting_record_id);
        feed_text(&mut hash, &event.use_class_after);
    }

    hash
}

impl LifecycleFixture {
    pub fn generate(config: LifecycleFixtureConfig) -> Result<Self, LifecycleError> {
        if !finite_nonnegative(config.initial_mass_kg) || config.initial_mass_kg == 0.0 {
            return Err(LifecycleError::InvalidMass);
        }
        if !(0.0..1.0).contains(&config.first_recovery_loss_fraction)
            || !(0.0..1.0).contains(&config.second_recovery_loss_fraction)
            || !config.first_recovery_loss_fraction.is_finite()
            || !config.second_recovery_loss_fraction.is_finite()
        {
            return Err(LifecycleError::InvalidLossFraction);
        }
        if config.late_final_recording_delay_years < 0
            || config.late_final_recording_delay_years > i64::MAX / YEAR_MICROS - 30
        {
            return Err(LifecycleError::InvalidRecordingDelay);
        }

        let passport = ComponentPassport {
            component_id: "fixture:component:beam-001".to_owned(),
            material_id: "fixture:material:engineered-timber".to_owned(),
            passport_id: "fixture:passport:beam-001".to_owned(),
            source_geometry_id: "fixture:geometry:assembly-r0".to_owned(),
            connection_type: "bolted-reversible-plate".to_owned(),
            initial_mass_kg: config.initial_mass_kg,
            initial_use_class: "floor_beam".to_owned(),
            initial_condition: ComponentCondition::Good,
        };

        let mut events = Vec::new();
        let mut state = LifecycleState::Designed;
        let mut condition = ComponentCondition::Good;
        let mut previous_event_id = None;
        let mass = passport.initial_mass_kg;

        let first_recovered = mass * (1.0 - config.first_recovery_loss_fraction);
        let second_recovered =
            first_recovered * (1.0 - config.second_recovery_loss_fraction);
        let mut next_event_id = 10_001_u64;

        let specifications = [
            (
                LifecycleAction::Install,
                LifecycleState::InService,
                ComponentCondition::Good,
                "floor_beam",
                0_i64,
                0_i64,
                mass,
                mass,
                0.0,
                0.0,
                0.0,
                "fixture:assembly:initial",
            ),
            (
                LifecycleAction::Inspect,
                LifecycleState::InService,
                ComponentCondition::Degraded,
                "floor_beam",
                5 * YEAR_MICROS,
                5 * YEAR_MICROS,
                mass,
                mass,
                0.0,
                0.0,
                0.0,
                "fixture:inspection:year-5",
            ),
            (
                LifecycleAction::Repair,
                LifecycleState::InService,
                ComponentCondition::Repaired,
                "floor_beam",
                5 * YEAR_MICROS + DAY_MICROS,
                5 * YEAR_MICROS + DAY_MICROS,
                mass,
                mass,
                0.0,
                0.0,
                0.0,
                "fixture:repair:year-5",
            ),
            (
                LifecycleAction::Remove,
                LifecycleState::Removed,
                ComponentCondition::Repaired,
                "floor_beam",
                10 * YEAR_MICROS,
                10 * YEAR_MICROS,
                mass,
                mass,
                0.0,
                0.0,
                0.0,
                "fixture:deconstruction:first",
            ),
            (
                LifecycleAction::Recover,
                LifecycleState::Recovered,
                ComponentCondition::Repaired,
                "floor_beam",
                10 * YEAR_MICROS + DAY_MICROS,
                10 * YEAR_MICROS + 2 * DAY_MICROS,
                mass,
                first_recovered,
                mass - first_recovered,
                0.0,
                0.0,
                "fixture:recovery:first",
            ),
            (
                LifecycleAction::Reuse,
                LifecycleState::InService,
                ComponentCondition::Repaired,
                "canopy_beam",
                11 * YEAR_MICROS,
                11 * YEAR_MICROS,
                first_recovered,
                first_recovered,
                0.0,
                0.0,
                0.0,
                "fixture:assembly:second-life",
            ),
            (
                LifecycleAction::Inspect,
                LifecycleState::InService,
                ComponentCondition::Degraded,
                "canopy_beam",
                15 * YEAR_MICROS,
                15 * YEAR_MICROS,
                first_recovered,
                first_recovered,
                0.0,
                0.0,
                0.0,
                "fixture:inspection:second-life",
            ),
            (
                LifecycleAction::Repair,
                LifecycleState::InService,
                ComponentCondition::Repaired,
                "canopy_beam",
                15 * YEAR_MICROS + DAY_MICROS,
                15 * YEAR_MICROS + DAY_MICROS,
                first_recovered,
                first_recovered,
                0.0,
                0.0,
                0.0,
                "fixture:repair:second-life",
            ),
            (
                LifecycleAction::Remove,
                LifecycleState::Removed,
                ComponentCondition::Repaired,
                "canopy_beam",
                20 * YEAR_MICROS,
                20 * YEAR_MICROS,
                first_recovered,
                first_recovered,
                0.0,
                0.0,
                0.0,
                "fixture:deconstruction:second",
            ),
            (
                LifecycleAction::Recover,
                LifecycleState::Recovered,
                ComponentCondition::Repaired,
                "canopy_beam",
                20 * YEAR_MICROS + DAY_MICROS,
                20 * YEAR_MICROS + 2 * DAY_MICROS,
                first_recovered,
                second_recovered,
                first_recovered - second_recovered,
                0.0,
                0.0,
                "fixture:recovery:second",
            ),
            (
                LifecycleAction::Recycle,
                LifecycleState::Recycled,
                ComponentCondition::Repaired,
                "canopy_beam",
                30 * YEAR_MICROS,
                (30 + config.late_final_recording_delay_years) * YEAR_MICROS,
                second_recovered,
                0.0,
                0.0,
                second_recovered,
                0.0,
                "fixture:recycling:final",
            ),
        ];

        for (
            action,
            next_state,
            next_condition,
            next_use_class,
            effective_at,
            recorded_at,
            mass_before,
            mass_after,
            mass_loss,
            recycled_mass,
            disposed_mass,
            supporting_record_id,
        ) in specifications
        {
            let event = ComponentLifecycleEvent {
                event_id: next_event_id,
                component_id: passport.component_id.clone(),
                revision: events.len() as u32 + 1,
                prior_event_id: previous_event_id,
                action,
                state_before: state,
                state_after: next_state,
                condition_before: condition,
                condition_after: next_condition,
                use_class_after: next_use_class.to_owned(),
                effective_at_micros: effective_at,
                recorded_at_micros: recorded_at,
                mass_before_kg: mass_before,
                mass_after_kg: mass_after,
                mass_loss_kg: mass_loss,
                recycled_mass_kg: recycled_mass,
                disposed_mass_kg: disposed_mass,
                supporting_record_id: supporting_record_id.to_owned(),
            };

            events.push(event);
            previous_event_id = Some(next_event_id);
            next_event_id += 1;
            state = next_state;
            condition = next_condition;
        }

        let ledger = ComponentLifecycleLedger { passport, events };
        ledger.validate()?;

        let mut fixture = Self {
            manifest: LifecycleFixtureManifest {
                schema_version: LIFECYCLE_SCHEMA_VERSION,
                generator: LIFECYCLE_FIXTURE_GENERATOR,
                config: config.clone(),
                digest_non_security: 0,
                component_count: 1,
                event_count: ledger.events.len(),
            },
            ledger,
        };

        fixture.manifest.digest_non_security =
            stable_digest(&fixture.manifest.config, &fixture.ledger);
        Ok(fixture)
    }

    pub fn validate(&self) -> Result<(), LifecycleError> {
        self.ledger.validate()?;
        if self.manifest.schema_version != LIFECYCLE_SCHEMA_VERSION
            || self.manifest.generator != LIFECYCLE_FIXTURE_GENERATOR
            || self.manifest.component_count != 1
            || self.manifest.event_count != self.ledger.events.len()
            || self.manifest.digest_non_security
                != stable_digest(&self.manifest.config, &self.ledger)
        {
            return Err(LifecycleError::ManifestMismatch);
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lifecycle_fixture_is_deterministic_for_a_fixed_seed_and_config() {
        let config = LifecycleFixtureConfig::default();
        let first = LifecycleFixture::generate(config.clone()).unwrap();
        let second = LifecycleFixture::generate(config).unwrap();

        assert_eq!(first, second);
        assert!(first.validate().is_ok());
    }

    #[test]
    fn two_use_cycles_close_mass_balance_exactly_with_explicit_recovery_loss() {
        let fixture = LifecycleFixture::generate(LifecycleFixtureConfig::default()).unwrap();
        let accounting = fixture
            .ledger
            .accounting_as_of(i64::MAX)
            .unwrap();

        assert!(mass_close(accounting.residual_kg, 0.0));
        assert_eq!(accounting.active_mass_kg, 0.0);
        assert_eq!(accounting.disposed_mass_kg, 0.0);
        assert!(accounting.recycled_mass_kg > 0.0);
        assert!(accounting.loss_mass_kg > 0.0);
    }

    #[test]
    fn late_recorded_recycling_does_not_leak_into_an_earlier_decision_cutoff() {
        let fixture = LifecycleFixture::generate(LifecycleFixtureConfig::default()).unwrap();
        let recycle = fixture.ledger.events.last().unwrap();
        let before_recording = recycle.effective_at_micros + YEAR_MICROS / 2;

        let earlier = fixture
            .ledger
            .snapshot_as_of(before_recording)
            .unwrap();
        let earlier_accounting = fixture
            .ledger
            .accounting_as_of(before_recording)
            .unwrap();

        assert_eq!(earlier.state, LifecycleState::Recovered);
        assert!(earlier.mass_kg > 0.0);
        assert_eq!(earlier_accounting.recycled_mass_kg, 0.0);

        let after = fixture
            .ledger
            .snapshot_as_of(recycle.recorded_at_micros)
            .unwrap();
        let after_accounting = fixture
            .ledger
            .accounting_as_of(recycle.recorded_at_micros)
            .unwrap();

        assert_eq!(after.state, LifecycleState::Recycled);
        assert_eq!(after.mass_kg, 0.0);
        assert!(mass_close(after_accounting.residual_kg, 0.0));
    }

    #[test]
    fn reuse_preserves_identity_but_changes_the_functional_use_class() {
        let fixture = LifecycleFixture::generate(LifecycleFixtureConfig::default()).unwrap();
        let snapshot = fixture
            .ledger
            .snapshot_as_of(12 * YEAR_MICROS)
            .unwrap();

        assert_eq!(snapshot.state, LifecycleState::InService);
        assert_eq!(snapshot.use_class, "canopy_beam");
        assert_eq!(
            fixture.ledger.passport.component_id,
            "fixture:component:beam-001"
        );
    }

    #[test]
    fn unsupported_transition_is_rejected() {
        let mut fixture = LifecycleFixture::generate(LifecycleFixtureConfig::default()).unwrap();
        fixture.ledger.events[0].action = LifecycleAction::Recycle;
        assert!(matches!(
            fixture.ledger.validate(),
            Err(LifecycleError::InvalidTransition(10_001))
        ));
    }

    #[test]
    fn mass_imbalance_is_rejected() {
        let mut fixture = LifecycleFixture::generate(LifecycleFixtureConfig::default()).unwrap();
        fixture.ledger.events[4].mass_after_kg += 1.0;
        assert!(matches!(
            fixture.ledger.validate(),
            Err(LifecycleError::MassImbalance(10_005))
                | Err(LifecycleError::MassDiscontinuity(10_006))
        ));
    }

    #[test]
    fn invalid_recovery_loss_parameters_fail_closed() {
        assert!(matches!(
            LifecycleFixture::generate(LifecycleFixtureConfig {
                first_recovery_loss_fraction: 1.0,
                ..LifecycleFixtureConfig::default()
            }),
            Err(LifecycleError::InvalidLossFraction)
        ));
        assert!(matches!(
            LifecycleFixture::generate(LifecycleFixtureConfig {
                late_final_recording_delay_years: -1,
                ..LifecycleFixtureConfig::default()
            }),
            Err(LifecycleError::InvalidRecordingDelay)
        ));
    }
}
