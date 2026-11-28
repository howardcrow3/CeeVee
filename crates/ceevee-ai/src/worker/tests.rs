use std::sync::atomic::AtomicUsize;
use std::sync::mpsc as std_mpsc;
use std::time::Instant;

use super::*;

/// Fake engine: echoes the prompt, logs the order of requests, and can block the first request.
struct Fake {
    loaded: bool,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
    order: Arc<Mutex<Vec<String>>>,
    gate: Option<std_mpsc::Receiver<()>>,
    started: std_mpsc::Sender<()>,
}

impl LlmBackend for Fake {
    fn load(&mut self) -> Result<()> {
        self.loaded = true;
        self.loads.fetch_add(1, Ordering::SeqCst);
        Ok(())
    }
    fn unload(&mut self) {
        if self.loaded {
            self.unloads.fetch_add(1, Ordering::SeqCst);
        }
        self.loaded = false;
    }
    fn is_loaded(&self) -> bool {
        self.loaded
    }
    fn generate(
        &mut self,
        req: &GenerateRequest,
        cancel: &AtomicBool,
        on_token: &mut dyn FnMut(&str),
    ) -> Result<String> {
        let _ = self.started.send(());
        if let Some(gate) = self.gate.take() {
            let _ = gate.recv(); // hold the worker busy until the test releases it
        }
        self.order.lock().unwrap().push(req.prompt.clone());
        for word in req.prompt.split(' ') {
            if cancel.load(Ordering::Relaxed) {
                return Err(Error::Cancelled);
            }
            on_token(word);
        }
        Ok(req.prompt.clone())
    }
}

struct Rig {
    worker: ModelWorker,
    loads: Arc<AtomicUsize>,
    unloads: Arc<AtomicUsize>,
    order: Arc<Mutex<Vec<String>>>,
    release: Option<std_mpsc::Sender<()>>,
    started: std_mpsc::Receiver<()>,
}

fn rig(idle: Duration, gated: bool) -> Rig {
    let loads = Arc::new(AtomicUsize::new(0));
    let unloads = Arc::new(AtomicUsize::new(0));
    let order: Arc<Mutex<Vec<String>>> = Arc::default();
    let (tx, rx) = std_mpsc::channel();
    let (started_tx, started) = std_mpsc::channel();
    let fake = Fake {
        loaded: false,
        loads: loads.clone(),
        unloads: unloads.clone(),
        order: order.clone(),
        gate: gated.then_some(rx),
        started: started_tx,
    };
    Rig {
        worker: ModelWorker::spawn(fake, idle),
        loads,
        unloads,
        order,
        release: gated.then_some(tx),
        started,
    }
}

fn req(s: &str) -> GenerateRequest {
    GenerateRequest::new(s)
}

#[tokio::test]
async fn interactive_jumps_ahead_of_background_and_ties_keep_order() {
    let r = rig(Duration::from_secs(60), true);
    let w = &r.worker;
    let first = w.submit(Priority::Background, req("first"), None); // starts, then blocks on the gate
    r.started.recv().unwrap(); // the worker is now busy with the first request
    let b1 = w.submit(Priority::Background, req("bg1"), None);
    let b2 = w.submit(Priority::Background, req("bg2"), None);
    let i1 = w.submit(Priority::Interactive, req("chat1"), None);
    let i2 = w.submit(Priority::Interactive, req("chat2"), None);
    r.release.as_ref().unwrap().send(()).unwrap();
    for j in [first, b1, b2, i1, i2] {
        j.result.await.unwrap().unwrap();
    }
    assert_eq!(
        *r.order.lock().unwrap(),
        vec!["first", "chat1", "chat2", "bg1", "bg2"]
    );
}

#[tokio::test]
async fn streams_tokens_and_returns_the_full_text() {
    let r = rig(Duration::from_secs(60), false);
    let (tx, mut rx) = mpsc::unbounded_channel();
    let job = r
        .worker
        .submit(Priority::Interactive, req("a b c"), Some(tx));
    assert_eq!(job.result.await.unwrap().unwrap(), "a b c");
    let mut pieces = vec![];
    while let Some(p) = rx.recv().await {
        pieces.push(p);
    }
    assert_eq!(pieces, vec!["a", "b", "c"]);
}

#[tokio::test]
async fn a_cancelled_queued_job_never_runs() {
    let r = rig(Duration::from_secs(60), true);
    let blocker = r.worker.submit(Priority::Background, req("blocker"), None);
    r.started.recv().unwrap(); // the worker is now busy with the first request
    let doomed = r.worker.submit(Priority::Interactive, req("doomed"), None);
    doomed.cancel.cancel();
    r.release.as_ref().unwrap().send(()).unwrap();
    blocker.result.await.unwrap().unwrap();
    assert!(matches!(
        doomed.result.await.unwrap(),
        Err(Error::Cancelled)
    ));
    assert_eq!(*r.order.lock().unwrap(), vec!["blocker"]);
}

#[tokio::test]
async fn loads_lazily_unloads_when_idle_and_reloads() {
    let r = rig(Duration::from_millis(80), false);
    assert_eq!(
        r.loads.load(Ordering::SeqCst),
        0,
        "nothing loaded before the first request"
    );
    r.worker
        .submit(Priority::Interactive, req("one"), None)
        .result
        .await
        .unwrap()
        .unwrap();
    assert_eq!(r.loads.load(Ordering::SeqCst), 1);

    let start = Instant::now();
    while r.unloads.load(Ordering::SeqCst) == 0 && start.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(
        r.unloads.load(Ordering::SeqCst),
        1,
        "model unloaded after idle time"
    );

    r.worker
        .submit(Priority::Interactive, req("two"), None)
        .result
        .await
        .unwrap()
        .unwrap();
    assert_eq!(
        r.loads.load(Ordering::SeqCst),
        2,
        "reloaded for the next request"
    );
}

#[tokio::test]
async fn dropping_the_last_handle_unloads_and_stops_the_thread() {
    let r = rig(Duration::from_secs(60), false);
    r.worker
        .submit(Priority::Interactive, req("x"), None)
        .result
        .await
        .unwrap()
        .unwrap();
    let unloads = r.unloads.clone();
    drop(r);
    let start = Instant::now();
    while unloads.load(Ordering::SeqCst) == 0 && start.elapsed() < Duration::from_secs(3) {
        std::thread::sleep(Duration::from_millis(20));
    }
    assert_eq!(unloads.load(Ordering::SeqCst), 1);
}
