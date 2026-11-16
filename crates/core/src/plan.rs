use crate::config::Protect;
use crate::matcher::CategoryIndex;
use crate::{AppSelector, Capability, FiredTrigger, Plan, Rule, Target, Trigger};

pub struct PlanContext<'a> {
    pub categories: &'a dyn CategoryIndex,
    pub protect: &'a Protect,
    /// Protected for this run only, e.g. the app that is capturing the screen.
    pub protect_apps: &'a [AppSelector],
    pub capabilities: &'a [Capability],
    pub now_ms: u64,
    pub home: &'a str,
}

pub fn armed(rule: &Rule, fired: &FiredTrigger) -> bool {
    todo!("closecall-core: armed {rule:?} {fired:?}")
}

pub fn plan(
    fired: &FiredTrigger,
    snapshot: &[Target],
    rules: &[Rule],
    ctx: &PlanContext<'_>,
) -> Plan {
    todo!(
        "closecall-core: plan {fired:?} over {} targets and {} rules at {}",
        snapshot.len(),
        rules.len(),
        ctx.now_ms
    )
}

pub fn trigger_allows_confirm(trigger: &Trigger) -> bool {
    matches!(
        trigger,
        Trigger::ItemOpenLongerThan(_) | Trigger::IdleLongerThan(_)
    )
}
