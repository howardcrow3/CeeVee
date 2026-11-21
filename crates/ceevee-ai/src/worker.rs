//! One thread owns the model and runs requests from a priority queue
//! (TECHNICAL_PLAN section 3, "Model worker"): interactive work such as chat and
//! drafting jumps ahead of background work such as scoring and mail labels. The
//! model is unloaded after a period with nothing to do.

use std::collections::BinaryHeap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::time::Duration;

use ceevee_core::{Error, Result};
use tokio::sync::{mpsc, oneshot};

use crate::backend::{GenerateRequest, LlmBackend};

/// Higher runs first. Equal priorities run in submission order.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Priority {
    Background = 0,
    Interactive = 1,
}

struct Queued {
    priority: Priority,
    seq: u64,
    req: GenerateRequest,
    cancel: Arc<AtomicBool>,
    stream: Option<mpsc::UnboundedSender<String>>,
    reply: oneshot::Sender<Result<String>>,
}

impl PartialEq for Queued {
    fn eq(&self, other: &Self) -> bool {
        self.priority == other.priority && self.seq == other.seq
    }
}
impl Eq for Queued {}
impl PartialOrd for Queued {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}
impl Ord for Queued {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // BinaryHeap pops the greatest: higher priority first, then the lower sequence number.
        self.priority
            .cmp(&other.priority)
            .then(other.seq.cmp(&self.seq))
    }
}

#[derive(Default)]
struct State {
    queue: BinaryHeap<Queued>,
    next_seq: u64,
    shutdown: bool,
}

struct Shared {
    state: Mutex<State>,
    wake: Condvar,
}

/// Cancels a submitted request. Cancelling a queued request answers it with
/// `Error::Cancelled` without running it; cancelling a running one stops it between tokens.
#[derive(Clone)]
pub struct CancelHandle(Arc<AtomicBool>);

impl CancelHandle {
    pub fn cancel(&self) {
        self.0.store(true, Ordering::Relaxed);
    }
}

pub struct Job {
    /// Resolves with the full text, or the error.
    pub result: oneshot::Receiver<Result<String>>,
    pub cancel: CancelHandle,
}

/// Handle to the worker thread. Cloning shares the same worker; the thread stops when the last
/// handle is dropped.
#[derive(Clone)]
pub struct ModelWorker {
    shared: Arc<Shared>,
    _guard: Arc<Guard>,
}

struct Guard(Arc<Shared>);
impl Drop for Guard {
    fn drop(&mut self) {
        self.0.state.lock().expect("worker state").shutdown = true;
        self.0.wake.notify_all();
    }
}

impl ModelWorker {
    /// Start the worker thread. `idle_unload` is how long the model stays in memory with no work.
    pub fn spawn(backend: impl LlmBackend, idle_unload: Duration) -> Self {
        let shared = Arc::new(Shared {
            state: Mutex::new(State::default()),
            wake: Condvar::new(),
        });
        let thread_shared = shared.clone();
        std::thread::Builder::new()
            .name("ceevee-model-worker".into())
            .spawn(move || run(backend, thread_shared, idle_unload))
            .expect("spawn model worker");
        Self {
            _guard: Arc::new(Guard(shared.clone())),
            shared,
        }
    }

    /// Queue a request. If `stream` is given, each token piece is sent to it as it is produced.
    pub fn submit(
        &self,
        priority: Priority,
        req: GenerateRequest,
        stream: Option<mpsc::UnboundedSender<String>>,
    ) -> Job {
        let (reply, result) = oneshot::channel();
        let cancel = Arc::new(AtomicBool::new(false));
        {
            let mut st = self.shared.state.lock().expect("worker state");
            let seq = st.next_seq;
            st.next_seq += 1;
            st.queue.push(Queued {
                priority,
                seq,
                req,
                cancel: cancel.clone(),
                stream,
                reply,
            });
        }
        self.shared.wake.notify_one();
        Job {
            result,
            cancel: CancelHandle(cancel),
        }
    }
}

fn run(mut backend: impl LlmBackend, shared: Arc<Shared>, idle_unload: Duration) {
    loop {
        let next = {
            let mut st = shared.state.lock().expect("worker state");
            loop {
                if st.shutdown {
                    backend.unload();
                    return;
                }
                if let Some(job) = st.queue.pop() {
                    break Some(job);
                }
                if !backend.is_loaded() {
                    st = shared.wake.wait(st).expect("worker state");
                    continue;
                }
                let (guard, timeout) = shared
                    .wake
                    .wait_timeout(st, idle_unload)
                    .expect("worker state");
                st = guard;
                if timeout.timed_out() && st.queue.is_empty() {
                    break None; // idle: unload outside the lock
                }
            }
        };
        let Some(job) = next else {
            backend.unload();
            continue;
        };
        if job.cancel.load(Ordering::Relaxed) {
            let _ = job.reply.send(Err(Error::Cancelled));
            continue;
        }
        let outcome = if backend.is_loaded() {
            Ok(())
        } else {
            backend.load()
        }
        .and_then(|()| {
            let stream = job.stream;
            backend.generate(&job.req, &job.cancel, &mut |piece| {
                if let Some(tx) = &stream {
                    let _ = tx.send(piece.to_owned());
                }
            })
        });
        let _ = job.reply.send(outcome);
    }
}

#[cfg(test)]
mod tests;
