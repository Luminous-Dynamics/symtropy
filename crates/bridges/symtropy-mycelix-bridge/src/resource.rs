// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::sync::Arc;

use bevy::prelude::*;
use flume::{Receiver, Sender, TrySendError};
use tokio::sync::{OwnedSemaphorePermit, Semaphore, TryAcquireError};

use crate::events::{MycelixRequest, MycelixResponse};

/// Bevy `Resource` holding the sending end of the request channel.
///
/// Bevy systems obtain this via `Res<MycelixClient>` and call [`send`] to
/// dispatch zome calls to the tokio background task.
///
/// [`send`]: MycelixClient::send
/// An admitted request owns one credit until its response is transferred to
/// Bevy's message queue. This makes the budget end-to-end: queued requests,
/// in-flight calls, and undelivered responses all count against the same cap.
pub(crate) struct QueuedRequest {
    pub(crate) request: MycelixRequest,
    pub(crate) _permit: OwnedSemaphorePermit,
}

/// A response retains its request's admission credit while waiting in the
/// bounded inbox. The permit is released only after Bevy accepts the response.
pub(crate) struct MycelixResponseDelivery {
    pub(crate) response: MycelixResponse,
    pub(crate) _permit: OwnedSemaphorePermit,
}

impl MycelixResponseDelivery {
    /// Extract a response for tests or adapters that explicitly consume it.
    /// Production pumping writes the message before dropping the permit.
    pub(crate) fn into_response(self) -> MycelixResponse {
        let Self { response, _permit } = self;
        drop(_permit);
        response
    }
}

#[derive(Resource, Clone)]
pub struct MycelixClient {
    tx: Sender<QueuedRequest>,
    admission: Arc<Semaphore>,
}

impl MycelixClient {
    pub(crate) fn new(tx: Sender<QueuedRequest>, admission: Arc<Semaphore>) -> Self {
        Self { tx, admission }
    }

    /// Enqueue a zome call without blocking the Bevy schedule.
    ///
    /// Admission credits bound all accepted work end-to-end: queued requests,
    /// dispatched requests, and responses not yet transferred to Bevy. Returns
    /// [`MycelixSendError::Full`] if the global budget is exhausted.
    pub fn send(&self, request: MycelixRequest) -> Result<(), MycelixSendError> {
        let permit = match self.admission.clone().try_acquire_owned() {
            Ok(permit) => permit,
            Err(TryAcquireError::NoPermits) => return Err(MycelixSendError::Full),
            Err(TryAcquireError::Closed) => return Err(MycelixSendError::Disconnected),
        };

        let queued = QueuedRequest {
            request,
            _permit: permit,
        };
        match self.tx.try_send(queued) {
            Ok(()) => Ok(()),
            Err(TrySendError::Full(_queued)) => Err(MycelixSendError::Full),
            Err(TrySendError::Disconnected(_queued)) => Err(MycelixSendError::Disconnected),
        }
    }
}

/// Reasons [`MycelixClient::send`] can fail.
#[derive(Debug, thiserror::Error)]
pub enum MycelixSendError {
    #[error("request channel is full (inflight budget reached)")]
    Full,
    #[error("request channel is disconnected — background task exited")]
    Disconnected,
}

/// Internal resource holding the receiving end of the response channel.
///
/// Pumped each frame by [`pump_responses`] into the Bevy event stream.
///
/// [`pump_responses`]: crate::systems::pump_responses
#[derive(Resource)]
pub(crate) struct MycelixResponseInbox {
    pub(crate) rx: Receiver<MycelixResponseDelivery>,
}

/// Internal resource holding the receiving end of the request channel so the
/// tokio background task can claim it at startup.
#[derive(Resource)]
pub(crate) struct MycelixRequestOutbox {
    pub(crate) rx: Option<Receiver<QueuedRequest>>,
    pub(crate) response_tx: Option<Sender<MycelixResponseDelivery>>,
}
