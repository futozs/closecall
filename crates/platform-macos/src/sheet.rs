//! Answering a save sheet by button role, never by its localized title.

use closecall_core::UnsavedPolicy;

use crate::keys;

pub(crate) trait Node: Sized {
    fn role(&self) -> Option<String>;
    fn children(&self) -> Vec<Self>;
    fn same(&self, other: &Self) -> bool;
}

const SEARCH_DEPTH: usize = 4;

pub(crate) fn find_sheet<N: Node>(window: &N) -> Option<N> {
    window
        .children()
        .into_iter()
        .find(|child| child.role().as_deref() == Some("AXSheet"))
}

pub(crate) fn buttons<N: Node>(root: &N) -> Vec<N> {
    fn walk<N: Node>(node: &N, depth: usize, out: &mut Vec<N>) {
        for child in node.children() {
            if child.role().as_deref() == Some("AXButton") {
                out.push(child);
            } else if depth > 0 {
                walk(&child, depth - 1, out);
            }
        }
    }
    let mut out = Vec::new();
    walk(root, SEARCH_DEPTH, &mut out);
    out
}

/// What a Save or Export panel has and an "unsaved changes" alert does not.
const FORM_CONTROLS: [&str; 6] = [
    "AXTextField",
    "AXComboBox",
    "AXPopUpButton",
    "AXOutline",
    "AXBrowser",
    "AXTable",
];

fn has_form_controls<N: Node>(root: &N) -> bool {
    fn walk<N: Node>(node: &N, depth: usize) -> bool {
        node.children().iter().any(|child| {
            child
                .role()
                .is_some_and(|role| FORM_CONTROLS.contains(&role.as_str()))
                || (depth > 0 && walk(child, depth - 1))
        })
    }
    walk(root, SEARCH_DEPTH)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) struct Roles {
    pub count: usize,
    /// Save.
    pub default: Option<usize>,
    pub cancel: Option<usize>,
    pub form_controls: bool,
}

pub(crate) fn roles<N: Node>(
    sheet: &N,
    buttons: &[N],
    default: Option<&N>,
    cancel: Option<&N>,
) -> Roles {
    let position = |wanted: Option<&N>| wanted.and_then(|w| buttons.iter().position(|b| b.same(w)));
    Roles {
        count: buttons.len(),
        default: position(default),
        cancel: position(cancel),
        form_controls: has_form_controls(sheet),
    }
}

impl Roles {
    /// Save, Don't Save and Cancel and nothing else: the only sheet it is safe to answer. A
    /// "Terminate running processes?" sheet has two buttons, a Save panel has a name field.
    fn is_save_alert(&self) -> bool {
        self.count == 3
            && !self.form_controls
            && matches!((self.default, self.cancel), (Some(d), Some(c)) if d != c)
    }

