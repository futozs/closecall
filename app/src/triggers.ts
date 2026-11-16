import type { TFunction } from "i18next";
import type { FiredTrigger } from "./bindings/FiredTrigger";
import type { Trigger } from "./bindings/Trigger";
import { formatDur } from "./format";
import { triggerDur, triggerKind } from "./model";

export function triggerLabel(t: TFunction, trigger: FiredTrigger, locale: string): string {
  switch (trigger.kind) {
    case "away":
      return t("fired.away", { duration: formatDur(trigger.away, locale) });
    case "idle":
      return t("fired.idle", { duration: formatDur(trigger.idle, locale) });
    case "item_age":
      return t("fired.item_age", { duration: formatDur(trigger.age, locale) });
    default:
      return t(`fired.${trigger.kind}`);
  }
}

export function whenLabel(t: TFunction, trigger: Trigger, locale: string): string {
  const dur = triggerDur(trigger);
  return t(`when_short.${triggerKind(trigger)}`, { duration: dur ? formatDur(dur, locale) : "" });
}
