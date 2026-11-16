use crate::{AppSelector, CategoryDef, CategoryId, Evidence, Matcher, Target};

pub trait CategoryIndex {
    fn category(&self, id: &CategoryId) -> Option<&CategoryDef>;
    /// `host` is already lowercased and punycoded; parent domains count as matches.
    fn list_contains(&self, list_id: &str, host: &str) -> bool;
}

pub struct MatchContext<'a> {
    pub categories: &'a dyn CategoryIndex,
    pub home: &'a str,
}

pub fn evaluate(matcher: &Matcher, target: &Target, ctx: &MatchContext<'_>) -> Option<Evidence> {
    todo!(
        "closecall-core: evaluate {matcher:?} {target:?} {}",
        ctx.home
    )
}

pub fn app_matches(selector: &AppSelector, target: &Target) -> bool {
    todo!("closecall-core: app_matches {selector:?} {target:?}")
}
