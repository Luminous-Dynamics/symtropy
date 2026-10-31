//! Essential-need intake and deterministic allocation policies.
//!
//! Need intake intentionally accepts only an authenticated subject and a
//! bounded request. It does not accept social, economic, credential, or
//! contribution scores. The intake layer records a claim of need; it does not
//! decide whether the person is morally deserving.
//!
//! Allocation policies are explicit simulation arms. None is asserted to be
//! universally fair. Real scarcity rules require public authorization,
//! accessible reasons, review, and appeal.

use std::collections::BTreeMap;

/// Opaque subject identifier supplied by a trusted identity adapter.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct AuthenticatedSubject(String);

impl AuthenticatedSubject {
    /// Construct a subject after the caller's identity adapter has authenticated it.
    ///
    /// This constructor is a trust boundary, not a cryptographic authentication
    /// primitive. Production adapters must derive the identifier from a verified
    /// session (for Holochain, the authenticated agent context), never from an
    /// untrusted request body.
    pub fn from_verified_adapter(value: impl Into<String>) -> Result<Self, NeedError> {
        let value = value.into();
        let normalized = value.trim();
        if normalized.is_empty() || normalized.len() > 256 {
            return Err(NeedError::InvalidSubject);
        }
        Ok(Self(normalized.to_owned()))
    }

    /// Return the opaque subject identifier.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Category of an essential need.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NeedCategory {
    /// Food and basic nutrition.
    Food,
    /// Safe drinking water and sanitation.
    Water,
    /// Safe shelter and housing.
    Shelter,
    /// Healthcare and essential medicines.
    Healthcare,
    /// Education and learning access.
    Education,
    /// Basic energy access.
    Energy,
    /// Basic digital connectivity.
    Connectivity,
    /// Care and support services.
    Care,
    /// A locally defined category with a bounded label.
    Other(String),
}

impl NeedCategory {
    fn validate(&self) -> Result<(), NeedError> {
        if let Self::Other(label) = self {
            if label.trim().is_empty() || label.len() > 64 {
                return Err(NeedError::InvalidCategory);
            }
        }
        Ok(())
    }
}

/// Urgency declaration for a need.
///
/// This is a claim used by explicit allocation policies, not a measure of a
/// person's worth. A deployed service needs review and appeal procedures for
/// disputed urgency classifications.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum NeedSeverity {
    /// Routine / non-urgent need.
    Low,
    /// Need should be addressed soon.
    Normal,
    /// Significant time-sensitive need.
    High,
    /// Imminent risk to health, safety, or basic subsistence.
    Critical,
}

impl NeedSeverity {
    fn priority_rank(self) -> u8 {
        match self {
            Self::Low => 0,
            Self::Normal => 1,
            Self::High => 2,
            Self::Critical => 3,
        }
    }
}

/// Minimal request data accepted by the intake kernel.
///
/// Intentionally absent: free-text sensitive narrative, requester identity,
/// wealth, reputation, credentials, Phi, contribution, and reciprocity offers.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeedRequest {
    /// Requested resource/service category.
    pub category: NeedCategory,
    /// Urgency declared for this request.
    pub severity: NeedSeverity,
    /// Requested quantity in the category's canonical integer unit.
    pub quantity: u64,
}

/// Stable identifier assigned by the intake kernel.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd, Hash)]
pub struct NeedId(u64);

impl NeedId {
    /// Return the stable numeric identifier.
    pub const fn value(self) -> u64 {
        self.0
    }
}

/// A stored need case with privacy-minimal structured fields.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeedCase {
    id: NeedId,
    subject: AuthenticatedSubject,
    category: NeedCategory,
    severity: NeedSeverity,
    requested: u64,
    fulfilled: u64,
    received_at: u64,
}

impl NeedCase {
    /// Return this case's identifier.
    pub const fn id(&self) -> NeedId {
        self.id
    }

    /// Return the authenticated subject identifier.
    pub fn subject(&self) -> &AuthenticatedSubject {
        &self.subject
    }

    /// Return this case's category.
    pub fn category(&self) -> &NeedCategory {
        &self.category
    }

