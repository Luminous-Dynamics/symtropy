// Copyright (c) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Subprocess IPC layer.
//!
//! We spawn `mycelix-conductor-bridge` as a child process and exchange JSON
//! messages over its stdin / stdout. See module-level docs on [`crate`] for
//! the "why subprocess, not in-process" rationale.
//!
//! ## Protocol (Milestone 2)
//!
//! Every request carries a `request_id` (u64, minted monotonically by the
//! writer loop). Responses echo the same `request_id` so the reader loop can
//! correlate them even if the conductor reorders execution.
//!
//! **Request** (one JSON per line, written to subprocess stdin):
//! ```json
//! {"request_id": 0, "type": "QueryActiveProposals"}
//! {"request_id": 1, "type": "SubmitProposal", "id": "MIP-042", "title": "...",
//!  "description": "...", "author": "did:key:z6Mk..."}
//! {"request_id": 2, "type": "CastVote", "proposal_id": "MIP-042",
//!  "voter_did": "did:key:z6Mk...", "approve": true, "rationale": ""}
//! {"request_id": 3, "type": "QueryTendBalance", "member_did": "did:key:z6Mk..."}
//! ```
//!
//! **Response** (one JSON per line, read from subprocess stdout):
//! ```json
//! {"request_id": 0, "ok": true, "data": [...]}
//! {"request_id": 1, "ok": false, "error": "..."}
//! ```

use std::collections::HashMap;
use std::process::Stdio;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use bevy::prelude::*;
use bevy_tokio_tasks::TokioTasksRuntime;
use flume::{Receiver, Sender};
use serde::{Deserialize, Serialize};
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::process::{ChildStdin, Command};
use tokio::sync::{Mutex, OwnedSemaphorePermit, Semaphore};

use crate::config::MycelixConfig;
use crate::events::{MycelixRequest, MycelixResponse};
use crate::resource::{MycelixRequestOutbox, MycelixResponseInbox};

// ---------------------------------------------------------------------------
// Wire protocol types
// ---------------------------------------------------------------------------

#[derive(Debug, Serialize)]
struct WireRequest {
    request_id: u64,
    #[serde(flatten)]
    command: WireCommand,
}

#[derive(Debug, Serialize)]
#[serde(tag = "type")]
enum WireCommand {
    QueryActiveProposals,
    SubmitProposal(WireProposalInput),
    CastVote(WireVoteInput),
    QueryTendBalance { member_did: String },
    GetProposal { proposal_id: String },
}

#[derive(Debug, Serialize)]
struct WireProposalInput {
    id: String,
    title: String,
    description: String,
    author: String,
}

#[derive(Debug, Serialize)]
struct WireVoteInput {
    proposal_id: String,
    voter_did: String,
    approve: bool,
    rationale: String,
}

#[derive(Debug, Deserialize)]
struct WireResponse {
    #[serde(default)]
    request_id: Option<u64>,
    ok: bool,
    #[serde(default)]
    data: Option<serde_json::Value>,
    #[serde(default)]
    error: Option<String>,
}

// ---------------------------------------------------------------------------
// Correlation metadata
// ---------------------------------------------------------------------------

#[derive(Debug)]
enum PendingKind {
    GetActiveProposals,
    ProposalSubmitted { proposal_id: String },
    VoteCast { proposal_id: String },
    TendBalance { member_did: String },
    Proposal { proposal_id: String },
}

struct Pending {
    requester: Entity,
    kind: PendingKind,
    // Keeps the in-flight permit alive until the correlated response or a
    // fail-closed teardown removes this request from the pending map.
    _permit: OwnedSemaphorePermit,
}

type PendingMap = Arc<Mutex<HashMap<u64, Pending>>>;

// ---------------------------------------------------------------------------
// Systems
// ---------------------------------------------------------------------------

