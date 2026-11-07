//! Bounded, deterministic observation delivery and reaction-maturation buffer.
//!
//! Sensor observation age, delivery age and policy reaction delay are distinct
//! clocks. This module does not generate observations or decide policy actions;
//! it preserves their provenance and selects only cues admitted by a frozen
//! decision cutoff and maturity/freshness contract.

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObservationAvailability {
    Observed,
    Missing,
    Occluded,
    Unsupported,
    Dropped,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DeliveryStamp {
    pub tick: u64,
    pub event_sequence: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CueSelectionDisposition {
    Selected,
    NoDeliveredCue,
    NoMatureCue,
    NoObservedCue,
    Expired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BufferError {
    ZeroCapacity,
    EmptyCueId,
    EmptySourceEventId,
    ObservationAfterGeneration,
    ZeroGeneratedEventSequence,
    PayloadAvailabilityMismatch,
    DuplicateCueId,
    CapacityExceeded,
    UnknownCueId,
    DuplicateDelivery,
    DeliveryBeforeGeneration,
    DeliveryEventNotAfterGeneration,
    NonMonotonicGeneratedEvent,
    NonMonotonicDeliveryEvent,
    NonMonotonicDeliveryTick,
}

/// A source observation before delivery. Availability is retained even when
/// there is no payload (Missing/Occluded/Unsupported/Dropped).
#[derive(Clone, Debug, PartialEq)]
pub struct BufferedObservation<T> {
    cue_id: String,
    source_event_id: String,
    observed_tick: u64,
    generated_tick: u64,
    generated_event_sequence: u64,
    availability: ObservationAvailability,
    payload: Option<T>,
    delivery: Option<DeliveryStamp>,
}

impl<T> BufferedObservation<T> {
    pub fn new(
        cue_id: impl Into<String>,
        source_event_id: impl Into<String>,
        observed_tick: u64,
        generated_tick: u64,
        generated_event_sequence: u64,
        availability: ObservationAvailability,
        payload: Option<T>,
    ) -> Result<Self, BufferError> {
        let cue_id = cue_id.into();
        let source_event_id = source_event_id.into();
        if cue_id.trim().is_empty() {
            return Err(BufferError::EmptyCueId);
        }
        if source_event_id.trim().is_empty() {
            return Err(BufferError::EmptySourceEventId);
        }
        if observed_tick > generated_tick {
            return Err(BufferError::ObservationAfterGeneration);
        }
        if generated_event_sequence == 0 {
            return Err(BufferError::ZeroGeneratedEventSequence);
        }
        let payload_matches = match availability {
            ObservationAvailability::Observed => payload.is_some(),
            ObservationAvailability::Missing
            | ObservationAvailability::Occluded
            | ObservationAvailability::Unsupported
            | ObservationAvailability::Dropped => payload.is_none(),
        };
        if !payload_matches {
            return Err(BufferError::PayloadAvailabilityMismatch);
        }
        Ok(Self {
            cue_id,
            source_event_id,
            observed_tick,
            generated_tick,
            generated_event_sequence,
            availability,
            payload,
            delivery: None,
        })
    }

    pub fn cue_id(&self) -> &str {
        &self.cue_id
    }

    pub fn source_event_id(&self) -> &str {
        &self.source_event_id
    }

    pub const fn observed_tick(&self) -> u64 {
        self.observed_tick
    }

    pub const fn generated_tick(&self) -> u64 {
        self.generated_tick
    }

    pub const fn generated_event_sequence(&self) -> u64 {
        self.generated_event_sequence
    }

    pub const fn availability(&self) -> ObservationAvailability {
        self.availability
    }

    pub const fn delivery(&self) -> Option<DeliveryStamp> {
        self.delivery
    }

    pub fn payload(&self) -> Option<&T> {
        self.payload.as_ref()
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BufferConfig {
    pub capacity: usize,
    pub reaction_delay_ticks: u64,
    /// Maximum source-observation age at decision time.
    pub max_observation_age_ticks: Option<u64>,
    /// Maximum time since delivery at decision time.
    pub max_delivery_age_ticks: Option<u64>,
}

impl BufferConfig {
    pub fn validate(self) -> Result<Self, BufferError> {
        if self.capacity == 0 {
            return Err(BufferError::ZeroCapacity);
        }
        Ok(self)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CueSelection<'a, T> {
    pub disposition: CueSelectionDisposition,
    /// Most recently delivered cue visible before the closed input cutoff.
    /// This may be immature; it is diagnostic and is not necessarily selected.
    pub newest_visible: Option<&'a BufferedObservation<T>>,
    /// Most recent cue that is delivered, mature, observed and within both
    /// freshness limits. A newer immature cue never overwrites an older mature
    /// one.
    pub selected: Option<&'a BufferedObservation<T>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ObservationDeliveryBuffer<T> {
    config: BufferConfig,
    entries: Vec<BufferedObservation<T>>,
    last_generated_event_sequence: Option<u64>,
    last_delivery_event_sequence: Option<u64>,
    last_delivery_tick: Option<u64>,
}

impl<T> ObservationDeliveryBuffer<T> {
    pub fn new(config: BufferConfig) -> Result<Self, BufferError> {
        let config = config.validate()?;
        Ok(Self {
            config,
            entries: Vec::with_capacity(config.capacity),
            last_generated_event_sequence: None,
            last_delivery_event_sequence: None,
            last_delivery_tick: None,
        })
    }

    pub const fn config(&self) -> BufferConfig {
        self.config
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn enqueue(&mut self, entry: BufferedObservation<T>) -> Result<(), BufferError> {
        if self.entries.iter().any(|current| current.cue_id == entry.cue_id) {
            return Err(BufferError::DuplicateCueId);
        }
        if self.entries.len() >= self.config.capacity {
            return Err(BufferError::CapacityExceeded);
        }
        if self
            .last_generated_event_sequence
            .is_some_and(|prior| entry.generated_event_sequence <= prior)
        {
            return Err(BufferError::NonMonotonicGeneratedEvent);
        }
        self.last_generated_event_sequence = Some(entry.generated_event_sequence);
        self.entries.push(entry);
        Ok(())
    }

    /// Append a delivery/drop event to an already queued observation. Delivery
    /// event order and tick order are monotonic within this recipient buffer.
    /// The observation's source time and generation event never change.
    pub fn deliver(
        &mut self,
        cue_id: &str,
        stamp: DeliveryStamp,
    ) -> Result<(), BufferError> {
        let entry = self
            .entries
            .iter_mut()
            .find(|entry| entry.cue_id == cue_id)
            .ok_or(BufferError::UnknownCueId)?;
        if entry.delivery.is_some() {
            return Err(BufferError::DuplicateDelivery);
        }
        if stamp.tick < entry.generated_tick {
            return Err(BufferError::DeliveryBeforeGeneration);
        }
        if stamp.event_sequence <= entry.generated_event_sequence {
            return Err(BufferError::DeliveryEventNotAfterGeneration);
        }
        if self
            .last_delivery_event_sequence
            .is_some_and(|prior| stamp.event_sequence <= prior)
        {
            return Err(BufferError::NonMonotonicDeliveryEvent);
        }
        if self
            .last_delivery_tick
            .is_some_and(|prior| stamp.tick < prior)
        {
            return Err(BufferError::NonMonotonicDeliveryTick);
        }
        entry.delivery = Some(stamp);
        self.last_delivery_event_sequence = Some(stamp.event_sequence);
        self.last_delivery_tick = Some(stamp.tick);
        Ok(())
    }

    /// Select a policy-usable cue using the exact decision input cutoff.
    /// Entries delivered after the cutoff are invisible even when their tick
    /// matches the current decision tick.
    pub fn select_matured(
        &self,
        decision_tick: u64,
        input_cutoff_event_sequence: u64,
    ) -> CueSelection<'_, T> {
        let mut newest_visible: Option<&BufferedObservation<T>> = None;
        let mut selected: Option<&BufferedObservation<T>> = None;
        let mut any_mature = false;
        let mut any_mature_observed = false;

        for entry in &self.entries {
            let Some(delivery) = entry.delivery else {
                continue;
            };
            if delivery.tick > decision_tick
                || delivery.event_sequence > input_cutoff_event_sequence
            {
                continue;
            }

            if newest_visible.is_none_or(|prior| {
                prior.delivery.is_some_and(|prior_delivery| {
                    delivery.event_sequence > prior_delivery.event_sequence
                })
            }) {
                newest_visible = Some(entry);
            }

            let delivery_age = decision_tick - delivery.tick;
            if delivery_age < self.config.reaction_delay_ticks {
                continue;
            }
            any_mature = true;

            if entry.availability != ObservationAvailability::Observed
                || entry.payload.is_none()
            {
                continue;
            }
            any_mature_observed = true;

            let observation_age = decision_tick - entry.observed_tick;
            if self
                .config
                .max_observation_age_ticks
                .is_some_and(|maximum| observation_age > maximum)
                || self
                    .config
                    .max_delivery_age_ticks
                    .is_some_and(|maximum| delivery_age > maximum)
            {
                continue;
            }

            if selected.is_none_or(|prior| {
                prior.delivery.is_some_and(|prior_delivery| {
                    delivery.event_sequence > prior_delivery.event_sequence
                })
            }) {
                selected = Some(entry);
            }
        }

        let disposition = if selected.is_some() {
            CueSelectionDisposition::Selected
        } else if newest_visible.is_none() {
            CueSelectionDisposition::NoDeliveredCue
        } else if !any_mature {
            CueSelectionDisposition::NoMatureCue
        } else if !any_mature_observed {
            CueSelectionDisposition::NoObservedCue
        } else {
            CueSelectionDisposition::Expired
        };

        CueSelection {
            disposition,
            newest_visible,
            selected,
        }
    }

    pub fn entries(&self) -> &[BufferedObservation<T>] {
        &self.entries
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config(
        capacity: usize,
        reaction_delay_ticks: u64,
        max_observation_age_ticks: Option<u64>,
        max_delivery_age_ticks: Option<u64>,
    ) -> BufferConfig {
        BufferConfig {
            capacity,
            reaction_delay_ticks,
            max_observation_age_ticks,
            max_delivery_age_ticks,
        }
    }

    fn observed(
        id: &str,
        observed_tick: u64,
        generated_tick: u64,
        generated_sequence: u64,
        value: i32,
    ) -> BufferedObservation<i32> {
        BufferedObservation::new(
            id,
            &format!("source:{id}"),
            observed_tick,
            generated_tick,
            generated_sequence,
            ObservationAvailability::Observed,
            Some(value),
        )
        .unwrap()
    }

    fn unavailable(
        id: &str,
        generated_tick: u64,
        generated_sequence: u64,
        availability: ObservationAvailability,
    ) -> BufferedObservation<i32> {
        BufferedObservation::new(
            id,
            &format!("source:{id}"),
            generated_tick.saturating_sub(1),
            generated_tick,
            generated_sequence,
            availability,
            None,
        )
        .unwrap()
    }

    #[test]
    fn zero_delay_zero_transport_selects_immediately() {
        let mut buffer = ObservationDeliveryBuffer::new(config(4, 0, None, None)).unwrap();
        buffer.enqueue(observed("cue-0", 0, 0, 1, 17)).unwrap();
        buffer
            .deliver(
                "cue-0",
                DeliveryStamp {
                    tick: 0,
                    event_sequence: 2,
                },
            )
            .unwrap();

        let selection = buffer.select_matured(0, 2);
        assert_eq!(selection.disposition, CueSelectionDisposition::Selected);
        assert_eq!(selection.selected.unwrap().payload(), Some(&17));
    }

    #[test]
    fn reaction_delay_becomes_eligible_at_exact_boundary() {
        let mut buffer = ObservationDeliveryBuffer::new(config(4, 1, None, None)).unwrap();
        buffer.enqueue(observed("cue-0", 0, 0, 1, 21)).unwrap();
        buffer
            .deliver(
                "cue-0",
                DeliveryStamp {
                    tick: 0,
                    event_sequence: 2,
                },
            )
            .unwrap();

        assert_eq!(
            buffer.select_matured(0, 2).disposition,
            CueSelectionDisposition::NoMatureCue
        );
        let selection = buffer.select_matured(1, 2);
        assert_eq!(selection.disposition, CueSelectionDisposition::Selected);
        assert_eq!(selection.selected.unwrap().payload(), Some(&21));
    }

    #[test]
    fn newer_immature_cue_does_not_overwrite_older_mature_cue() {
        let mut buffer = ObservationDeliveryBuffer::new(config(4, 1, None, None)).unwrap();
        buffer.enqueue(observed("cue-old", 0, 0, 1, 31)).unwrap();
        buffer
            .deliver(
                "cue-old",
                DeliveryStamp {
                    tick: 0,
                    event_sequence: 2,
                },
            )
            .unwrap();
        buffer.enqueue(observed("cue-new", 1, 1, 3, 41)).unwrap();
        buffer
            .deliver(
                "cue-new",
                DeliveryStamp {
                    tick: 1,
                    event_sequence: 4,
                },
            )
            .unwrap();

        let selection = buffer.select_matured(1, 4);
        assert_eq!(selection.disposition, CueSelectionDisposition::Selected);
        assert_eq!(selection.newest_visible.unwrap().cue_id(), "cue-new");
        assert_eq!(selection.selected.unwrap().cue_id(), "cue-old");
        assert_eq!(selection.selected.unwrap().payload(), Some(&31));
    }

    #[test]
    fn observation_age_and_delivery_age_have_independent_expiry_limits() {
        let mut buffer =
            ObservationDeliveryBuffer::new(config(4, 0, Some(2), Some(5))).unwrap();
        buffer.enqueue(observed("old-source", 0, 3, 1, 7)).unwrap();
        buffer
            .deliver(
                "old-source",
                DeliveryStamp {
                    tick: 3,
                    event_sequence: 2,
                },
            )
            .unwrap();

        assert_eq!(
            buffer.select_matured(3, 2).disposition,
            CueSelectionDisposition::Expired
        );

        let mut delivery_limited =
            ObservationDeliveryBuffer::new(config(4, 0, Some(8), Some(1))).unwrap();
        delivery_limited
            .enqueue(observed("late-delivery", 0, 0, 1, 9))
            .unwrap();
        delivery_limited
            .deliver(
                "late-delivery",
                DeliveryStamp {
                    tick: 1,
                    event_sequence: 2,
                },
            )
            .unwrap();
        assert_eq!(
            delivery_limited.select_matured(3, 2).disposition,
            CueSelectionDisposition::Expired
        );
    }

    #[test]
    fn unavailable_cue_dispositions_never_become_observed_payloads() {
        for availability in [
            ObservationAvailability::Missing,
            ObservationAvailability::Occluded,
            ObservationAvailability::Unsupported,
            ObservationAvailability::Dropped,
        ] {
            let mut buffer =
                ObservationDeliveryBuffer::new(config(2, 0, None, None)).unwrap();
            buffer
                .enqueue(unavailable("cue-unavailable", 0, 1, availability))
                .unwrap();
            buffer
                .deliver(
                    "cue-unavailable",
                    DeliveryStamp {
                        tick: 0,
                        event_sequence: 2,
                    },
                )
                .unwrap();
            assert_eq!(
                buffer.select_matured(0, 2).disposition,
                CueSelectionDisposition::NoObservedCue
            );
            assert!(buffer.select_matured(0, 2).selected.is_none());
        }
    }

    #[test]
    fn input_cutoff_hides_late_same_tick_delivery() {
        let mut buffer = ObservationDeliveryBuffer::new(config(2, 0, None, None)).unwrap();
        buffer.enqueue(observed("late", 1, 1, 7, 5)).unwrap();
        buffer
            .deliver(
                "late",
                DeliveryStamp {
                    tick: 1,
                    event_sequence: 9,
                },
            )
            .unwrap();

        assert_eq!(
            buffer.select_matured(1, 8).disposition,
            CueSelectionDisposition::NoDeliveredCue
        );
        assert_eq!(
            buffer.select_matured(1, 9).disposition,
            CueSelectionDisposition::Selected
        );
    }

    #[test]
    fn duplicate_ids_and_non_monotonic_events_reject() {
        let mut buffer = ObservationDeliveryBuffer::new(config(4, 0, None, None)).unwrap();
        buffer.enqueue(observed("cue-0", 0, 0, 2, 5)).unwrap();
        assert_eq!(
            buffer.enqueue(observed("cue-0", 1, 1, 3, 6)),
            Err(BufferError::DuplicateCueId)
        );
        assert_eq!(
            buffer.enqueue(observed("cue-1", 1, 1, 2, 7)),
            Err(BufferError::NonMonotonicGeneratedEvent)
        );
        buffer
            .deliver(
                "cue-0",
                DeliveryStamp {
                    tick: 0,
                    event_sequence: 4,
                },
            )
            .unwrap();
        assert_eq!(
            buffer.deliver(
                "cue-0",
                DeliveryStamp {
                    tick: 0,
                    event_sequence: 5,
                },
            ),
            Err(BufferError::DuplicateDelivery)
        );
        buffer.enqueue(observed("cue-1", 1, 1, 3, 7)).unwrap();
        assert_eq!(
            buffer.deliver(
                "cue-1",
                DeliveryStamp {
                    tick: 1,
                    event_sequence: 3,
                },
            ),
            Err(BufferError::DeliveryEventNotAfterGeneration)
        );
    }

    #[test]
    fn capacity_overflow_does_not_evict_or_mutate_existing_cues() {
        let mut buffer = ObservationDeliveryBuffer::new(config(1, 0, None, None)).unwrap();
        buffer.enqueue(observed("cue-0", 0, 0, 1, 5)).unwrap();
        assert_eq!(
            buffer.enqueue(observed("cue-1", 1, 1, 2, 6)),
            Err(BufferError::CapacityExceeded)
        );
        assert_eq!(buffer.len(), 1);
        assert_eq!(buffer.entries()[0].cue_id(), "cue-0");
    }

    #[test]
    fn delivery_before_generation_rejects() {
        let mut buffer = ObservationDeliveryBuffer::new(config(4, 0, None, None)).unwrap();
        buffer.enqueue(observed("cue-0", 1, 2, 1, 5)).unwrap();
        assert_eq!(
            buffer.deliver(
                "cue-0",
                DeliveryStamp {
                    tick: 1,
                    event_sequence: 2,
                }
            ),
            Err(BufferError::DeliveryBeforeGeneration)
        );
    }

    #[test]
    fn delivery_tick_regression_rejects_even_with_later_event_ordinal() {
        let mut buffer = ObservationDeliveryBuffer::new(config(4, 0, None, None)).unwrap();
        buffer.enqueue(observed("cue-0", 0, 0, 1, 5)).unwrap();
        buffer.enqueue(observed("cue-1", 1, 1, 3, 6)).unwrap();
        buffer
            .deliver(
                "cue-0",
                DeliveryStamp {
                    tick: 2,
                    event_sequence: 4,
                },
            )
            .unwrap();
        assert_eq!(
            buffer.deliver(
                "cue-1",
                DeliveryStamp {
                    tick: 1,
                    event_sequence: 5,
                }
            ),
            Err(BufferError::NonMonotonicDeliveryTick)
        );
    }

    #[test]
    fn zero_capacity_rejects() {
        assert_eq!(
            ObservationDeliveryBuffer::<i32>::new(config(0, 0, None, None)),
            Err(BufferError::ZeroCapacity)
        );
    }

    #[test]
    fn selection_reports_missing_delivery_and_no_observed_cue_separately() {
        let empty = ObservationDeliveryBuffer::<i32>::new(config(2, 0, None, None)).unwrap();
        assert_eq!(
            empty.select_matured(0, 10).disposition,
            CueSelectionDisposition::NoDeliveredCue
        );

        let mut unavailable_buffer =
            ObservationDeliveryBuffer::new(config(2, 0, None, None)).unwrap();
        unavailable_buffer
            .enqueue(unavailable(
                "missing",
                0,
                1,
                ObservationAvailability::Missing,
            ))
            .unwrap();
        unavailable_buffer
            .deliver(
                "missing",
                DeliveryStamp {
                    tick: 0,
                    event_sequence: 2,
                },
            )
            .unwrap();
        assert_eq!(
            unavailable_buffer.select_matured(0, 2).disposition,
            CueSelectionDisposition::NoObservedCue
        );
    }
}
