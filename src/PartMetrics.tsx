import { useLanguage } from "./i18n";
import type { Definition } from "./types";
import { primaryMetrics, metricLabels, metricValue } from "./parts";

export function Metrics({ def }: { def: Definition | null | undefined }) {
  const { t, locale } = useLanguage();
  const partLocale = locale === "en" ? "en" : "zh";
  if (!def)
    return (
      <p className="micro">{t("参数未知。该配件尚未索引，原始数据会保留。")}</p>
    );
  const rows = primaryMetrics(def);
  return (
    <>
      <dl className="metrics">
        {rows.map(([key, value]) => (
          <div key={key}>
            <dt>{t(key)}</dt>
            <dd title={value}>{metricValue(value, partLocale)}</dd>
          </div>
        ))}
      </dl>
      {!rows.length && <p className="micro">{t("此配件未提供性能参数。")}</p>}
      {def.category === "engine" && (
        <p className="micro">
          {t("马力与转速范围采用游戏标称值；扭矩采用定义参数。")}
        </p>
      )}
      <details>
        <summary>{t("全部定义参数")}</summary>
        <dl className="metrics">
          {Object.entries(def.metrics).map(([key, value]) => (
            <div key={key}>
              <dt>{t(metricLabels[key] || key)}</dt>
              <dd>{metricValue(value, partLocale)}</dd>
            </div>
          ))}
        </dl>
      </details>
      <details className="part-source">
        <summary>{t("数据来源与定义")}</summary>
        <p>
          {t("来自本机游戏定义")} · {def.source}
        </p>
        <code className="break">{def.path}</code>
        {def.name_alias && (
          <p>{t("旧名称键缺失；名称已按同模型、同图标的游戏配件核对。")}</p>
        )}
        <p>
          {t("原始名称")}：{def.raw_name || def.name}
        </p>
      </details>
    </>
  );
}

export function Comparison({
  before,
  after,
}: {
  before: Definition | null | undefined;
  after: Definition | null | undefined;
}) {
  const { t, locale } = useLanguage();
  const partLocale = locale === "en" ? "en" : "zh";
  if (!after) return <Metrics def={after} />;
  const old = new Map(before ? primaryMetrics(before) : []);
  return (
    <div className="comparison">
      <div className="comparison-head">
        <span>{t("关键属性")}</span>
        <span>{t("当前")}</span>
        <span>{t("候选")}</span>
      </div>
      {primaryMetrics(after).map(([key, value]) => (
        <div className="comparison-row" key={key}>
          <span>{t(key)}</span>
          <span>{metricValue(old.get(key) || "未知", partLocale)}</span>
          <strong>{metricValue(value, partLocale)}</strong>
        </div>
      ))}
      <details>
        <summary>{t("候选配件来源")}</summary>
        <p>{after.source}</p>
        <code className="break">{after.path}</code>
        <p>
          {t("原始名称")}：{after.raw_name || after.name}
        </p>
      </details>
    </div>
  );
}
