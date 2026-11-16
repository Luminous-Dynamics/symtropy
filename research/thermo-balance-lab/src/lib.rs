//! Small executable energy-flow accounting model.
//!
//! This is a research oracle, not a production thermodynamics engine.
//! Stored quantities are SI joules. Impulse is kept in N·s and is never
//! treated as energy.

use std::{error::Error, fmt};

/// Finite, non-negative energy measured in joules.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Joules(f64);

impl Joules {
    pub fn new(value: f64) -> Result<Self, LedgerError> {
        if !value.is_finite() {
            return Err(LedgerError::NonFiniteEnergy);
        }
        if value < 0.0 {
            return Err(LedgerError::NegativeEnergy);
        }
        Ok(Self(value))
    }

    pub const fn zero() -> Self { Self(0.0) }
    pub const fn get(self) -> f64 { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EnergyStore { Kinetic, Thermal, InternalReservoir }

impl EnergyStore {
    const fn index(self) -> usize {
        match self {
            Self::Kinetic => 0,
            Self::Thermal => 1,
            Self::InternalReservoir => 2,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FlowKind { InternalTransfer, BoundaryInflow, BoundaryOutflow }

#[derive(Clone, Debug, PartialEq)]
pub struct EnergyFlowEvent {
    pub sequence: u64,
    pub from: Option<EnergyStore>,
    pub to: Option<EnergyStore>,
    pub amount: Joules,
    pub kind: FlowKind,
    pub causal_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LedgerError {
    NegativeEnergy,
    NonFiniteEnergy,
    InsufficientEnergy { store: EnergyStore },
    SameStoreTransfer,
    EmptyCausalId,
    InvalidMass,
    InvalidVelocity,
    InvalidRestitution,
    LedgerMismatch,
    SequenceOverflow,
}

impl fmt::Display for LedgerError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NegativeEnergy => write!(f, "energy must not be negative"),
            Self::NonFiniteEnergy => write!(f, "energy and derived state must be finite"),
            Self::InsufficientEnergy { store } => write!(f, "insufficient energy in {store:?}"),
            Self::SameStoreTransfer => write!(f, "source and destination stores must differ"),
            Self::EmptyCausalId => write!(f, "causal_id must not be empty"),
            Self::InvalidMass => write!(f, "mass must be finite and strictly positive"),
            Self::InvalidVelocity => write!(f, "velocity must be finite; A must approach B"),
            Self::InvalidRestitution => write!(f, "restitution must be finite and in [0, 1]"),
            Self::LedgerMismatch => write!(f, "ledger kinetic store does not match collision input"),
            Self::SequenceOverflow => write!(f, "event sequence overflow"),
        }
    }
}
impl Error for LedgerError {}

/// Energy accounting for an explicitly bounded control volume.
///
/// Invariant: current_total = initial_total + boundary_in - boundary_out.
/// Internal transfers change store composition but not total energy.
#[derive(Clone, Debug)]
pub struct EnergyLedger {
    stores: [f64; 3],
    initial_total: f64,
    boundary_in: f64,
    boundary_out: f64,
    events: Vec<EnergyFlowEvent>,
    next_sequence: u64,
}

impl EnergyLedger {
    pub fn new(kinetic: Joules, thermal: Joules, internal_reservoir: Joules)
        -> Result<Self, LedgerError>
    {
        let initial_total = kinetic.get() + thermal.get() + internal_reservoir.get();
        if !initial_total.is_finite() { return Err(LedgerError::NonFiniteEnergy); }
        Ok(Self {
            stores: [kinetic.get(), thermal.get(), internal_reservoir.get()],
            initial_total,
            boundary_in: 0.0,
            boundary_out: 0.0,
            events: Vec::new(),
            next_sequence: 0,
        })
    }

    pub fn amount(&self, store: EnergyStore) -> Joules { Joules(self.stores[store.index()]) }
    pub fn total(&self) -> f64 { self.stores.iter().sum() }
    pub fn boundary_in(&self) -> f64 { self.boundary_in }
    pub fn boundary_out(&self) -> f64 { self.boundary_out }
    pub fn events(&self) -> &[EnergyFlowEvent] { &self.events }

    pub fn residual(&self) -> f64 {
        self.total() - (self.initial_total + self.boundary_in - self.boundary_out)
    }

    fn next_sequence(&self) -> Result<u64, LedgerError> {
        self.next_sequence.checked_add(1).ok_or(LedgerError::SequenceOverflow)?;
        Ok(self.next_sequence)
    }

    fn validate_causal_id(id: &str) -> Result<(), LedgerError> {
        if id.trim().is_empty() { return Err(LedgerError::EmptyCausalId); }
        Ok(())
    }

    /// Atomic internal transfer between stores.
    pub fn transfer(
        &mut self,
        from: EnergyStore,
        to: EnergyStore,
        amount: Joules,
        causal_id: impl Into<String>,
    ) -> Result<(), LedgerError> {
        if from == to { return Err(LedgerError::SameStoreTransfer); }
        let causal_id = causal_id.into();
        Self::validate_causal_id(&causal_id)?;
        let sequence = self.next_sequence()?;
        let fi = from.index();
        let ti = to.index();
        let value = amount.get();
        if self.stores[fi] + f64::EPSILON < value {
            return Err(LedgerError::InsufficientEnergy { store: from });
        }
        let next_from = self.stores[fi] - value;
        let next_to = self.stores[ti] + value;
        if !next_from.is_finite() || !next_to.is_finite() || next_from < 0.0 {
            return Err(LedgerError::NonFiniteEnergy);
        }
        self.stores[fi] = next_from.max(0.0);
        self.stores[ti] = next_to;
        self.events.push(EnergyFlowEvent {
            sequence, from: Some(from), to: Some(to), amount,
            kind: FlowKind::InternalTransfer, causal_id,
        });
        self.next_sequence = sequence + 1;
        Ok(())
    }

    /// Add energy across the declared control-volume boundary.
    pub fn boundary_inflow(
        &mut self,
        to: EnergyStore,
        amount: Joules,
        causal_id: impl Into<String>,
    ) -> Result<(), LedgerError> {
        let causal_id = causal_id.into();
        Self::validate_causal_id(&causal_id)?;
        let sequence = self.next_sequence()?;
        let i = to.index();
        let next_store = self.stores[i] + amount.get();
        let next_in = self.boundary_in + amount.get();
        if !next_store.is_finite() || !next_in.is_finite() {
            return Err(LedgerError::NonFiniteEnergy);
        }
        self.stores[i] = next_store;
        self.boundary_in = next_in;
        self.events.push(EnergyFlowEvent {
            sequence, from: None, to: Some(to), amount,
            kind: FlowKind::BoundaryInflow, causal_id,
        });
        self.next_sequence = sequence + 1;
        Ok(())
    }

    /// Remove energy from a store across the declared control-volume boundary.
    pub fn boundary_outflow(
        &mut self,
        from: EnergyStore,
        amount: Joules,
        causal_id: impl Into<String>,
    ) -> Result<(), LedgerError> {
        let causal_id = causal_id.into();
        Self::validate_causal_id(&causal_id)?;
        let sequence = self.next_sequence()?;
        let i = from.index();
        let value = amount.get();
        if self.stores[i] + f64::EPSILON < value {
            return Err(LedgerError::InsufficientEnergy { store: from });
        }
        let next_store = self.stores[i] - value;
        let next_out = self.boundary_out + value;
        if !next_store.is_finite() || !next_out.is_finite() || next_store < 0.0 {
            return Err(LedgerError::NonFiniteEnergy);
        }
        self.stores[i] = next_store.max(0.0);
        self.boundary_out = next_out;
        self.events.push(EnergyFlowEvent {
            sequence, from: Some(from), to: None, amount,
            kind: FlowKind::BoundaryOutflow, causal_id,
        });
        self.next_sequence = sequence + 1;
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Collision1D {
    pub impulse_ns: f64,
    pub v_a_after: f64,
    pub v_b_after: f64,
    pub kinetic_before_j: f64,
    pub kinetic_after_j: f64,
    pub dissipated_to_heat_j: f64,
}

/// Solve a 1D impact where body A approaches body B (v_a > v_b).
/// Impulse is in N·s; kinetic-energy loss is independently in joules.
pub fn solve_collision_1d(
    mass_a_kg: f64,
    mass_b_kg: f64,
    velocity_a_mps: f64,
    velocity_b_mps: f64,
    restitution: f64,
) -> Result<Collision1D, LedgerError> {
    if !mass_a_kg.is_finite() || mass_a_kg <= 0.0
        || !mass_b_kg.is_finite() || mass_b_kg <= 0.0 {
        return Err(LedgerError::InvalidMass);
    }
    if !velocity_a_mps.is_finite() || !velocity_b_mps.is_finite()
        || velocity_a_mps <= velocity_b_mps {
        return Err(LedgerError::InvalidVelocity);
    }
    if !restitution.is_finite() || !(0.0..=1.0).contains(&restitution) {
        return Err(LedgerError::InvalidRestitution);
    }

    let inverse_mass_sum = 1.0 / mass_a_kg + 1.0 / mass_b_kg;
    let relative_speed = velocity_a_mps - velocity_b_mps;
    let impulse_ns = (1.0 + restitution) * relative_speed / inverse_mass_sum;
    let v_a_after = velocity_a_mps - impulse_ns / mass_a_kg;
    let v_b_after = velocity_b_mps + impulse_ns / mass_b_kg;
    let kinetic_before_j =
        0.5 * mass_a_kg * velocity_a_mps.powi(2) + 0.5 * mass_b_kg * velocity_b_mps.powi(2);
    let kinetic_after_j =
        0.5 * mass_a_kg * v_a_after.powi(2) + 0.5 * mass_b_kg * v_b_after.powi(2);
    let heat_j = kinetic_before_j - kinetic_after_j;

    if !impulse_ns.is_finite() || !v_a_after.is_finite() || !v_b_after.is_finite()
        || !kinetic_before_j.is_finite() || !kinetic_after_j.is_finite() || !heat_j.is_finite() {
        return Err(LedgerError::NonFiniteEnergy);
    }
    if heat_j < -1e-12 * kinetic_before_j.max(1.0) {
        return Err(LedgerError::LedgerMismatch);
    }

    Ok(Collision1D {
        impulse_ns, v_a_after, v_b_after, kinetic_before_j, kinetic_after_j,
        dissipated_to_heat_j: heat_j.max(0.0),
    })
}

/// Account the collision's actual kinetic-energy difference as internal heat.
/// The kinetic store must represent exactly these two bodies' initial kinetic energy.
pub fn collide_and_account(
    ledger: &mut EnergyLedger,
    mass_a_kg: f64,
    mass_b_kg: f64,
    velocity_a_mps: f64,
    velocity_b_mps: f64,
    restitution: f64,
    causal_id: impl Into<String>,
) -> Result<Collision1D, LedgerError> {
    let result = solve_collision_1d(
        mass_a_kg, mass_b_kg, velocity_a_mps, velocity_b_mps, restitution,
    )?;
    let ledger_kinetic = ledger.amount(EnergyStore::Kinetic).get();
    if (ledger_kinetic - result.kinetic_before_j).abs() > 1e-10 * result.kinetic_before_j.max(1.0) {
        return Err(LedgerError::LedgerMismatch);
    }
    if result.dissipated_to_heat_j > 0.0 {
        ledger.transfer(
            EnergyStore::Kinetic,
            EnergyStore::Thermal,
            Joules::new(result.dissipated_to_heat_j)?,
            causal_id,
        )?;
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn j(x: f64) -> Joules { Joules::new(x).expect("valid fixture") }
    fn near(a: f64, b: f64, tol: f64) {
        assert!((a - b).abs() <= tol, "a={a:.16e}, b={b:.16e}, tol={tol:.3e}");
    }

    #[test]
    fn rejects_negative_and_non_finite_energy() {
        assert_eq!(Joules::new(-0.1), Err(LedgerError::NegativeEnergy));
        assert_eq!(Joules::new(f64::NAN), Err(LedgerError::NonFiniteEnergy));
        assert_eq!(Joules::new(f64::INFINITY), Err(LedgerError::NonFiniteEnergy));
        assert_eq!(Joules::new(f64::NEG_INFINITY), Err(LedgerError::NonFiniteEnergy));
    }

    #[test]
    fn elastic_collision_preserves_kinetic_energy() {
        let r = solve_collision_1d(1.0, 1.0, 1.0, 0.0, 1.0).unwrap();
        near(r.impulse_ns, 1.0, 1e-12);
        near(r.v_a_after, 0.0, 1e-12);
        near(r.v_b_after, 1.0, 1e-12);
        near(r.kinetic_before_j, r.kinetic_after_j, 1e-12);
        near(r.dissipated_to_heat_j, 0.0, 1e-12);
    }

    #[test]
    fn inelastic_collision_transfers_kinetic_loss_to_heat() {
        let mut ledger = EnergyLedger::new(j(0.5), j(0.2), j(0.0)).unwrap();
        let r = collide_and_account(&mut ledger, 1.0, 1.0, 1.0, 0.0, 0.5, "impact-001").unwrap();
        near(r.impulse_ns, 0.75, 1e-12);
        near(r.kinetic_after_j, 0.3125, 1e-12);
        near(r.dissipated_to_heat_j, 0.1875, 1e-12);
        near(ledger.amount(EnergyStore::Kinetic).get(), 0.3125, 1e-12);
        near(ledger.amount(EnergyStore::Thermal).get(), 0.3875, 1e-12);
        near(ledger.total(), 0.7, 1e-12);
        near(ledger.residual(), 0.0, 1e-12);
        assert_eq!(ledger.events().len(), 1);
    }

    #[test]
    fn boundary_transfers_are_explicit() {
        let mut ledger = EnergyLedger::new(j(1.0), j(0.0), j(0.0)).unwrap();
        ledger.boundary_inflow(EnergyStore::Thermal, j(2.0), "heater").unwrap();
        ledger.boundary_outflow(EnergyStore::Kinetic, j(0.25), "work-export").unwrap();
        near(ledger.total(), 2.75, 1e-12);
        near(ledger.residual(), 0.0, 1e-12);
        near(ledger.boundary_in(), 2.0, 1e-12);
        near(ledger.boundary_out(), 0.25, 1e-12);
    }

    #[test]
    fn insufficient_transfer_is_atomic() {
        let mut ledger = EnergyLedger::new(j(0.5), j(0.0), j(0.0)).unwrap();
        assert_eq!(
            ledger.transfer(EnergyStore::Kinetic, EnergyStore::Thermal, j(0.6), "too-much"),
            Err(LedgerError::InsufficientEnergy { store: EnergyStore::Kinetic })
        );
        near(ledger.total(), 0.5, 0.0);
        assert!(ledger.events().is_empty());
        near(ledger.residual(), 0.0, 0.0);
    }

    #[test]
    fn same_store_and_empty_causal_id_are_rejected() {
        let mut ledger = EnergyLedger::new(j(1.0), j(0.0), j(0.0)).unwrap();
        assert_eq!(
            ledger.transfer(EnergyStore::Kinetic, EnergyStore::Kinetic, j(0.1), "self"),
            Err(LedgerError::SameStoreTransfer)
        );
        assert_eq!(
            ledger.transfer(EnergyStore::Kinetic, EnergyStore::Thermal, j(0.1), " "),
            Err(LedgerError::EmptyCausalId)
        );
        assert!(ledger.events().is_empty());
    }

    #[test]
    fn invalid_collision_parameters_are_rejected() {
        assert_eq!(solve_collision_1d(0.0, 1.0, 1.0, 0.0, 0.5), Err(LedgerError::InvalidMass));
        assert_eq!(solve_collision_1d(1.0, 1.0, f64::NAN, 0.0, 0.5), Err(LedgerError::InvalidVelocity));
        assert_eq!(solve_collision_1d(1.0, 1.0, 1.0, 0.0, 1.5), Err(LedgerError::InvalidRestitution));
        assert_eq!(solve_collision_1d(1.0, 1.0, 0.0, 1.0, 0.5), Err(LedgerError::InvalidVelocity));
    }

    #[test]
    fn mismatched_control_volume_is_rejected_without_mutation() {
        let mut ledger = EnergyLedger::new(j(10.0), j(0.0), j(0.0)).unwrap();
        assert_eq!(
            collide_and_account(&mut ledger, 1.0, 1.0, 1.0, 0.0, 0.5, "wrong-volume"),
            Err(LedgerError::LedgerMismatch)
        );
        near(ledger.total(), 10.0, 0.0);
        assert!(ledger.events().is_empty());
    }
}
