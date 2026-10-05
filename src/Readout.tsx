// src/Readout.tsx  (new file, copy verbatim)
// Gauge + metric bars at the top of the inspector.
// Shows the current part's primary value; when a candidate is selected it
// overlays the candidate (amber) and prints the delta.
import { useLanguage } from "./i18n";
import type { Definition } from "./types";
import { primaryMetrics, metricValue } from "./parts";

/** Gauge full-scale per category. Values above it extend the scale. */
const GAUGE_MAX: Record<string, number> = {
  engine: 800,
  transmission: 16,
  tank: 1600,
  chassis: 1600,
};

/** Returns the single number in a metric string, or null for unknown/list/range values. */
export function metricNumber(value: string | undefined): number | null {
  if (!value || value === "未知") return null;
  const plain = value.replace(/(\d),(\d{3})\b/g, "$1$2");
  const numbers = plain.match(/-?\d+(?:\.\d+)?/g);
  if (!numbers || numbers.length !== 1) return null;
  if (/\d\s*[-–]\s*\d/.test(plain)) return null;
  return Number(numbers[0]);
}

function splitValue(shown: string) {
  const m = shown.match(/^\s*(-?[\d.,]+)\s*(.*)$/);
  return m ? { num: m[1], unit: m[2] } : { num: shown, unit: "" };
}

const fmt = (n: number) =>
  Number.isInteger(n)
    ? n.toLocaleString("en-US")
    : n.toLocaleString("en-US", { maximumFractionDigits: 2 });

// Geometry: viewBox 0 0 190 112, centre (95,96), radius 78, 180° sweep.
function arc(value: number, max: number) {
  const ratio = Math.max(0, Math.min(value / max, 1));
  const a = Math.PI * (1 - ratio);
  const r = 78;
  const cx = 95;
  const cy = 96;
  const x = cx + r * Math.cos(a);
  const y = cy - r * Math.sin(a);
  return `M ${cx - r} ${cy} A ${r} ${r} 0 0 1 ${x.toFixed(2)} ${y.toFixed(2)}`;
}

export function Readout({
  category,
  before,
  after,
}: {
  category: string;
  before: Definition | null | undefined;
  after: Definition | null | undefined;
}) {
  const { t, locale } = useLanguage();
  const partLocale = locale === "en" ? "en" : "zh";
  if (!before || !(category in GAUGE_MAX)) return null;

  const beforeRows = primaryMetrics(before);
  const afterMap = new Map(after ? primaryMetrics(after) : []);
  const rows = beforeRows
    .map(([label, value]) => ({
      label,
      value,
      cur: metricNumber(value),
      cand: after ? metricNumber(afterMap.get(label)) : null,
      candValue: afterMap.get(label),
    }))
    .filter((r) => r.cur !== null)
    .slice(0, 3);
  if (!rows.length) return null;

  const main = rows[0];
  const base = GAUGE_MAX[category];
  const peak = Math.max(main.cur!, main.cand ?? 0);
  const step = base / 8;
  const max = peak > base ? Math.ceil(peak / step) * step : base;
  const unit = splitValue(
    metricValue(
      main.cand !== null ? main.candValue! : main.value,
      partLocale,
    ),
  ).unit;
  const bigNumber = fmt(main.cand ?? main.cur!);
  const delta = main.cand !== null ? main.cand - main.cur! : null;
  const deltaClass =
    delta === null || delta === 0 ? "" : delta > 0 ? "up" : "down";
  const deltaText =
    delta === null
      ? ""
      : `${delta > 0 ? "+" : delta < 0 ? "−" : "±"}${fmt(Math.abs(delta))}${unit ? " " + unit : ""}`;

  return (
    <>
      <div className="readout">
        <svg className="gauge" viewBox="0 0 190 112" aria-hidden="true">
          <path
            className="gauge-track"
            d={arc(max, max)}
            strokeWidth={10}
            fill="none"
            strokeLinecap="round"
          />
          <path
            className="gauge-current"
            d={arc(main.cur!, max)}
            strokeWidth={10}
            fill="none"
            strokeLinecap="round"
          />
          {main.cand !== null && (
            <path
              className="gauge-candidate"
              d={arc(main.cand, max)}
              strokeWidth={3}
              fill="none"
              strokeLinecap="round"
            />
          )}
          <text className="gauge-tick" x="17" y="110">
            0
          </text>
          <text className="gauge-tick" x="173" y="110" textAnchor="end">
            {fmt(max)}
          </text>
          <text className="gauge-label" x="95" y="86" textAnchor="middle">
            {t(main.label)}
          </text>
        </svg>
        <div className="readout-values">
          <span className="readout-label">
            {main.cand !== null ? t("当前 → 候选") : t("当前")}
          </span>
          <span className="readout-big">
            {bigNumber}
            {unit && <small>{unit}</small>}
          </span>
          {delta !== null && (
            <span className={"readout-delta " + deltaClass}>
              {deltaText}
              <span>
                {t("原 {value}", { value: metricValue(main.value, partLocale) })}
              </span>
            </span>
          )}
        </div>
      </div>
      <div className="readout-bars">
        {rows.map((r, i) => {
          const scale =
            i === 0 ? max : Math.max(r.cur!, r.cand ?? 0) * 1.15 || 1;
          return (
            <div className="readout-bar" key={r.label}>
              <span className="bar-label">{t(r.label)}</span>
              <span className="bar-track">
                <i
                  className="bar-current"
                  style={{ width: `${(r.cur! / scale) * 100}%` }}
                />
                {r.cand !== null && (
                  <i
                    className="bar-candidate"
                    style={{ width: `${(r.cand / scale) * 100}%` }}
                  />
                )}
              </span>
              <span className="bar-values">
                {fmt(r.cur!)}
                {r.cand !== null && (
                  <>
                    {" → "}
                    <b>{fmt(r.cand)}</b>
                  </>
                )}
              </span>
            </div>
          );
        })}
      </div>
    </>
  );
}
