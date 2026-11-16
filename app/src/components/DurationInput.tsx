import { useTranslation } from "react-i18next";
import type { Dur } from "../bindings/Dur";
import { type DurUnit, parseDur } from "../format";

export function DurationInput({
  value,
  onChange,
  units = ["m", "h", "d"],
  label,
  disabled,
}: {
  value: Dur;
  onChange: (next: Dur) => void;
  units?: DurUnit[];
  label: string;
  disabled?: boolean;
}) {
  const { t } = useTranslation();
  const parsed = parseDur(value) ?? { value: 15, unit: "m" };
  return (
    <span className="inline-flex gap-1.5">
      <input
        type="number"
        min={1}
        className="input data w-20 text-right"
        value={parsed.value}
        aria-label={label}
        disabled={disabled}
        onChange={(event) => {
          const n = Math.max(1, Math.round(Number(event.target.value) || 1));
          onChange(`${n}${parsed.unit}`);
        }}
      />
      <select
        className="select w-auto"
        value={parsed.unit}
        aria-label={t("duration.unit")}
        disabled={disabled}
        onChange={(event) => {
          const unit = units.find((candidate) => candidate === event.target.value);
          if (unit) onChange(`${parsed.value}${unit}`);
        }}
      >
        {units.map((unit) => (
          <option key={unit} value={unit}>
            {t(`duration.${unit}`, { count: parsed.value })}
          </option>
        ))}
      </select>
    </span>
  );
}
