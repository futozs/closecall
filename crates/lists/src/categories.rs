use closecall_core::{AppSelector, CategoryDef, CategoryId};

/// Desktop ids are desktop-file ids without the `.desktop` suffix (the Wayland app id for
/// well-behaved apps). Keywords and globs are matched by core after case and diacritic folding.
pub(crate) struct Builtin {
    pub id: &'static str,
    pub default_enabled: bool,
    pub domains: &'static [&'static str],
    pub lists: &'static [&'static str],
    pub bundle_ids: &'static [&'static str],
    pub exes: &'static [&'static str],
    pub desktop_ids: &'static [&'static str],
    pub title_keywords: &'static [&'static str],
    pub file_globs: &'static [&'static str],
}

const EMPTY: Builtin = Builtin {
    id: "",
    default_enabled: true,
    domains: &[],
    lists: &[],
    bundle_ids: &[],
    exes: &[],
    desktop_ids: &[],
    title_keywords: &[],
    file_globs: &[],
};

pub(crate) const BUILTINS: &[Builtin] = &[
    Builtin {
        id: "adult",
        lists: &[
            "hagezi-nsfw",
            "sinfonietta-porn",
            "pornhosts",
            "hostsvn-adult",
            "tiuxo-porn",
            "sinfonietta-snuff",
        ],
        ..EMPTY
    },
    Builtin {
        id: "banking",
        lists: &["seed-banking", "ut1-bank"],
        title_keywords: &["Internetbank", "NetBank", "e-bank"],
        ..EMPTY
    },
    Builtin {
        id: "webmail",
        lists: &["seed-webmail", "ut1-webmail"],
        ..EMPTY
    },
    Builtin {
        id: "medical",
        lists: &["seed-medical"],
        title_keywords: &[
            "EESZT",
            "zárójelentés",
            "laborlelet",
            "leleteim",
            "ambuláns lap",
            "kórlap",
            "MyChart",
            "patient portal",
            "lab results",
            "medical record",
            "discharge summary",
        ],
        ..EMPTY
    },
    Builtin {
        id: "password_managers",
        bundle_ids: &[
            "com.apple.Passwords",
            "com.apple.keychainaccess",
            "com.1password.1password",
            "com.agilebits.onepassword7",
            "com.bitwarden.desktop",
            "org.keepassxc.keepassxc",
            "com.hicknhacksoftware.MacPass",
            "in.sinew.Enpass-Desktop",
            "com.markmcguill.strongbox.mac",
            "me.proton.pass.electron",
            "com.lastpass.lastpassmacdesktop",
        ],
        exes: &[
            "1Password.exe",
            "Bitwarden.exe",
            "KeePassXC.exe",
            "KeePass.exe",
            "Enpass.exe",
        ],
        desktop_ids: &[
            "1password",
            "bitwarden",
            "com.bitwarden.desktop",
            "org.keepassxc.KeePassXC",
            "keepass2",
            "org.gnome.seahorse.Application",
        ],
        ..EMPTY
    },
    Builtin {
        id: "messengers",
        domains: &[
            "web.whatsapp.com",
            "web.telegram.org",
            "messenger.com",
            "messages.google.com",
            "app.slack.com",
        ],
        bundle_ids: &[
            "com.apple.MobileSMS",
            "org.whispersystems.signal-desktop",
            "net.whatsapp.WhatsApp",
            "desktop.WhatsApp",
            "ru.keepcoder.Telegram",
            "com.tdesktop.Telegram",
            "com.tinyspeck.slackmacgap",
            "com.facebook.archon",
            "com.facebook.archon.developerID",
            "com.viber.osx",
        ],
        exes: &[
            "Signal.exe",
            "WhatsApp.exe",
            "WhatsApp.Root.exe",
            "Telegram.exe",
            "slack.exe",
            "Messenger.exe",
            "Viber.exe",
        ],
        desktop_ids: &[
            "org.signal.Signal",
            "signal-desktop",
            "org.telegram.desktop",
            "com.slack.Slack",
            "slack",
            "com.viber.Viber",
            "viber",
        ],
        ..EMPTY
    },
    Builtin {
        id: "invoices",
        title_keywords: &[
            "számla",
            "szamla",
            "invoice",
            "bérjegyzék",
            "payslip",
            "bankszámlakivonat",
            "bank statement",
            "adóbevallás",
            "tax return",
            "receipt",
        ],
        file_globs: &[
            "**/*szamla*.pdf",
            "**/*számla*.pdf",
            "**/*invoice*.pdf",
            "**/*berjegyzek*.pdf",
            "**/*bérjegyzék*.pdf",
            "**/*payslip*.pdf",
            "**/*kivonat*.pdf",
            "**/*adobevallas*.pdf",
            "**/*adóbevallás*.pdf",
            "**/*receipt*.pdf",
        ],
        ..EMPTY
    },
    Builtin {
        id: "meeting",
        domains: &[
            "meet.google.com",
            "teams.microsoft.com",
            "teams.live.com",
            "teams.cloud.microsoft",
            "zoom.us",
            "zoom.com",
            "webex.com",
            "whereby.com",
            "discord.com",
        ],
        bundle_ids: &[
            "us.zoom.xos",
            "com.microsoft.teams2",
            "com.microsoft.teams",
            "com.hnc.Discord",
            "com.cisco.webexmeetingsapp",
            "Cisco-Systems.Spark",
            "com.apple.FaceTime",
            "com.obsproject.obs-studio",
        ],
        exes: &[
            "Zoom.exe",
            "ms-teams.exe",
            "Teams.exe",
            "Discord.exe",
            "CiscoCollabHost.exe",
            "atmgr.exe",
            "obs64.exe",
        ],
        desktop_ids: &[
            "us.zoom.Zoom",
            "Zoom",
            "com.discordapp.Discord",
            "discord",
            "com.obsproject.Studio",
            "com.github.IsmaelMartinez.teams_for_linux",
        ],
        ..EMPTY
    },
    Builtin {
        id: "financial",
        default_enabled: false,
        lists: &["ut1-financial"],
        ..EMPTY
    },
    Builtin {
        id: "dating",
        default_enabled: false,
        lists: &["ut1-dating"],
        ..EMPTY
    },
    Builtin {
        id: "gambling",
        default_enabled: false,
        lists: &["ut1-gambling"],
        ..EMPTY
    },
];

pub(crate) fn find(id: &str) -> Option<&'static Builtin> {
    BUILTINS.iter().find(|builtin| builtin.id == id)
}

impl Builtin {
    pub fn to_def(&self) -> CategoryDef {
        let strings = |items: &[&str]| items.iter().map(|item| (*item).to_owned()).collect();
        let apps = self
            .bundle_ids
            .iter()
            .map(|id| AppSelector::BundleId((*id).to_owned()))
            .chain(
                self.exes
                    .iter()
                    .map(|exe| AppSelector::Exe((*exe).to_owned())),
            )
            .chain(
                self.desktop_ids
                    .iter()
                    .map(|id| AppSelector::DesktopId((*id).to_owned())),
            )
            .collect();
        CategoryDef {
            id: CategoryId(self.id.to_owned()),
            name: None,
            domains: strings(self.domains),
            lists: strings(self.lists),
            apps,
            title_keywords: strings(self.title_keywords),
            file_globs: strings(self.file_globs),
        }
    }
}