    /// "Don't Save", or "Delete" on an autosaved untitled document: the one button that is
    /// neither default nor cancel. `None` when that is ambiguous.
    fn other(&self) -> Option<usize> {
        let mut rest =
            (0..self.count).filter(|&i| Some(i) != self.default && Some(i) != self.cancel);
        let first = rest.next()?;
        rest.next().is_none().then_some(first)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SheetAnswer {
    Press(usize),
    /// Press Cancel (when there is one), then conceal the window.
    CancelAndConceal(&'static str),
    CancelAndSkip(&'static str),
}

pub(crate) fn choose(
    policy: UnsavedPolicy,
    has_path: bool,
    autosaves: bool,
    roles: Roles,
) -> SheetAnswer {
    match policy {
        UnsavedPolicy::Hide => SheetAnswer::CancelAndConceal(keys::UNSAVED_HIDDEN),
        UnsavedPolicy::SkipAndNotify => SheetAnswer::CancelAndSkip(keys::UNSAVED_SKIPPED),
        UnsavedPolicy::SaveThenClose if !has_path => {
            SheetAnswer::CancelAndConceal(keys::UNTITLED_HIDDEN)
        }
        UnsavedPolicy::Discard if !has_path => {
            SheetAnswer::CancelAndConceal(keys::DISCARD_UNTITLED_HIDDEN)
        }
        UnsavedPolicy::Discard if autosaves => {
            SheetAnswer::CancelAndConceal(keys::AUTOSAVE_DISCARD_HIDDEN)
        }
        _ if !roles.is_save_alert() => {
            SheetAnswer::CancelAndConceal(keys::SHEET_UNRECOGNIZED_HIDDEN)
        }
        UnsavedPolicy::SaveThenClose => roles.default.map_or(
            SheetAnswer::CancelAndConceal(keys::SHEET_UNRECOGNIZED_HIDDEN),
            SheetAnswer::Press,
        ),
        UnsavedPolicy::Discard => roles.other().map_or(
            SheetAnswer::CancelAndConceal(keys::SHEET_UNRECOGNIZED_HIDDEN),
            SheetAnswer::Press,
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug, Clone)]
    struct Fake {
        id: u32,
        role: &'static str,
        children: Vec<Fake>,
    }

    fn node(id: u32, role: &'static str, children: Vec<Fake>) -> Fake {
        Fake { id, role, children }
    }

    impl Node for Fake {
        fn role(&self) -> Option<String> {
            Some(self.role.to_owned())
        }
        fn children(&self) -> Vec<Self> {
            self.children.clone()
        }
        fn same(&self, other: &Self) -> bool {
            self.id == other.id
        }
    }

    /// A Hungarian "Menti a módosításokat?" sheet: the titles are irrelevant, only roles.
    fn save_sheet_window() -> Fake {
        node(
            1,
            "AXWindow",
            vec![
                node(2, "AXTextArea", vec![]),
                node(
                    3,
                    "AXSheet",
                    vec![
                        node(4, "AXStaticText", vec![]),
                        node(
                            5,
                            "AXGroup",
                            vec![
                                node(10, "AXButton", vec![]),
                                node(11, "AXButton", vec![]),
                                node(12, "AXButton", vec![]),
                            ],
                        ),
                    ],
                ),
            ],
        )
    }

    fn roles_of(window: &Fake, default: u32, cancel: u32) -> Roles {
        let sheet = find_sheet(window).unwrap();
        let found = buttons(&sheet);
        let by_id = |id| found.iter().find(|b| b.id == id).cloned();
        roles(
            &sheet,
            &found,
            by_id(default).as_ref(),
            by_id(cancel).as_ref(),
        )
    }

    fn sheet_roles(default: u32, cancel: u32) -> Roles {
        roles_of(&save_sheet_window(), default, cancel)
    }

    #[test]
    fn only_a_three_button_alert_counts_as_a_save_sheet() {
        // Terminal's "Terminate running processes?": Terminate is the default button.
        let terminate = node(
            1,
            "AXWindow",
            vec![node(
                3,
                "AXSheet",
                vec![node(10, "AXButton", vec![]), node(11, "AXButton", vec![])],
            )],
        );
        let roles = roles_of(&terminate, 11, 10);
        for policy in [UnsavedPolicy::SaveThenClose, UnsavedPolicy::Discard] {
            assert_eq!(
                choose(policy, true, false, roles),
                SheetAnswer::CancelAndConceal(keys::SHEET_UNRECOGNIZED_HIDDEN)
            );
        }

        // A Save panel: a name field and a folder pop-up next to three buttons.
        let panel = node(
            1,
            "AXWindow",
            vec![node(
                3,
                "AXSheet",
                vec![
                    node(4, "AXTextField", vec![]),
                    node(5, "AXGroup", vec![node(6, "AXPopUpButton", vec![])]),
                    node(10, "AXButton", vec![]),
                    node(11, "AXButton", vec![]),
                    node(12, "AXButton", vec![]),
                ],
            )],
        );
        let roles = roles_of(&panel, 12, 11);
        assert!(roles.form_controls);
        assert_eq!(
            choose(UnsavedPolicy::SaveThenClose, true, false, roles),
            SheetAnswer::CancelAndConceal(keys::SHEET_UNRECOGNIZED_HIDDEN)
        );
    }

    #[test]
    fn finds_the_sheet_and_its_nested_buttons() {
        let window = save_sheet_window();
        let sheet = find_sheet(&window).unwrap();
        assert_eq!(sheet.id, 3);
        let ids: Vec<u32> = buttons(&sheet).iter().map(|b| b.id).collect();
        assert_eq!(ids, [10, 11, 12]);
        assert!(find_sheet(&node(1, "AXWindow", vec![node(2, "AXGroup", vec![])])).is_none());
    }

    #[test]
    fn save_presses_the_default_button() {
        let roles = sheet_roles(12, 11);
        assert_eq!(
            choose(UnsavedPolicy::SaveThenClose, true, false, roles),
            SheetAnswer::Press(2)
        );
    }

    #[test]
    fn discard_presses_the_remaining_button() {
        let roles = sheet_roles(12, 11);
        assert_eq!(
            choose(UnsavedPolicy::Discard, true, false, roles),
            SheetAnswer::Press(0)
        );
    }

    #[test]
    fn ambiguous_or_missing_roles_never_guess() {
        let no_cancel = Roles {
            count: 3,
            default: Some(2),
            cancel: None,
            form_controls: false,
        };
        assert_eq!(
            choose(UnsavedPolicy::Discard, true, false, no_cancel),
            SheetAnswer::CancelAndConceal(keys::SHEET_UNRECOGNIZED_HIDDEN)
        );
        let no_default = Roles {
            count: 2,
            default: None,
            cancel: Some(1),
            form_controls: false,
        };
        assert_eq!(
            choose(UnsavedPolicy::SaveThenClose, true, false, no_default),
            SheetAnswer::CancelAndConceal(keys::SHEET_UNRECOGNIZED_HIDDEN)
        );
    }

    #[test]
    fn untitled_and_autosaved_documents_are_concealed_instead() {
        let roles = sheet_roles(12, 11);
        assert_eq!(
            choose(UnsavedPolicy::SaveThenClose, false, false, roles),
            SheetAnswer::CancelAndConceal(keys::UNTITLED_HIDDEN)
        );
        assert_eq!(
            choose(UnsavedPolicy::Discard, false, false, roles),
            SheetAnswer::CancelAndConceal(keys::DISCARD_UNTITLED_HIDDEN)
        );
        assert_eq!(
            choose(UnsavedPolicy::Discard, true, true, roles),
            SheetAnswer::CancelAndConceal(keys::AUTOSAVE_DISCARD_HIDDEN)
        );
        assert_eq!(
            choose(UnsavedPolicy::Hide, true, false, roles),
            SheetAnswer::CancelAndConceal(keys::UNSAVED_HIDDEN)
        );
        assert_eq!(
            choose(UnsavedPolicy::SkipAndNotify, true, false, roles),
            SheetAnswer::CancelAndSkip(keys::UNSAVED_SKIPPED)
        );
    }
}