/// Startup system: spawn the subprocess + background IPC task.
pub(crate) fn spawn_dispatcher_task(
    runtime: Res<TokioTasksRuntime>,
    config: Res<MycelixConfig>,
    mut outbox: ResMut<MycelixRequestOutbox>,
) {
    let Some(req_rx) = outbox.rx.take() else {
        warn!("symtropy-mycelix-bridge: dispatcher task already started; skipping");
        return;
    };
    let Some(resp_tx) = outbox.response_tx.take() else {
        warn!("symtropy-mycelix-bridge: response sender missing; skipping");
        return;
    };

    let config = config.clone();

    runtime.spawn_background_task(move |_ctx| async move {
        if let Err(err) = run_dispatcher_loop(config, req_rx, resp_tx).await {
            error!(
                ?err,
                "symtropy-mycelix-bridge: dispatcher loop exited with error"
            );
        }
    });
}

/// Update-schedule system: drain the inbox into Bevy [`MycelixResponse`]
/// messages.
pub(crate) fn pump_responses(
    inbox: Res<MycelixResponseInbox>,
    mut writer: MessageWriter<MycelixResponse>,
) {
    for response in inbox.rx.try_iter() {
        writer.write(response);
    }
}

// ---------------------------------------------------------------------------
// Dispatcher loop (runs inside the tokio runtime)
// ---------------------------------------------------------------------------

async fn run_dispatcher_loop(
    config: MycelixConfig,
    req_rx: Receiver<MycelixRequest>,
    resp_tx: Sender<MycelixResponse>,
) -> Result<(), DispatcherError> {
    info!(
        binary = %config.bridge_binary.display(),
        conductor_url = %config.conductor_url,
        app_id = %config.app_id,
        role = %config.role,
        "symtropy-mycelix-bridge: spawning subprocess"
    );

    let mut child = match Command::new(&config.bridge_binary)
        .arg("--conductor-url")
        .arg(&config.conductor_url)
        .arg("--app-id")
        .arg(&config.app_id)
        .arg("--role")
        .arg(&config.role)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .kill_on_drop(true)
        .spawn()
    {
        Ok(c) => c,
        Err(e) => {
            let reason = format!("spawn {:?}: {e}", config.bridge_binary);
            error!(%reason, "symtropy-mycelix-bridge: spawn failed");
            drain_with_error(req_rx, resp_tx, reason).await;
            return Err(DispatcherError::Spawn {
                path: config.bridge_binary.to_string_lossy().to_string(),
                source: e,
            });
        }
    };

    let stdin = child.stdin.take().ok_or(DispatcherError::MissingStdin)?;
    let stdout = child.stdout.take().ok_or(DispatcherError::MissingStdout)?;
    let mut stderr = child.stderr.take().ok_or(DispatcherError::MissingStderr)?;

    // A piped stderr that nobody reads can fill its OS buffer and stall the
    // child. Drain it without logging raw output, which may contain secrets.
    let _stderr_task = tokio::spawn(async move {
        let mut sink = tokio::io::sink();
        if let Err(err) = tokio::io::copy(&mut stderr, &mut sink).await {
            debug!(?err, "bridge stderr pipe closed with an I/O error");
        }
    });

    let pending: PendingMap = Arc::new(Mutex::new(HashMap::new()));
    let next_id = Arc::new(AtomicU64::new(0));
    // The request channel bounds queued work; this semaphore separately bounds
    // sent requests awaiting a correlated reply. Treat a configured zero as one
    // so a bad setting cannot deadlock all dispatch.
    let inflight = Arc::new(Semaphore::new(config.effective_inflight_budget()));

    // Keep supervisor-owned receiver/sender clones so either task's exit can
    // fail queued and pending callers, including a request racing with stdout EOF.
    let supervisor_req_rx = req_rx.clone();
    let supervisor_resp_tx = resp_tx.clone();

    let mut writer_task = {
        let pending = pending.clone();
        let next_id = next_id.clone();
        let inflight = inflight.clone();
        let response_tx = resp_tx.clone();
        tokio::spawn(async move {
            writer_loop(stdin, req_rx, pending, next_id, inflight, response_tx).await
        })
    };
    let mut reader_task = {
        let pending = pending.clone();
        tokio::spawn(async move { reader_loop(stdout, resp_tx, pending).await })
    };

    // If the request channel closes, the writer closes child stdin; then allow
    // the reader to drain final replies before accepting shutdown. Conversely,
    // any reader exit while the writer is still alive is treated as a failure;
    // all pending and queued requests are completed with an error before return.
    tokio::select! {
        biased;
        writer_result = &mut writer_task => {
            match writer_result {
                Err(join) => {
                    let failure = DispatcherError::Join(join);
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    reader_task.abort();
                    let _ = reader_task.await;
                    let reason = failure.to_string();
                    fail_pending_and_queued(
                        &pending,
                        supervisor_req_rx.clone(),
                        supervisor_resp_tx.clone(),
                        &reason,
                    ).await;
                    Err(failure)
                }
                Ok(Err(failure)) => {
                    let _ = child.start_kill();
                    let _ = child.wait().await;
                    reader_task.abort();
                    let _ = reader_task.await;
                    let reason = failure.to_string();
                    fail_pending_and_queued(
                        &pending,
                        supervisor_req_rx.clone(),
                        supervisor_resp_tx.clone(),
                        &reason,
                    ).await;
                    Err(failure)
                }
                Ok(Ok(())) => {
                    match reader_task.await {
                        Ok(Ok(())) => Ok(()),
                        Ok(Err(failure)) => {
                            let _ = child.start_kill();
                            let _ = child.wait().await;
                            let reason = failure.to_string();
                            fail_pending_and_queued(
                                &pending,
                                supervisor_req_rx.clone(),
                                supervisor_resp_tx.clone(),
                                &reason,
                            ).await;
                            Err(failure)
                        }
                        Err(join) => {
                            let failure = DispatcherError::Join(join);
                            let _ = child.start_kill();
                            let _ = child.wait().await;
                            let reason = failure.to_string();
                            fail_pending_and_queued(
                                &pending,
                                supervisor_req_rx.clone(),
                                supervisor_resp_tx.clone(),
                                &reason,
                            ).await;
                            Err(failure)
                        }
                    }
                }
            }
        }
        reader_result = &mut reader_task => {
            // Stop the writer before draining the shared request queue; otherwise
            // it could consume a queued mutation while blocked awaiting a reply
            // from the reader that has already exited.
            writer_task.abort();
            let _ = writer_task.await;
            let _ = child.start_kill();
            let _ = child.wait().await;
            let failure = match reader_result {
                Ok(Ok(())) => DispatcherError::UnexpectedBridgeExit,
                Ok(Err(failure)) => failure,
                Err(join) => DispatcherError::Join(join),
            };
            let reason = failure.to_string();
            fail_pending_and_queued(
                &pending,
                supervisor_req_rx,
                supervisor_resp_tx,
                &reason,
            ).await;
            Err(failure)
        }
    }
}