    /// Return its current urgency declaration.
    pub const fn severity(&self) -> NeedSeverity {
        self.severity
    }

    /// Return the originally requested quantity.
    pub const fn requested(&self) -> u64 {
        self.requested
    }

    /// Return the quantity already fulfilled.
    pub const fn fulfilled(&self) -> u64 {
        self.fulfilled
    }

    /// Return the outstanding quantity.
    pub const fn remaining(&self) -> u64 {
        self.requested - self.fulfilled
    }

    /// Return the trusted intake timestamp (integer epoch units chosen by the adapter).
    pub const fn received_at(&self) -> u64 {
        self.received_at
    }

    /// Return whether any quantity remains unmet.
    pub const fn is_open(&self) -> bool {
        self.fulfilled < self.requested
    }
}

/// Explicit policy arms supported by the initial kernel.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AllocationPolicy {
    /// Higher urgency first, then earlier trusted receipt time, then stable ID.
    PriorityThenOldest,
    /// Earlier trusted receipt time, then stable ID, regardless of urgency.
    OldestFirst,
    /// Proportional allocation with deterministic largest-remainder rounding.
    ProRata,
}

/// A committed partial or complete allocation for one need.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Allocation {
    /// Need receiving the allocation.
    pub need_id: NeedId,
    /// Integer quantity allocated in this operation.
    pub quantity: u64,
    /// Remaining unmet quantity after this operation.
    pub remaining_after: u64,
}

/// Errors returned by the need-intake and allocation kernel.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum NeedError {
    /// The adapter supplied an empty or overlong subject identifier.
    InvalidSubject,
    /// A request has zero quantity.
    InvalidQuantity,
    /// An `Other` category label is empty or overlong.
    InvalidCategory,
    /// The subject already has an open need in this category.
    DuplicateOpenNeed,
    /// The monotonic need identifier space is exhausted.
    IdExhausted,
    /// A quantity calculation overflowed.
    QuantityOverflow,
    /// An allocation references a case that no longer exists.
    UnknownNeed,
}

/// In-memory deterministic need intake and allocation state.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeedIntake {
    next_id: u64,
    cases: BTreeMap<NeedId, NeedCase>,
}

impl Default for NeedIntake {
    fn default() -> Self {
        Self::new()
    }
}

impl NeedIntake {
    /// Create an empty need-intake instance.
    pub fn new() -> Self {
        Self { next_id: 1, cases: BTreeMap::new() }
    }

    /// Accept a need from an authenticated subject.
    ///
    /// One open need per subject/category prevents a requester from splitting a
    /// single claim into multiple queue positions in this initial model. A
    /// fulfilled case may be followed by a new request.
    pub fn submit(
        &mut self,
        subject: AuthenticatedSubject,
        request: NeedRequest,
        received_at: u64,
    ) -> Result<NeedId, NeedError> {
        request.category.validate()?;
        if request.quantity == 0 {
            return Err(NeedError::InvalidQuantity);
        }
        if self.cases.values().any(|case| {
            case.subject == subject && case.category == request.category && case.is_open()
        }) {
            return Err(NeedError::DuplicateOpenNeed);
        }

        let id = NeedId(self.next_id);
        let next_id = self.next_id.checked_add(1).ok_or(NeedError::IdExhausted)?;
        let case = NeedCase {
            id,
            subject,
            category: request.category,
            severity: request.severity,
            requested: request.quantity,
            fulfilled: 0,
            received_at,
        };

        self.cases.insert(id, case);
        self.next_id = next_id;
        Ok(id)
    }

    /// Fetch a need case by its identifier.
    pub fn get(&self, id: NeedId) -> Option<&NeedCase> {
        self.cases.get(&id)
    }

    /// Iterate over cases in stable identifier order.
    pub fn cases(&self) -> impl Iterator<Item = &NeedCase> {
        self.cases.values()
    }

