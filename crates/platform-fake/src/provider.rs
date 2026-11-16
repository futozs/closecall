use std::sync::Arc;
use std::time::{Duration, Instant};

use closecall_core::platform::{
    Discovered, EventSender, ExecContext, InventoryEvent, Provider, ProviderError, Reverify,
};
use closecall_core::{
    Action, Capability, Dirty, DocumentWindow, ProviderId, Step, StepOutcome, Support, Target,
    TargetKind, TargetKindTag, UnsavedPolicy,
};

use crate::targets::key_of;
use crate::{APPS, CHROMIUM, DOCUMENTS, Executed, Item, Restored, SAFARI, Shared, State};

/// A hung provider blocks at most this long, so a forgotten `release` cannot wedge a test run.
const HANG_LIMIT: Duration = Duration::from_secs(10);

pub(crate) struct FakeProvider {
    id: &'static str,
    shared: Arc<Shared>,
}

impl FakeProvider {
    pub fn new(id: &'static str, shared: Arc<Shared>) -> Self {
        Self { id, shared }
    }
}

impl Provider for FakeProvider {
    fn id(&self) -> ProviderId {
        ProviderId(self.id.to_owned())
    }

    fn capabilities(&self) -> Vec<Capability> {
        capabilities(self.id)
    }

    fn start(&mut self, events: Box<dyn EventSender<InventoryEvent>>) -> Result<(), ProviderError> {
        let mut state = self.shared.state();
        state.inventory.insert(self.id.to_owned(), events);
        match state.denied_providers.get(self.id) {
            Some(permission) => Err(ProviderError::PermissionMissing(permission.clone())),
            None => Ok(()),
        }
    }

    fn snapshot(&mut self, _deadline: Instant) -> Result<Vec<Discovered>, ProviderError> {
        let state = self.shared.state();
        if let Some(permission) = state.denied_providers.get(self.id) {
            return Err(ProviderError::PermissionMissing(permission.clone()));
        }
        Ok(state
            .items
            .iter()
            .filter(|(_, item)| item.target.provider.0 == self.id)
            .map(|(key, item)| Discovered {
                key: key.clone(),
                target: item.target.clone(),
            })
            .collect())
    }

    fn reverify(&mut self, target: &Target, _deadline: Instant) -> Reverify {
        self.shared.wait_while_hung(self.id, HANG_LIMIT);
        let state = self.shared.state();
        match state.items.get(&key_of(target)) {
            None => Reverify::Gone,
            Some(item) if item.target.app == target.app && item.target.kind == target.kind => {
                Reverify::Same
            }
            Some(item) => Reverify::Changed(Target {
                id: target.id,
                first_seen_ms: target.first_seen_ms,
                ..item.target.clone()
            }),
        }
    }

    fn execute(
        &mut self,
        step: &Step,
        target: &Target,
        ctx: &ExecContext,
        _deadline: Instant,
    ) -> StepOutcome {
        self.shared.wait_while_hung(self.id, HANG_LIMIT);
        let key = key_of(target);
        let mut state = self.shared.state();
        let outcome = act(&mut state, self.id, &key, step);
        state.executed.push(Executed {
            provider: self.id.to_owned(),
            key,
            action: step.action,
            on_unsaved: step.on_unsaved,
            outcome: outcome.clone(),
            untitled_save_dir: ctx.untitled_save_dir.clone(),
            planned: ctx.planned.iter().map(key_of).collect(),
        });
        outcome
    }

    fn restore(&mut self, target: &Target, action: Action, _deadline: Instant) -> StepOutcome {
        self.shared.wait_while_hung(self.id, HANG_LIMIT);
        let key = key_of(target);
        let mut state = self.shared.state();
        let outcome = if let Some(closed) = state.closed.remove(&key) {
            send(
                &state,
                self.id,
                InventoryEvent::Added(Discovered {
                    key: key.clone(),
                    target: closed.clone(),
                }),
            );
            state.items.insert(key.clone(), Item::new(closed));
            StepOutcome::Done
        } else {
            match state.items.get_mut(&key) {
                Some(item) if item.concealed => {
                    item.concealed = false;
                    StepOutcome::Done
                }
                _ => skipped("reason.gone"),
            }
        };
        state.restored.push(Restored {
            provider: self.id.to_owned(),
            key,
            action,
            outcome: outcome.clone(),
        });
        outcome
    }
}

/// The provider's own state is the truth: dirty flags and presence come from the item, not from
/// the target the engine passed in.
fn act(state: &mut State, provider: &str, key: &str, step: &Step) -> StepOutcome {
    let Some(item) = state.items.get(key) else {
        return skipped("reason.gone");
    };
    if !supported(provider, item.target.kind.tag(), step.action) {
        return skipped("reason.unsupported");
    }
    match (step.action, &item.target.kind) {
        (Action::NotifyOnly, _) => StepOutcome::Done,
        (Action::Hide | Action::Minimize, _) => {
            conceal(state, key);
            StepOutcome::Done
        }
        (Action::Close | Action::Quit, TargetKind::DocumentWindow(doc)) => {
            let doc = doc.clone();
            close_document(state, provider, key, &doc, step.on_unsaved)
        }
        (Action::Close | Action::Quit, _) => {
            close(state, provider, key);
            StepOutcome::Done
        }
    }
}