async fn drain_with_error(
    req_rx: Receiver<MycelixRequest>,
    resp_tx: Sender<MycelixResponse>,
    reason: String,
) {
    while let Ok(req) = req_rx.recv_async().await {
        let requester = req.requester();
        if resp_tx
            .send_async(MycelixResponse::Error {
                requester,
                reason: reason.clone(),
            })
            .await
            .is_err()
        {
            return;
        }
    }
}

async fn writer_loop<W>(
    mut stdin: W,
    req_rx: Receiver<MycelixRequest>,
    pending: PendingMap,
    next_id: Arc<AtomicU64>,
    inflight: Arc<Semaphore>,
    resp_tx: Sender<MycelixResponse>,
) -> Result<(), DispatcherError>
where
    W: tokio::io::AsyncWrite + Unpin + Send + 'static,
{
    while let Ok(req) = req_rx.recv_async().await {
        let requester = req.requester();
        let permit = match inflight.clone().acquire_owned().await {
            Ok(permit) => permit,
            Err(_) => {
                let reason = "in-flight request limiter closed unexpectedly".to_string();
                let _ = resp_tx
                    .send_async(MycelixResponse::Error {
                        requester,
                        reason: reason.clone(),
                    })
                    .await;
                drop(stdin);
                fail_all_pending(&pending, &resp_tx, &reason).await;
                drain_with_error(req_rx, resp_tx, reason).await;
                return Err(DispatcherError::InFlightClosed);
            }
        };

        let request_id = next_id.fetch_add(1, Ordering::SeqCst);
        if request_id == u64::MAX {
            let reason = "bridge request_id space exhausted; refusing identifier reuse".to_string();
            let _ = resp_tx
                .send_async(MycelixResponse::Error {
                    requester,
                    reason: reason.clone(),
                })
                .await;
            drop(stdin);
            fail_all_pending(&pending, &resp_tx, &reason).await;
            drain_with_error(req_rx, resp_tx, reason).await;
            return Err(DispatcherError::RequestIdExhausted);
        }

        let (kind, command) = match req {
            MycelixRequest::GetActiveProposals { .. } => (
                PendingKind::GetActiveProposals,
                WireCommand::QueryActiveProposals,
            ),
            MycelixRequest::SubmitProposal {
                proposal_id,
                title,
                description,
                author_did,
                ..
            } => (
                PendingKind::ProposalSubmitted {
                    proposal_id: proposal_id.clone(),
                },
                WireCommand::SubmitProposal(WireProposalInput {
                    id: proposal_id,
                    title,
                    description,
                    author: author_did,
                }),
            ),
            MycelixRequest::CastVote {
                proposal_id,
                voter_did,
                approve,
                rationale,
                ..
            } => (
                PendingKind::VoteCast {
                    proposal_id: proposal_id.clone(),
                },
                WireCommand::CastVote(WireVoteInput {
                    proposal_id,
                    voter_did,
                    approve,
                    rationale,
                }),
            ),
            MycelixRequest::QueryTendBalance { member_did, .. } => (
                PendingKind::TendBalance {
                    member_did: member_did.clone(),
                },
                WireCommand::QueryTendBalance { member_did },
            ),
            MycelixRequest::GetProposal { proposal_id, .. } => (
                PendingKind::Proposal {
                    proposal_id: proposal_id.clone(),
                },
                WireCommand::GetProposal { proposal_id },
            ),
        };

        {
            let mut p = pending.lock().await;
            p.insert(
                request_id,
                Pending {
                    requester,
                    kind,
                    _permit: permit,
                },
            );
        }

        let wire = WireRequest {
            request_id,
            command,
        };
        let mut line = match serde_json::to_string(&wire) {
            Ok(line) => line,
            Err(err) => {
                let reason = format!("failed to serialise bridge request: {err}");
                drop(stdin);
                fail_all_pending(&pending, &resp_tx, &reason).await;
                drain_with_error(req_rx, resp_tx, reason).await;
                return Err(DispatcherError::Serialise(err));
            }
        };
        line.push('\n');

        if let Err(err) = stdin.write_all(line.as_bytes()).await {
            let reason = format!("bridge subprocess request write failed: {err}");
            drop(stdin);
            fail_all_pending(&pending, &resp_tx, &reason).await;
            drain_with_error(req_rx, resp_tx, reason).await;
            return Err(DispatcherError::Stdin(err));
        }
        if let Err(err) = stdin.flush().await {
            let reason = format!("bridge subprocess request flush failed: {err}");
            drop(stdin);
            fail_all_pending(&pending, &resp_tx, &reason).await;
            drain_with_error(req_rx, resp_tx, reason).await;
            return Err(DispatcherError::Stdin(err));
        }

        trace!(%request_id, %requester, "dispatched request to subprocess");
    }

    drop(stdin);
    Ok(())
}

