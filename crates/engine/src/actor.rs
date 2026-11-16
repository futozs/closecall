//! Each provider lives on its own thread and is driven over a channel, so a provider may block
//! until its deadline (or keep thread-affine state) without stalling the engine.

use std::io;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use closecall_core::platform::{
    EventSender, ExecContext, InventoryEvent, Provider, ProviderError, Reverify,
};
use closecall_core::{Action, ProviderId, Step, StepOutcome, Target};
use crossbeam_channel::{Receiver, Sender, bounded, unbounded};

use crate::runtime::Msg;

const SCAN_BUDGET: Duration = Duration::from_secs(5);

pub(crate) enum Command {
    Refresh,
    Reverify {
        target: Target,
        deadline: Instant,
        reply: Sender<Reverify>,
    },
    Execute {
        step: Step,
        target: Target,
        ctx: ExecContext,
        deadline: Instant,
        reply: Sender<StepOutcome>,
    },
    Restore {
        target: Target,
        action: Action,
        deadline: Instant,
        reply: Sender<StepOutcome>,
    },
    Flush(Sender<()>),
}

pub(crate) struct ProviderHandle {
    pub id: ProviderId,
    tx: Sender<Command>,
    /// Commands sent and not yet finished, the start-up scan included.
    inflight: Arc<AtomicUsize>,
    timed_out: AtomicBool,
}

impl ProviderHandle {
    pub fn spawn(
        index: usize,
        provider: Box<dyn Provider>,
        events: Sender<Msg>,
    ) -> io::Result<Self> {
        let id = provider.id();
        let (tx, rx) = unbounded();
        let inflight = Arc::new(AtomicUsize::new(1));
        let counter = Arc::clone(&inflight);
        thread::Builder::new()
            .name(format!("closecall-{}", id.0))
            .spawn(move || run(index, provider, &rx, &events, &counter))?;
        Ok(Self {
            id,
            tx,
            inflight,
            timed_out: AtomicBool::new(false),
        })
    }

    pub fn refresh(&self) {
        self.send(Command::Refresh);
    }

    /// `None` when the provider did not answer by `wait_until`. After one missed answer, later
    /// requests fail at once until the provider has worked through its queue, so one hung app
    /// costs a run one deadline, not one per step.
    pub fn request<T>(
        &self,
        make: impl FnOnce(Sender<T>) -> Command,
        wait_until: Instant,
    ) -> Option<T> {
        if self.timed_out.load(Ordering::SeqCst) {
            if self.inflight.load(Ordering::SeqCst) > 0 {
                return None;
            }
            self.timed_out.store(false, Ordering::SeqCst);
        }
        let (reply, answer) = bounded(1);
        if !self.send(make(reply)) {
            return None;
        }
        match answer.recv_deadline(wait_until) {
            Ok(value) => Some(value),
            Err(_) => {
                self.timed_out.store(true, Ordering::SeqCst);
                None
            }
        }
    }

    /// Waits until everything sent before has been handled, even by a provider that missed
    /// a deadline.
    pub fn flush(&self, wait: Duration) -> bool {
        let (reply, done) = bounded(1);
        self.send(Command::Flush(reply)) && done.recv_timeout(wait).is_ok()
    }

    pub fn stalled(&self) -> bool {
        self.timed_out.load(Ordering::SeqCst) && self.inflight.load(Ordering::SeqCst) > 0
    }

    fn send(&self, command: Command) -> bool {
        self.inflight.fetch_add(1, Ordering::SeqCst);
        if self.tx.send(command).is_ok() {
            true
        } else {
            self.inflight.fetch_sub(1, Ordering::SeqCst);
            false
        }
    }
}

struct Forward {
    tx: Sender<Msg>,
    provider: usize,
}

impl EventSender<InventoryEvent> for Forward {
    fn send(&self, event: InventoryEvent) -> bool {
        self.tx
            .send(Msg::Inventory {
                provider: self.provider,
                event,
            })
            .is_ok()
    }
}

fn run(
    index: usize,
    mut provider: Box<dyn Provider>,
    commands: &Receiver<Command>,
    events: &Sender<Msg>,
    inflight: &AtomicUsize,
) {
    let mut started = false;
    let scan = |provider: &mut Box<dyn Provider>, started: &mut bool| {
        let result = if *started {
            Ok(())
        } else {
            provider.start(Box::new(Forward {
                tx: events.clone(),
                provider: index,
            }))
        };
        *started = result.is_ok();
        let result = result.and_then(|()| provider.snapshot(Instant::now() + SCAN_BUDGET));
        events
            .send(Msg::Snapshot {
                provider: index,
                result,
                capabilities: provider.capabilities(),
            })
            .is_ok()
    };
    let alive = scan(&mut provider, &mut started);
    inflight.fetch_sub(1, Ordering::SeqCst);
    if !alive {
        return;
    }
    for command in commands {
        match command {
            Command::Refresh => {
                if !scan(&mut provider, &mut started) {
                    return;
                }
            }
            Command::Reverify {
                target,
                deadline,
                reply,
            } => {
                let _ = reply.send(provider.reverify(&target, deadline));
            }
            Command::Execute {
                step,
                target,
                ctx,
                deadline,
                reply,
            } => {
                let _ = reply.send(provider.execute(&step, &target, &ctx, deadline));
            }
            Command::Restore {
                target,
                action,
                deadline,
                reply,
            } => {
                let _ = reply.send(provider.restore(&target, action, deadline));
            }
            Command::Flush(reply) => {
                let _ = reply.send(());
            }
        }
        inflight.fetch_sub(1, Ordering::SeqCst);
    }
}

pub(crate) fn is_permission_missing(error: &ProviderError) -> bool {
    matches!(error, ProviderError::PermissionMissing(_))
}

/// `Other` carries the adapter's own text, which may quote a window title, so logs get the
/// kind only.
pub(crate) fn error_kind(error: &ProviderError) -> &'static str {
    match error {
        ProviderError::PermissionMissing(_) => "permission_missing",
        ProviderError::Timeout => "timeout",
        ProviderError::Other(_) => "other",
    }
}
