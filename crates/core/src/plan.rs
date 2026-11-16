use crate::config::Protect;
use crate::matcher::{CategoryIndex, MatchContext, app_matches, evaluate, matches};
use crate::{
    Action, AppSelector, Capability, Confirm, Evidence, FiredTrigger, Matcher, Phase, Plan, Reason,
    Rule, Step, Support, Target, TargetKind, TargetKindTag, Tri, Trigger, UnsavedPolicy,
};

pub struct PlanContext<'a> {
    pub categories: &'a dyn CategoryIndex,
    pub protect: &'a Protect,
    /// Protected for this run only, e.g. the app that is capturing the screen.
    pub protect_apps: &'a [AppSelector],
    pub capabilities: &'a [Capability],
    pub now_ms: u64,
    pub home: &'a str,
}

/// Screen-share confidence is not checked here; the engine filters it before firing.
pub fn armed(rule: &Rule, fired: &FiredTrigger) -> bool {
    rule.enabled && rule.when.iter().any(|trigger| fires(trigger, fired))
}

fn fires(trigger: &Trigger, fired: &FiredTrigger) -> bool {
    match (trigger, fired) {
        (Trigger::Panic, FiredTrigger::Panic)
        | (Trigger::ScreenShare, FiredTrigger::ScreenShare { .. })
        | (Trigger::DisplayMirrored, FiredTrigger::DisplayMirrored)
        | (Trigger::DisplayConnected, FiredTrigger::DisplayConnected)
        | (Trigger::CallLikely, FiredTrigger::CallLikely)
        | (Trigger::WillSleep, FiredTrigger::WillSleep)
        | (Trigger::LidClosed, FiredTrigger::LidClosed)
        | (Trigger::Locked, FiredTrigger::Locked) => true,
        (Trigger::AwayLongerThan(min), FiredTrigger::Away { away }) => away >= min,
        (Trigger::IdleLongerThan(min), FiredTrigger::Idle { idle }) => idle >= min,
        (Trigger::ItemOpenLongerThan(min), FiredTrigger::ItemAge { age, .. }) => age >= min,
        _ => false,
    }
}

/// Steps are ordered by phase, then target id, whatever the order of `snapshot` and `rules`.
pub fn plan(
    fired: &FiredTrigger,
    snapshot: &[Target],
    rules: &[Rule],
    ctx: &PlanContext<'_>,
) -> Plan {
    let match_ctx = MatchContext {
        categories: ctx.categories,
        home: ctx.home,
    };
    let armed_rules: Vec<&Rule> = rules.iter().filter(|rule| armed(rule, fired)).collect();
    let only = match fired {
        FiredTrigger::ItemAge { target, .. } => Some(*target),
        _ => None,
    };
    let protected = protection_matcher(ctx);

    let mut steps: Vec<Step> = snapshot
        .iter()
        .filter(|target| only.is_none_or(|id| id == target.id))
        .filter_map(|target| {
            let hits: Vec<(&Rule, Evidence)> = armed_rules
                .iter()
                .filter_map(|rule| select(rule, target, &match_ctx).map(|ev| (*rule, ev)))
                .collect();
            if hits.is_empty()
                || is_protected(
                    target,
                    &protected,
                    ctx.protect,
                    ctx.protect_apps,
                    &match_ctx,
                )
            {
                return None;
            }
            Some(step(target, hits, ctx.capabilities))
        })
        .collect();
    steps.sort_by_key(|step| (step.phase, step.target));
    Plan { steps }
}

fn select(rule: &Rule, target: &Target, ctx: &MatchContext<'_>) -> Option<Evidence> {
    if !rule.targets.contains(&target.kind.tag()) {
        return None;
    }
    let evidence = evaluate(&rule.matcher, target, ctx)?;
    let excepted = rule
        .except
        .as_ref()
        .is_some_and(|except| matches(except, target, ctx));
    (!excepted).then_some(evidence)
}

fn protection_matcher(ctx: &PlanContext<'_>) -> Matcher {
    let protect = ctx.protect;
    Matcher::Any(
        protect
            .categories
            .iter()
            .cloned()
            .map(Matcher::Category)
            .chain(protect.domains.iter().cloned().map(Matcher::Domain))
            .chain(protect.apps.iter().cloned().map(Matcher::App))
            .collect(),
    )
}