async fn reader_loop<R>(
    stdout: R,
    resp_tx: Sender<MycelixResponse>,
    pending: PendingMap,
) -> Result<(), DispatcherError>
where
    R: tokio::io::AsyncRead + Unpin,
{
    let mut lines = BufReader::new(stdout).lines();

    loop {
        let line = match lines.next_line().await {
            Ok(Some(line)) => line,
            Ok(None) => {
                let outstanding = pending.lock().await.len();
                if outstanding == 0 {
                    return Ok(());
                }
                let reason = format!(
                    "bridge subprocess closed stdout with {outstanding} request(s) unresolved"
                );
                fail_all_pending(&pending, &resp_tx, &reason).await;
                return Err(DispatcherError::UnexpectedStdoutEof { outstanding });
            }
            Err(err) => {
                let reason = format!("failed to read bridge subprocess stdout: {err}");
                fail_all_pending(&pending, &resp_tx, &reason).await;
                return Err(DispatcherError::Stdout(err));
            }
        };

        let wire: WireResponse = match serde_json::from_str(&line) {
            Ok(wire) => wire,
            Err(err) => {
                // Do not log the raw line: it can contain personal or secret data.
                warn!(
                    ?err,
                    "invalid JSON from bridge subprocess; fencing pending requests"
                );
                fail_all_pending(
                    &pending,
                    &resp_tx,
                    "malformed JSON from bridge subprocess; request outcome is unknown",
                )
                .await;
                return Err(DispatcherError::MalformedResponseJson(err));
            }
        };

        let id = match wire.request_id {
            Some(id) => id,
            None => {
                fail_all_pending(
                    &pending,
                    &resp_tx,
                    "bridge response omitted request_id; request outcome is unknown",
                )
                .await;
                return Err(DispatcherError::MissingRequestId);
            }
        };

        let response = match translate(id, wire, &pending).await {
            Ok(response) => response,
            Err(err) => {
                let reason = err.to_string();
                fail_all_pending(&pending, &resp_tx, &reason).await;
                return Err(err);
            }
        };

        if resp_tx.send_async(response).await.is_err() {
            fail_all_pending(
                &pending,
                &resp_tx,
                "response consumer closed while bridge requests were in flight",
            )
            .await;
            info!("symtropy-mycelix-bridge: inbox closed; reader exiting");
            return Ok(());
        }
    }
}