/// Mirrors the unsaved-document policy table: never lose work, never drive a Save panel, and an
/// untitled document cannot be saved by this app, so it is hidden instead.
fn close_document(
    state: &mut State,
    provider: &str,
    key: &str,
    doc: &DocumentWindow,
    policy: UnsavedPolicy,
) -> StepOutcome {
    if doc.dirty == Dirty::Clean {
        close(state, provider, key);
        return StepOutcome::Done;
    }
    let has_path = doc.path.is_some();
    match policy {
        UnsavedPolicy::SaveThenClose | UnsavedPolicy::Discard if has_path => {
            close(state, provider, key);
            StepOutcome::Done
        }
        UnsavedPolicy::SkipAndNotify => skipped("reason.unsaved_skipped"),
        UnsavedPolicy::SaveThenClose => {
            conceal(state, key);
            fell_back("reason.untitled_hidden")
        }
        UnsavedPolicy::Hide | UnsavedPolicy::Discard => {
            conceal(state, key);
            fell_back("reason.unsaved_hidden")
        }
    }
}

fn conceal(state: &mut State, key: &str) {
    if let Some(item) = state.items.get_mut(key) {
        item.concealed = true;
    }
}

fn close(state: &mut State, provider: &str, key: &str) {
    if let Some(item) = state.items.remove(key) {
        state.closed.insert(key.to_owned(), item.target);
        send(
            state,
            provider,
            InventoryEvent::Removed {
                provider: ProviderId(provider.to_owned()),
                key: key.to_owned(),
            },
        );
    }
}

pub(crate) fn send(state: &State, provider: &str, event: InventoryEvent) -> bool {
    state
        .inventory
        .get(provider)
        .is_some_and(|sender| sender.send(event))
}

fn skipped(reason_key: &str) -> StepOutcome {
    StepOutcome::Skipped {
        reason_key: reason_key.to_owned(),
    }
}

fn fell_back(reason_key: &str) -> StepOutcome {
    StepOutcome::FellBack {
        action: Action::Hide,
        reason_key: reason_key.to_owned(),
    }
}

fn supported(provider: &str, kind: TargetKindTag, action: Action) -> bool {
    capabilities(provider).iter().any(|capability| {
        capability.target_kind == kind
            && capability.action == action
            && capability.support != Support::Unsupported
    })
}

fn capabilities(provider: &str) -> Vec<Capability> {
    let partial = |note_key: &str| Support::Partial {
        note_key: note_key.to_owned(),
    };
    let rows: Vec<(TargetKindTag, Action, Support)> = match provider {
        CHROMIUM | SAFARI => vec![
            (TargetKindTag::Tab, Action::Close, Support::Full),
            (
                TargetKindTag::Tab,
                Action::Hide,
                partial("note.tab_hide_minimizes"),
            ),
            (TargetKindTag::Tab, Action::Minimize, Support::Unsupported),
            (TargetKindTag::Tab, Action::Quit, Support::Unsupported),
            (TargetKindTag::Tab, Action::NotifyOnly, Support::Full),
        ],
        DOCUMENTS => vec![
            (TargetKindTag::Document, Action::Close, Support::Full),
            (TargetKindTag::Document, Action::Hide, Support::Full),
            (
                TargetKindTag::Document,
                Action::Minimize,
                partial("note.minimize_animates"),
            ),
            (TargetKindTag::Document, Action::Quit, Support::Unsupported),
            (TargetKindTag::Document, Action::NotifyOnly, Support::Full),
        ],
        APPS => vec![
            (TargetKindTag::AppWindow, Action::Close, Support::Full),
            (TargetKindTag::AppWindow, Action::Hide, Support::Full),
            (
                TargetKindTag::AppWindow,
                Action::Minimize,
                partial("note.minimize_animates"),
            ),
            (TargetKindTag::AppWindow, Action::Quit, Support::Unsupported),
            (TargetKindTag::AppWindow, Action::NotifyOnly, Support::Full),
            (TargetKindTag::Process, Action::Hide, Support::Full),
            (TargetKindTag::Process, Action::Close, Support::Unsupported),
            (
                TargetKindTag::Process,
                Action::Minimize,
                Support::Unsupported,
            ),
            (TargetKindTag::Process, Action::Quit, Support::Unsupported),
            (TargetKindTag::Process, Action::NotifyOnly, Support::Full),
        ],
        _ => Vec::new(),
    };
    rows.into_iter()
        .map(|(target_kind, action, support)| Capability {
            provider: ProviderId(provider.to_owned()),
            app: None,
            target_kind,
            action,
            support,
        })
        .collect()
}