    /// Allocate available physical supply according to an explicit policy.
    ///
    /// Allocations never exceed the available quantity. Unmet need remains on
    /// each case. The operation is deterministic for identical state, policy,
    /// category and supply. It does not inspect money, reputation or credentials.
    pub fn allocate(
        &mut self,
        category: &NeedCategory,
        available: u64,
        policy: AllocationPolicy,
    ) -> Result<Vec<Allocation>, NeedError> {
        if available == 0 {
            return Ok(Vec::new());
        }

        let mut candidates: Vec<(NeedId, u64, NeedSeverity, u64)> = self
            .cases
            .values()
            .filter(|case| &case.category == category && case.is_open())
            .map(|case| (case.id, case.remaining(), case.severity, case.received_at))
            .collect();

        if candidates.is_empty() {
            return Ok(Vec::new());
        }

        let plan = match policy {
            AllocationPolicy::PriorityThenOldest => {
                candidates.sort_by(|a, b| {
                    b.2.priority_rank()
                        .cmp(&a.2.priority_rank())
                        .then_with(|| a.3.cmp(&b.3))
                        .then_with(|| a.0.cmp(&b.0))
                });
                greedy_plan(&candidates, available)
            }
            AllocationPolicy::OldestFirst => {
                candidates.sort_by(|a, b| a.3.cmp(&b.3).then_with(|| a.0.cmp(&b.0)));
                greedy_plan(&candidates, available)
            }
            AllocationPolicy::ProRata => proportional_plan(&candidates, available)?,
        };

        // Validate the complete plan before mutating any case.
        for (id, quantity) in &plan {
            let case = self.cases.get(id).ok_or(NeedError::UnknownNeed)?;
            if *quantity > case.remaining() {
                return Err(NeedError::QuantityOverflow);
            }
        }

        let mut allocations = Vec::with_capacity(plan.len());
        for (id, quantity) in plan {
            if quantity == 0 {
                continue;
            }
            let case = self.cases.get_mut(&id).ok_or(NeedError::UnknownNeed)?;
            case.fulfilled = case.fulfilled.checked_add(quantity).ok_or(NeedError::QuantityOverflow)?;
            allocations.push(Allocation {
                need_id: id,
                quantity,
                remaining_after: case.remaining(),
            });
        }
        Ok(allocations)
    }
}

fn greedy_plan(
    candidates: &[(NeedId, u64, NeedSeverity, u64)],
    available: u64,
) -> Vec<(NeedId, u64)> {
    let mut remaining_supply = available;
    let mut result = Vec::new();
    for (id, unmet, _, _) in candidates {
        if remaining_supply == 0 {
            break;
        }
        let quantity = (*unmet).min(remaining_supply);
        if quantity > 0 {
            result.push((*id, quantity));
            remaining_supply -= quantity;
        }
    }
    result
}