async fn fail_all_pending(pending: &PendingMap, resp_tx: &Sender<MycelixResponse>, reason: &str) {
    let entries = {
        let mut guard = pending.lock().await;
        std::mem::take(&mut *guard)
    };
    for (_, entry) in entries {
        if resp_tx
            .send_async(MycelixResponse::Error {
                requester: entry.requester,
                reason: reason.to_string(),
            })
            .await
            .is_err()
        {
            return;
        }
    }
}

async fn fail_pending_and_queued(
    pending: &PendingMap,
    queued: Receiver<MycelixRequest>,
    resp_tx: Sender<MycelixResponse>,
    reason: &str,
) {
    fail_all_pending(pending, &resp_tx, reason).await;
    drain_with_error(queued, resp_tx, reason.to_string()).await;
}

async fn translate(
    id: u64,
    wire: WireResponse,
    pending: &PendingMap,
) -> Result<MycelixResponse, DispatcherError> {
    let pending_entry = { pending.lock().await.remove(&id) };
    let Some(Pending {
        requester,
        kind,
        _permit: _,
    }) = pending_entry
    else {
        return Err(DispatcherError::UnknownRequestId(id));
    };

    if !wire.ok {
        return Ok(MycelixResponse::Error {
            requester,
            reason: wire
                .error
                .unwrap_or_else(|| "bridge reported failure with no reason".to_string()),
        });
    }

    let invalid_success = |detail: &str| MycelixResponse::Error {
        requester,
        reason: format!("bridge returned an invalid successful response: {detail}"),
    };

    let response = match kind {
        PendingKind::GetActiveProposals => match wire.data {
            Some(serde_json::Value::Array(proposals)) => MycelixResponse::ActiveProposals {
                requester,
                proposals,
            },
            _ => invalid_success("QueryActiveProposals requires an array in data"),
        },
        PendingKind::ProposalSubmitted { proposal_id } => match wire.data {
            Some(serde_json::Value::String(action_hash)) if !action_hash.trim().is_empty() => {
                MycelixResponse::ProposalSubmitted {
                    requester,
                    proposal_id,
                    action_hash,
                }
            }
            _ => invalid_success("SubmitProposal requires a non-empty action hash"),
        },
        PendingKind::VoteCast { proposal_id } => match wire.data {
            Some(serde_json::Value::String(returned_id)) if returned_id == proposal_id => {
                MycelixResponse::VoteCast {
                    requester,
                    proposal_id,
                }
            }
            _ => invalid_success("CastVote requires the matching proposal ID in data"),
        },
        PendingKind::TendBalance { member_did } => match wire.data {
            Some(balance) => MycelixResponse::TendBalance {
                requester,
                member_did,
                balance,
            },
            None => invalid_success("QueryTendBalance omitted data"),
        },
        PendingKind::Proposal { proposal_id } => match wire.data {
            Some(serde_json::Value::Null) | None => MycelixResponse::Proposal {
                requester,
                proposal_id,
                record: None,
            },
            Some(record @ serde_json::Value::Object(_)) => MycelixResponse::Proposal {
                requester,
                proposal_id,
                record: Some(record),
            },
            Some(_) => invalid_success("GetProposal data must be null or a record object"),
        },
    };

    Ok(response)
}

