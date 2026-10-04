import type { Definition } from "./types";
import { primaryMetrics, metricLabels } from "./parts";

export function Metrics({ def }: { def: Definition | null | undefined }) {
  if (!def)
    return <p className="micro">参数未知。该配件尚未索引，原始数据会保留。</p>;
  const rows = primaryMetrics(def);
  return (
    <>
      <dl className="metrics">
        {rows.map(([key, value]) => (
          <div key={key}>
            <dt>{key}</dt>
            <dd title={value}>{value}</dd>
          </div>
        ))}
      </dl>
      {!rows.length && <p className="micro">此配件未提供性能参数。</p>}
      {def.category === "engine" && (
        <p className="micro">
          马力与转速范围采用游戏标称值；扭矩采用定义参数。
        </p>
      )}
      <details>
        <summary>全部定义参数</summary>
        <dl className="metrics">
          {Object.entries(def.metrics).map(([key, value]) => (
            <div key={key}>
              <dt>{metricLabels[key] || key}</dt>
              <dd>{value}</dd>
            </div>
          ))}
        </dl>
      </details>
      <details className="part-source">
        <summary>数据来源与定义</summary>
        <p>来自本机游戏定义 · {def.source}</p>
        <code className="break">{def.path}</code>
        <p>原始名称：{def.name}</p>
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
  if (!after) return <Metrics def={after} />;
  const old = new Map(before ? primaryMetrics(before) : []);
  return (
    <div className="comparison">
      <div className="comparison-head">
        <span>关键属性</span>
        <span>当前</span>
        <span>候选</span>
      </div>
      {primaryMetrics(after).map(([key, value]) => (
        <div className="comparison-row" key={key}>
          <span>{key}</span>
          <span>{old.get(key) || "未知"}</span>
          <strong>{value}</strong>
        </div>
      ))}
      <details>
        <summary>候选配件来源</summary>
        <p>{after.source}</p>
        <code className="break">{after.path}</code>
        <p>原始名称：{after.name}</p>
      </details>
    </div>
  );
}
