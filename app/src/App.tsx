import { type ComponentType, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { ActivityIcon, CategoriesIcon, Mark, RulesIcon, SettingsIcon, StatusIcon } from "./components/icons";
import { formatWhen } from "./format";
import { useHash } from "./hash";
import { Activity } from "./screens/Activity";
import { Categories } from "./screens/Categories";
import { Home } from "./screens/Home";
import { Onboarding } from "./screens/Onboarding";
import { Rules } from "./screens/Rules";
import { Settings } from "./screens/Settings";
import { useAppData } from "./store";

const SECTIONS = ["home", "rules", "categories", "activity", "settings"] as const;
type Section = (typeof SECTIONS)[number];
type Route = Section | "setup";

const ICONS: Record<Section, ComponentType<{ className?: string }>> = {
  home: StatusIcon,
  rules: RulesIcon,
  categories: CategoriesIcon,
  activity: ActivityIcon,
  settings: SettingsIcon,
};

const SCREENS: Record<Section, ComponentType> = {
  home: Home,
  rules: Rules,
  categories: Categories,
  activity: Activity,
  settings: Settings,
};

function parseRoute(hash: string): Route {
  const name = hash.replace(/^#/, "").split("/")[0];
  if (name === "setup") return "setup";
  return SECTIONS.find((section) => section === name) ?? "home";
}

function useRoute(initial: Route): Route {
  const hash = useHash();
  return hash ? parseRoute(hash) : initial;
}

function ShareState() {
  const { t } = useTranslation();
  const { status, locale } = useAppData();
  const { share } = status;
  const live = share.active || share.mirrored_display || share.external_display;
  const label = share.active
    ? t("shell.sharing")
    : share.mirrored_display
      ? t("shell.mirrored")
      : share.external_display
        ? t("shell.external")
        : status.paused_until_ms !== null
          ? t("shell.paused", { time: formatWhen(status.paused_until_ms, locale) })
          : t("shell.watching");
  return (
    <p className="flex min-w-0 items-center gap-2 text-sm text-ink-2" role="status">
      <span
        aria-hidden="true"
        className={`size-2 flex-none rounded-full ${live ? "bg-signal" : status.paused_until_ms !== null ? "bg-caution" : "bg-ok"}`}
      />
      <span className="truncate">{label}</span>
    </p>
  );
}

/** The engine runs the recommended rules because config.toml was rejected. */
function ConfigProblem() {
  const { t } = useTranslation();
  const { status } = useAppData();
  if (status.config_problem === null) return null;
  return (
    <div role="alert" className="mb-8 rounded-ctl bg-signal-wash px-4 py-3">
      <p className="font-semibold">{t("config.problem.title")}</p>
      <p className="mt-1 max-w-prose text-sm">{t(status.config_problem, { defaultValue: "" })}</p>
      <p className="mt-1 max-w-prose text-sm text-ink-2">{t("config.problem.kept")}</p>
    </div>
  );
}

export function App() {
  const { t } = useTranslation();
  const { status } = useAppData();
  // Decided once: the flag flips as soon as setup saves a config, and setup must not vanish then.
  const [start] = useState<Route>(status.needs_onboarding ? "setup" : "home");
  const route = useRoute(start);

  useEffect(() => {
    document.title = route === "setup" ? `${t("app.name")} – ${t("onboarding.title")}` : t("app.name");
  }, [route, t]);

  if (route === "setup") return <Onboarding />;
  const Screen = SCREENS[route];

  return (
    <div className="flex min-h-dvh flex-col sm:flex-row">
      <a
        href="#main"
        className="sr-only focus:not-sr-only focus:absolute focus:top-2 focus:left-2 focus:z-10 focus:rounded-ctl focus:bg-ink focus:px-3 focus:py-2 focus:text-on-ink"
      >
        {t("shell.skip")}
      </a>
      <header className="flex-none border-b border-rule bg-side sm:w-56 sm:border-r sm:border-b-0">
        <div className="sm:sticky sm:top-0 sm:flex sm:h-dvh sm:flex-col">
          <div className="flex items-center gap-2.5 px-4 pt-4 pb-3 sm:px-5 sm:pt-6 sm:pb-6">
            <Mark className="size-7 flex-none" />
            <div className="min-w-0 leading-tight">
              <p className="font-semibold">{t("app.name")}</p>
              <p className="text-xs text-ink-2">{t("app.tagline")}</p>
            </div>
            <div className="ml-auto min-w-0 sm:hidden">
              <ShareState />
            </div>
          </div>
          <nav aria-label={t("nav.label")} className="px-2 pb-2 sm:flex-1 sm:px-3">
            <ul className="grid grid-cols-5 gap-1 sm:flex sm:flex-col">
              {SECTIONS.map((section) => {
                const Icon = ICONS[section];
                const current = route === section;
                return (
                  <li key={section} className="min-w-0">
                    <a
                      href={`#${section}`}
                      aria-current={current ? "page" : undefined}
                      className={`flex flex-col items-center gap-1 rounded-ctl px-0.5 py-1.5 text-xs tracking-tight no-underline transition-colors sm:flex-row sm:gap-2.5 sm:px-3 sm:py-2 sm:text-base sm:tracking-normal ${
                        current
                          ? "bg-hover font-semibold text-ink"
                          : "text-ink-2 hover:bg-hover hover:text-ink"
                      }`}
                    >
                      <Icon className="size-4 flex-none" />
                      <span className="max-w-full truncate">{t(`nav.${section}`)}</span>
                    </a>
                  </li>
                );
              })}
            </ul>
          </nav>
          <div className="hidden border-t border-rule px-5 py-4 sm:block">
            <ShareState />
          </div>
        </div>
      </header>
      <main id="main" className="min-w-0 flex-1 px-4 pt-6 pb-16 sm:px-10 sm:pt-10">
        <div className="mx-auto max-w-[46rem]">
          <ConfigProblem />
          <Screen />
        </div>
      </main>
    </div>
  );
}