fn is_protected(
    target: &Target,
    protected: &Matcher,
    protect: &Protect,
    run_apps: &[AppSelector],
    ctx: &MatchContext<'_>,
) -> bool {
    match &target.kind {
        TargetKind::BrowserTab(tab) => {
            if (protect.audible_tabs && tab.audible == Tri::Yes)
                || (protect.pinned_tabs && tab.pinned == Tri::Yes)
            {
                return true;
            }
        }
        // The capturing app is protected as a whole, but its tabs are not: when Meet runs in
        // Chrome, the bank tab next to it must still close.
        _ if run_apps.iter().any(|app| app_matches(app, target)) => return true,
        _ => {}
    }
    matches(protected, target, ctx)
}

fn step(target: &Target, hits: Vec<(&Rule, Evidence)>, capabilities: &[Capability]) -> Step {
    let wanted = hits
        .iter()
        .map(|(rule, _)| rule.action)
        .fold(Action::NotifyOnly, |a, b| {
            if severity(b) > severity(a) { b } else { a }
        });
    let on_unsaved =
        hits.iter()
            .map(|(rule, _)| rule.on_unsaved)
            .fold(UnsavedPolicy::Discard, |a, b| {
                if preservation(b) > preservation(a) {
                    b
                } else {
                    a
                }
            });
    let confirm = hits
        .iter()
        .filter_map(|(rule, _)| match rule.confirm {
            Confirm::Countdown(countdown) => Some(countdown),
            Confirm::Never => None,
        })
        .max()
        .map_or(Confirm::Never, Confirm::Countdown);
    let mut because: Vec<Reason> = hits
        .into_iter()
        .map(|(rule, evidence)| Reason {
            rule: rule.id.clone(),
            evidence,
        })
        .collect();
    because.sort_by(|a, b| a.rule.cmp(&b.rule));

    let action = resolve(wanted, target, capabilities);
    Step {
        target: target.id,
        action,
        on_unsaved,
        phase: phase(action, target.kind.tag()),
        fell_back_from: (action != wanted).then_some(wanted),
        confirm,
        because,
    }
}

fn severity(action: Action) -> u8 {
    match action {
        Action::NotifyOnly => 0,
        Action::Minimize => 1,
        Action::Hide => 2,
        Action::Close => 3,
        Action::Quit => 4,
    }
}

fn preservation(policy: UnsavedPolicy) -> u8 {
    match policy {
        UnsavedPolicy::Discard => 0,
        UnsavedPolicy::SaveThenClose => 1,
        UnsavedPolicy::SkipAndNotify => 2,
        UnsavedPolicy::Hide => 3,
    }
}

fn resolve(wanted: Action, target: &Target, capabilities: &[Capability]) -> Action {
    let mut action = wanted;
    // The engine sends notifications itself, so NotifyOnly needs no provider support.
    while action != Action::NotifyOnly && !supported(capabilities, target, action) {
        action = match action {
            Action::Quit => Action::Close,
            Action::Close => Action::Hide,
            Action::Hide => Action::Minimize,
            Action::Minimize | Action::NotifyOnly => Action::NotifyOnly,
        };
    }
    action
}

fn supported(capabilities: &[Capability], target: &Target, action: Action) -> bool {
    let kind = target.kind.tag();
    let rows = || {
        capabilities.iter().filter(move |capability| {
            capability.provider == target.provider
                && capability.target_kind == kind
                && capability.action == action
        })
    };
    rows()
        .find(|capability| capability.app.as_deref() == Some(target.app.id.as_str()))
        .or_else(|| rows().find(|capability| capability.app.is_none()))
        .is_some_and(|capability| !matches!(capability.support, Support::Unsupported))
}

fn phase(action: Action, kind: TargetKindTag) -> Phase {
    match action {
        Action::Hide | Action::Minimize | Action::Quit => Phase::Conceal,
        Action::Close if kind == TargetKindTag::Tab => Phase::CloseTabs,
        Action::Close => Phase::CloseDocuments,
        Action::NotifyOnly => Phase::Notify,
    }
}

pub fn trigger_allows_confirm(trigger: &Trigger) -> bool {
    matches!(
        trigger,
        Trigger::ItemOpenLongerThan(_) | Trigger::IdleLongerThan(_)
    )
}

#[cfg(test)]
mod tests;
