import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import en from "../../locales/en/ui.json";
import hu from "../../locales/hu/ui.json";
import type { Language } from "./bindings/Language";

export type Locale = "en" | "hu";

export function systemLocale(languages: readonly string[] = navigator.languages): Locale {
  const first = languages[0]?.toLowerCase() ?? "en";
  return first.startsWith("hu") ? "hu" : "en";
}

export function resolveLocale(language: Language): Locale {
  return language === "system" ? systemLocale() : language;
}

export async function initI18n(locale: Locale) {
  await i18next.use(initReactI18next).init({
    resources: { en: { ui: en }, hu: { ui: hu } },
    lng: locale,
    fallbackLng: "en",
    defaultNS: "ui",
    ns: ["ui"],
    interpolation: { escapeValue: false },
  });
  document.documentElement.lang = locale;
}

export async function setLocale(locale: Locale) {
  await i18next.changeLanguage(locale);
  document.documentElement.lang = locale;
}

export { i18next };