#[derive(Debug, thiserror::Error)]
pub(crate) enum DispatcherError {
    #[error("failed to spawn bridge subprocess at {path:?}: {source}")]
    Spawn {
        path: String,
        #[source]
        source: std::io::Error,
    },
    #[error("subprocess stdin handle missing")]
    MissingStdin,
    #[error("subprocess stdout handle missing")]
    MissingStdout,
    #[error("subprocess stderr handle missing")]
    MissingStderr,
    #[error("bridge subprocess exited before its request channel was shut down")]
    UnexpectedBridgeExit,
    #[error("in-flight request limiter closed unexpectedly")]
    InFlightClosed,
    #[error("bridge request_id space exhausted")]
    RequestIdExhausted,
    #[error("bridge subprocess closed stdout with {outstanding} unresolved request(s)")]
    UnexpectedStdoutEof { outstanding: usize },
    #[error("bridge response JSON was malformed: {0}")]
    MalformedResponseJson(#[source] serde_json::Error),
    #[error("bridge response omitted request_id")]
    MissingRequestId,
    #[error("bridge response referenced unknown request_id {0}")]
    UnknownRequestId(u64),
    #[error("failed to write to subprocess stdin: {0}")]
    Stdin(#[source] std::io::Error),
    #[error("failed to read from subprocess stdout: {0}")]
    Stdout(#[source] std::io::Error),
    #[error("failed to serialise request: {0}")]
    Serialise(#[source] serde_json::Error),
    #[error("tokio task panicked: {0}")]
    Join(#[source] tokio::task::JoinError),
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;
    use tokio::io::{AsyncBufReadExt, AsyncWriteExt};

    async fn pending_one(id: u64, semaphore: &Arc<Semaphore>) -> PendingMap {
        let permit = semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("in-flight permit");
        let entry = Pending {
            requester: Entity::PLACEHOLDER,
            kind: PendingKind::ProposalSubmitted {
                proposal_id: "proposal-test".to_string(),
            },
            _permit: permit,
        };
        Arc::new(Mutex::new(HashMap::from([(id, entry)])))
    }

    #[test]
    fn malformed_response_fails_all_inflight_requests() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let (mut source, stdout) = tokio::io::duplex(1024);
                source
                    .write_all(b"{not-json\n")
                    .await
                    .expect("write malformed response");
                drop(source);

                let semaphore = Arc::new(Semaphore::new(1));
                let pending = pending_one(7, &semaphore).await;
                let (resp_tx, resp_rx) = flume::bounded(2);
                let result = reader_loop(stdout, resp_tx, pending.clone()).await;

                assert!(matches!(
                    result,
                    Err(DispatcherError::MalformedResponseJson(_))
                ));
                assert!(pending.lock().await.is_empty());
                assert_eq!(semaphore.available_permits(), 1);
                assert!(matches!(
                    resp_rx.recv_async().await.expect("failure response"),
                    MycelixResponse::Error { requester, .. } if requester == Entity::PLACEHOLDER
                ));
            });
    }

    #[test]
    fn stdout_eof_with_pending_request_is_not_reported_as_clean_shutdown() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let (source, stdout) = tokio::io::duplex(1024);
                drop(source);

                let semaphore = Arc::new(Semaphore::new(1));
                let pending = pending_one(8, &semaphore).await;
                let (resp_tx, resp_rx) = flume::bounded(2);
                let result = reader_loop(stdout, resp_tx, pending.clone()).await;

                assert!(matches!(
                    result,
                    Err(DispatcherError::UnexpectedStdoutEof { outstanding: 1 })
                ));
                assert!(pending.lock().await.is_empty());
                assert!(matches!(
                    resp_rx.recv_async().await.expect("failure response"),
                    MycelixResponse::Error { requester, .. } if requester == Entity::PLACEHOLDER
                ));
            });
    }

    #[test]
    fn unmatched_request_id_fences_all_remaining_requests() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let (mut source, stdout) = tokio::io::duplex(1024);
                source
                    .write_all(b"{\"request_id\":99,\"ok\":true,\"data\":\"unexpected\"}\n")
                    .await
                    .expect("write unmatched response");
                drop(source);

                let semaphore = Arc::new(Semaphore::new(1));
                let pending = pending_one(8, &semaphore).await;
                let (resp_tx, resp_rx) = flume::bounded(2);
                let result = reader_loop(stdout, resp_tx, pending.clone()).await;

                assert!(matches!(result, Err(DispatcherError::UnknownRequestId(99))));
                assert!(pending.lock().await.is_empty());
                assert!(matches!(
                    resp_rx.recv_async().await.expect("failure response"),
                    MycelixResponse::Error { requester, reason }
                        if requester == Entity::PLACEHOLDER && reason.contains("unknown request_id 99")
                ));
            });
    }

    #[test]
    fn supervisor_failure_completes_both_pending_and_queued_requests() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let semaphore = Arc::new(Semaphore::new(1));
                let pending = pending_one(17, &semaphore).await;
                let (request_tx, request_rx) = flume::bounded(2);
                request_tx
                    .send(MycelixRequest::QueryTendBalance {
                        requester: Entity::PLACEHOLDER,
                        member_did: "did:key:queued".to_string(),
                    })
                    .expect("queued request");
                let (resp_tx, resp_rx) = flume::bounded(3);

                let pending_for_drain = pending.clone();
                let drain_task = tokio::spawn(async move {
                    fail_pending_and_queued(
                        &pending_for_drain,
                        request_rx,
                        resp_tx,
                        "bridge exited unexpectedly",
                    )
                    .await;
                });

                let first = resp_rx.recv_async().await.expect("pending failure");
                let second = resp_rx.recv_async().await.expect("queued failure");
                assert!(matches!(first, MycelixResponse::Error { .. }));
                assert!(matches!(second, MycelixResponse::Error { .. }));
                assert!(resp_rx.is_empty());

                // A failed generation remains a rejection sink while callers
                // still own senders; it must never silently accept later work.
                request_tx
                    .send(MycelixRequest::GetActiveProposals {
                        requester: Entity::PLACEHOLDER,
                    })
                    .expect("failed bridge retains a rejection sink");
                assert!(matches!(
                    resp_rx.recv_async().await.expect("post-failure rejection"),
                    MycelixResponse::Error { .. }
                ));

                assert!(pending.lock().await.is_empty());
                assert_eq!(semaphore.available_permits(), 1);
                drop(request_tx);
                drain_task.await.expect("error drain task");
            });
    }

    #[test]
    fn dispatched_requests_are_bounded_by_inflight_budget() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let (request_tx, request_rx) = flume::bounded(2);
                request_tx
                    .send(MycelixRequest::GetActiveProposals {
                        requester: Entity::PLACEHOLDER,
                    })
                    .expect("first request");
                request_tx
                    .send(MycelixRequest::QueryTendBalance {
                        requester: Entity::PLACEHOLDER,
                        member_did: "did:key:test".to_string(),
                    })
                    .expect("second request");

                let (stdin, stdout) = tokio::io::duplex(4096);
                let mut stdout = BufReader::new(stdout);
                let (resp_tx, _resp_rx) = flume::bounded(2);
                let pending = Arc::new(Mutex::new(HashMap::new()));
                let semaphore = Arc::new(Semaphore::new(1));
                let writer = tokio::spawn(writer_loop(
                    stdin,
                    request_rx,
                    pending.clone(),
                    Arc::new(AtomicU64::new(0)),
                    semaphore,
                    resp_tx,
                ));

                let mut first_line = String::new();
                tokio::time::timeout(Duration::from_secs(1), stdout.read_line(&mut first_line))
                    .await
                    .expect("first request was written")
                    .expect("read first request");
                assert!(!first_line.is_empty());
                assert_eq!(pending.lock().await.len(), 1);

                let mut second_line = String::new();
                assert!(
                    tokio::time::timeout(
                        Duration::from_millis(50),
                        stdout.read_line(&mut second_line),
                    )
                    .await
                    .is_err(),
                    "the second request must wait for the first response to release its permit"
                );
                assert_eq!(pending.lock().await.len(), 1);

                writer.abort();
                fail_all_pending(&pending, &_resp_tx, "test cleanup").await;
            });
    }

    #[test]
    fn successful_vote_response_requires_the_matching_proposal_id() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let semaphore = Arc::new(Semaphore::new(1));
                let permit = semaphore
                    .clone()
                    .acquire_owned()
                    .await
                    .expect("in-flight permit");
                let pending = Arc::new(Mutex::new(HashMap::from([(
                    3,
                    Pending {
                        requester: Entity::PLACEHOLDER,
                        kind: PendingKind::VoteCast {
                            proposal_id: "P1".to_string(),
                        },
                        _permit: permit,
                    },
                )])));
                let response = translate(
                    3,
                    WireResponse {
                        request_id: Some(3),
                        ok: true,
                        data: Some(serde_json::json!("P2")),
                        error: None,
                    },
                    &pending,
                )
                .await
                .expect("invalid vote acknowledgement becomes a typed error response");

                assert!(matches!(
                    response,
                    MycelixResponse::Error { requester, reason }
                        if requester == Entity::PLACEHOLDER && reason.contains("matching proposal ID")
                ));
                assert!(pending.lock().await.is_empty());
                assert_eq!(semaphore.available_permits(), 1);
            });
    }

    #[test]
    fn successful_proposal_response_preserves_the_original_proposal_id() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let semaphore = Arc::new(Semaphore::new(1));
                let pending = pending_one(4, &semaphore).await;
                let response = translate(
                    4,
                    WireResponse {
                        request_id: Some(4),
                        ok: true,
                        data: Some(serde_json::json!("uhCkk_action_hash")),
                        error: None,
                    },
                    &pending,
                )
                .await
                .expect("valid proposal acknowledgement");

                assert!(matches!(
                    response,
                    MycelixResponse::ProposalSubmitted {
                        proposal_id,
                        action_hash,
                        ..
                    } if proposal_id == "proposal-test" && action_hash == "uhCkk_action_hash"
                ));
                assert!(pending.lock().await.is_empty());
                assert_eq!(semaphore.available_permits(), 1);
            });
    }

    #[test]
    fn successful_proposal_response_requires_a_nonempty_action_hash() {
        tokio::runtime::Runtime::new()
            .expect("runtime")
            .block_on(async {
                let semaphore = Arc::new(Semaphore::new(1));
                let pending = pending_one(3, &semaphore).await;
                let response = translate(
                    3,
                    WireResponse {
                        request_id: Some(3),
                        ok: true,
                        data: None,
                        error: None,
                    },
                    &pending,
                )
                .await
                .expect("invalid success becomes a typed error response");
                assert!(matches!(
                    response,
                    MycelixResponse::Error { requester, reason }
                        if requester == Entity::PLACEHOLDER && reason.contains("action hash")
                ));
                assert!(pending.lock().await.is_empty());
                assert_eq!(semaphore.available_permits(), 1);
            });
    }
}