fn proportional_plan(
    candidates: &[(NeedId, u64, NeedSeverity, u64)],
    available: u64,
) -> Result<Vec<(NeedId, u64)>, NeedError> {
    let total_unmet = candidates.iter().try_fold(0_u128, |sum, item| {
        sum.checked_add(item.1 as u128).ok_or(NeedError::QuantityOverflow)
    })?;
    if total_unmet == 0 {
        return Ok(Vec::new());
    }

    let allocatable = (available as u128).min(total_unmet);
    let mut shares: Vec<(NeedId, u64, u128)> = Vec::with_capacity(candidates.len());
    let mut base_total = 0_u128;

    for (id, unmet, _, _) in candidates {
        let product = allocatable
            .checked_mul(*unmet as u128)
            .ok_or(NeedError::QuantityOverflow)?;
        let base = product / total_unmet;
        let remainder = product % total_unmet;
        let base = u64::try_from(base).map_err(|_| NeedError::QuantityOverflow)?;
        base_total = base_total.checked_add(base as u128).ok_or(NeedError::QuantityOverflow)?;
        shares.push((*id, base, remainder));
    }

    let extra = usize::try_from(allocatable - base_total).map_err(|_| NeedError::QuantityOverflow)?;
    shares.sort_by(|a, b| b.2.cmp(&a.2).then_with(|| a.0.cmp(&b.0)));
    if extra > shares.len() {
        return Err(NeedError::QuantityOverflow);
    }
    for share in shares.iter_mut().take(extra) {
        share.1 = share.1.checked_add(1).ok_or(NeedError::QuantityOverflow)?;
    }

    shares.sort_by_key(|share| share.0);
    Ok(shares.into_iter().filter(|share| share.1 > 0).map(|share| (share.0, share.1)).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn subject(name: &str) -> AuthenticatedSubject {
        AuthenticatedSubject::from_verified_adapter(name).expect("valid adapter subject")
    }

    fn request(category: NeedCategory, severity: NeedSeverity, quantity: u64) -> NeedRequest {
        NeedRequest { category, severity, quantity }
    }

    #[test]
    fn uncredentialed_subject_can_submit_without_any_social_score_inputs() {
        let mut intake = NeedIntake::new();
        let id = intake
            .submit(
                subject("subject-with-no-civic-profile"),
                request(NeedCategory::Food, NeedSeverity::Critical, 4),
                100,
            )
            .expect("need intake must not require a profile");
        let case = intake.get(id).expect("accepted case");
        assert_eq!(case.remaining(), 4);
    }

    #[test]
    fn duplicate_open_need_cannot_split_a_queue_position() {
        let mut intake = NeedIntake::new();
        let who = subject("alice");
        intake
            .submit(who.clone(), request(NeedCategory::Water, NeedSeverity::High, 10), 1)
            .expect("first request");
        let error = intake
            .submit(who, request(NeedCategory::Water, NeedSeverity::Critical, 10), 2)
            .expect_err("duplicate open request must be rejected");
        assert_eq!(error, NeedError::DuplicateOpenNeed);
    }

    #[test]
    fn severity_then_oldest_is_deterministic_and_supply_bounded() {
        let mut intake = NeedIntake::new();
        let low = intake
            .submit(subject("low"), request(NeedCategory::Food, NeedSeverity::Low, 5), 1)
            .unwrap();
        let critical = intake
            .submit(subject("critical"), request(NeedCategory::Food, NeedSeverity::Critical, 5), 2)
            .unwrap();
        let result = intake
            .allocate(&NeedCategory::Food, 3, AllocationPolicy::PriorityThenOldest)
            .unwrap();
        assert_eq!(result, vec![Allocation { need_id: critical, quantity: 3, remaining_after: 2 }]);
        assert_eq!(intake.get(low).unwrap().fulfilled(), 0);
    }

    #[test]
    fn proportional_allocation_uses_largest_remainder_and_never_overallocates() {
        let mut intake = NeedIntake::new();
        let ids = ["a", "b", "c"].map(|name| {
            intake
                .submit(subject(name), request(NeedCategory::Energy, NeedSeverity::Normal, 10), 10)
                .unwrap()
        });
        let result = intake.allocate(&NeedCategory::Energy, 5, AllocationPolicy::ProRata).unwrap();
        assert_eq!(result.iter().map(|a| a.quantity).sum::<u64>(), 5);
        assert_eq!(result.iter().map(|a| a.quantity).collect::<Vec<_>>(), vec![2, 2, 1]);
        assert!(result.iter().all(|a| a.remaining_after == 10 - a.quantity));
        assert_eq!(result.iter().map(|a| a.need_id).collect::<Vec<_>>(), ids.to_vec());
    }

    #[test]
    fn default_intake_starts_need_ids_at_one() {
        let mut intake = NeedIntake::default();
        let id = intake
            .submit(subject("first"), request(NeedCategory::Food, NeedSeverity::Normal, 1), 0)
            .unwrap();
        assert_eq!(id.value(), 1);
    }

    #[test]
    fn fulfilled_need_can_be_submitted_again() {
        let mut intake = NeedIntake::new();
        let who = subject("alice");
        let first = intake
            .submit(who.clone(), request(NeedCategory::Care, NeedSeverity.Normal, 2), 1)
            .unwrap();
        intake.allocate(&NeedCategory::Care, 2, AllocationPolicy::OldestFirst).unwrap();
        assert_eq!(intake.get(first).unwrap().remaining(), 0);
        assert!(intake.submit(who, request(NeedCategory::Care, NeedSeverity.Normal, 1), 2).is_ok());
    }
}
