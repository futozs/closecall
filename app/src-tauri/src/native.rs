use closecall_core::config::Language;
use closecall_engine::Notice;
use serde::Deserialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Lang {
    En,
    Hu,
}

impl Lang {
    pub fn resolve(language: Language) -> Self {
        match language {
            Language::En => Self::En,
            Language::Hu => Self::Hu,
            Language::System => sys_locale::get_locale()
                .as_deref()
                .map_or(Self::En, Self::from_locale),
        }
    }

    fn from_locale(locale: &str) -> Self {
        let primary = locale.split(['-', '_']).next().unwrap_or_default();
        if primary.eq_ignore_ascii_case("hu") {
            Self::Hu
        } else {
            Self::En
        }
    }
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Strings {
    pub tray: TrayStrings,
    notify: NotifyStrings,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TrayStrings {
    pub open: String,
    pub clean_now: String,
    pub pause_15: String,
    pub pause_60: String,
    pub resume: String,
    pub status_idle: String,
    pub status_sharing: String,
    pub status_paused: String,
    pub quit: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NotifyStrings {
    run_title: String,
    run_body: RunBody,
    unsaved_hidden_one: String,
    unsaved_hidden_other: String,
    permission_missing: String,
    share_detected: String,
}

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct RunBody {
    done_one: String,
    done_other: String,
    hidden_one: String,
    hidden_other: String,
    skipped_one: String,
    skipped_other: String,
    separator: String,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Message {
    pub title: String,
    pub body: String,
}

impl Strings {
    /// `app_name` titles the notices that are not about a run.
    pub fn notice(&self, notice: &Notice, app_name: &str) -> Option<Message> {
        let notify = &self.notify;
        let (title, body) = match *notice {
            Notice::RunSummary {
                done,
                hidden,
                skipped,
            } => {
                let body = &notify.run_body;
                let parts: Vec<String> = [
                    (done, &body.done_one, &body.done_other),
                    (hidden, &body.hidden_one, &body.hidden_other),
                    (skipped, &body.skipped_one, &body.skipped_other),
                ]
                .into_iter()
                .filter(|(n, _, _)| *n > 0)
                .map(|(n, one, other)| plural(n, one, other))
                .collect();
                if parts.is_empty() {
                    return None;
                }
                (notify.run_title.clone(), parts.join(&body.separator))
            }
            Notice::UnsavedHidden { count } => (
                notify.run_title.clone(),
                plural(
                    count,
                    &notify.unsaved_hidden_one,
                    &notify.unsaved_hidden_other,
                ),
            ),
            Notice::PermissionMissing => (app_name.to_owned(), notify.permission_missing.clone()),
            Notice::ShareDetected => (app_name.to_owned(), notify.share_detected.clone()),
        };
        Some(Message { title, body })
    }
}

/// English and Hungarian both use `one` only for exactly 1.
fn plural(count: u32, one: &str, other: &str) -> String {
    let text = if count == 1 { one } else { other };
    text.replace("{{count}}", &count.to_string())
}

pub struct Native {
    en: Strings,
    hu: Strings,
}

impl Native {
    pub fn load() -> Result<Self, serde_json::Error> {
        Ok(Self {
            en: serde_json::from_str(include_str!("../../../locales/en/native.json"))?,
            hu: serde_json::from_str(include_str!("../../../locales/hu/native.json"))?,
        })
    }

    pub fn get(&self, lang: Lang) -> &Strings {
        match lang {
            Lang::En => &self.en,
            Lang::Hu => &self.hu,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn native() -> Native {
        Native::load().expect("embedded native.json files parse")
    }

    #[test]
    fn locale_picks_hungarian_only_for_hu() {
        assert_eq!(Lang::from_locale("hu-HU"), Lang::Hu);
        assert_eq!(Lang::from_locale("hu_HU"), Lang::Hu);
        assert_eq!(Lang::from_locale("HU"), Lang::Hu);
        assert_eq!(Lang::from_locale("en-US"), Lang::En);
        assert_eq!(Lang::from_locale("de"), Lang::En);
        assert_eq!(Lang::from_locale("hup"), Lang::En);
        assert_eq!(Lang::from_locale(""), Lang::En);
        assert_eq!(Lang::resolve(Language::Hu), Lang::Hu);
        assert_eq!(Lang::resolve(Language::En), Lang::En);
    }

    #[test]
    fn run_summary_lists_only_nonzero_counts() {
        let native = native();
        let message = native
            .get(Lang::En)
            .notice(
                &Notice::RunSummary {
                    done: 3,
                    hidden: 0,
                    skipped: 1,
                },
                "Close Call",
            )
            .expect("a run with counts has a message");
        assert_eq!(message.title, "Close Call cleaned up");
        assert_eq!(message.body, "3 closed, 1 left open");

        let hungarian = native
            .get(Lang::Hu)
            .notice(
                &Notice::RunSummary {
                    done: 1,
                    hidden: 2,
                    skipped: 0,
                },
                "Close Call",
            )
            .expect("a run with counts has a message");
        assert_eq!(hungarian.body, "1 bezárva, 2 elrejtve");
    }

    #[test]
    fn empty_run_sends_nothing() {
        let notice = Notice::RunSummary {
            done: 0,
            hidden: 0,
            skipped: 0,
        };
        assert_eq!(native().get(Lang::En).notice(&notice, "Close Call"), None);
    }

    #[test]
    fn plural_picks_one_and_other() {
        let native = native();
        let en = native.get(Lang::En);
        let one = en
            .notice(&Notice::UnsavedHidden { count: 1 }, "Close Call")
            .expect("message");
        let many = en
            .notice(&Notice::UnsavedHidden { count: 4 }, "Close Call")
            .expect("message");
        assert!(one.body.starts_with("1 document "), "{}", one.body);
        assert!(many.body.starts_with("4 documents "), "{}", many.body);
    }

    #[test]
    fn other_notices_are_titled_with_the_app_name() {
        let native = native();
        for lang in [Lang::En, Lang::Hu] {
            for notice in [Notice::PermissionMissing, Notice::ShareDetected] {
                let message = native
                    .get(lang)
                    .notice(&notice, "Close Call")
                    .expect("message");
                assert_eq!(message.title, "Close Call");
                assert!(!message.body.is_empty());
                assert!(!message.body.contains("{{"), "{}", message.body);
            }
        }
    }
}
