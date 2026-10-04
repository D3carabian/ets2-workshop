# ETS2 配件分类与 DAF 引擎标签研究

研究日期：2026-10-03。范围：配件分类的英文语义、简体中文映射的证据边界、DAF NGD/XD 引擎型号与功率显示。未修改源码。

## 分类映射建议

SCS 的 ETS2 官方介绍确认，卡车可选驾驶室尺寸、底盘、发动机、变速箱，并可安装侧裙、后视镜、灯具等外观元素及涂装；它没有公布中文菜单标签对照表。项目 `src-tauri/src/catalog.rs` 则从定义路径提取 `engine`、`transmission`、`chassis`、`cabin`、`interior`、`paint_job`、轮胎/轮毂等分类，并将 `/accessory/<类型>/` 下一级作为分类。以下中文是适合界面呈现的候选词；除“游戏支持简体中文”外，现有公开来源不足以证明这些词逐字等于官方简中 UI 标签。实现若要求“以官方名为准”，应优先读取运行时游戏本地化文本；没有对应本地化键时保留英文官方名或使用下表的透明回退译名，不要声称已核实为官方中文。

| 数据/官方英文名 | 建议简中显示名 | 证据与边界 |
| --- | --- | --- |
| `cabin` / Cabin | 驾驶室 | SCS 将其描述为 cabin size；“驾驶室”是行业通用译名，尚未从官方简中资源核对菜单用字。 |
| `chassis` / Chassis | 底盘 | SCS 明确列出 chassis；建议“底盘”。 |
| `engine` / Engine | 发动机 | SCS 明确列出 engine；建议“发动机”。 |
| `transmission` / Transmission | 变速箱 | SCS 明确列出 transmission；建议“变速箱”。 |
| `interior` / Interior | 内饰 | SCS 官方博客把 truck interior 作为一类可定制内容；建议“内饰”。 |
| `paint_job` / Paint job | 涂装 | SCS 官方使用 paint jobs；建议“涂装”，比“喷漆”更能覆盖图案/皮肤。 |
| accessory exterior / Exterior accessories | 外观配件 | SCS 官方描述侧裙、镜子、灯具等外观元素；分类名见游戏/社区常用分类，不足以确认官方简中 UI 字面。 |
| accessory interior / Interior accessories | 内饰配件 | SCS 官方使用 cabin accessories、interior accessories；建议“内饰配件”。 |
| `tank` / Tank | 油箱 | 数据路径可作分类依据；用户列举的主分类之外，保留子类名。 |
| `f_tire`, `r_tire` / tires | 前轮胎、后轮胎 | 文件路径区分前后轴时可分别显示；不要合并丢失信息。 |
| `f_disc`, `r_disc`, `f_hub`, `r_hub`, `f_nuts`, `r_nuts` | 前轮盘、后轮盘、前轮毂、后轮毂、前轮螺母、后轮螺母 | 项目路径中存在这些组件；中文是语义翻译，尚未核实官方 UI 分类词。 |
| unknown / 未识别 | 未知配件（或保留原始分类） | 不按猜测将零件归入外观/内饰。 |

注意：SCS 官方站的总述是功能类别说明，不一定等于改装店当前版本的导航标签。游戏会更新，分类显示名应可按本地化版本调整。

## DAF 引擎型号与功率

DAF 官方资料表明，型号数字是 kW 级别：MX-13 315 = 315 kW / 428 hp；MX-13 355 = 355 kW / 483 hp；MX-13 390 = 390 kW / 530 hp。DAF XD 官方规格也称 MX-11 220 为 220 kW / 299 hp。**这些型号后缀不能当作 hp。** 若 UI 显示 hp，应展示相应马力换算/官方 hp 值；若显示 kW，应附 `kW` 单位。

ETS2・steam Wiki 的当前配件/曲线表（曲线表标明对应 1.58）给出游戏模拟曲线峰值：MX-13 315 为 317 kW（431 PS），355 为 354 kW（482 PS），390 为 389 kW（529 PS）；MX-11 220 为 216 kW（294 PS）。这是游戏曲线数据，和 DAF 官方名义额定值有细小差异，应分字段记录，不能把模拟峰值误当命名后缀或反过来用。建议显示：保留官方模型名（如 `MX-13 315`），另显示游戏资源可验证的功率数字及单位；可同时标注“约 428 hp / 315 kW（型号额定值）”，但不要把 wiki 的 PS 峰值覆盖到官方型号名。项目环境另已实测本机 `def` 中 MX-13 315 的 `info` 字段为 `428 hp (315kW)`，峰值扭矩 2150 Nm、次级扭矩 2300 Nm；本项目当前应优先保留游戏资源明示的 `428 hp (315kW)`，不以宣传资料的 430 hp 或 Wiki 曲线峰值替换。

对于仅有 `ch 3`、`ch 3 bd`、`ch 3 cet ax`、`ch 3 hook` 等短 ID/文件名的条目，已确认它们位于本机 `/def/vehicle/trailer_owned/` 挂车底盘定义中；因此可按来源归入挂车底盘数据，不应当作卡车替换候选。现有证据仍无法确定各短 ID 的具体外观/配置含义，保留原始 ID/文件名并标“未识别配件”，不要推测展开。

## 数据策略

- 分类优先级：运行时官方本地化键/游戏内显示名（若可取得）→ 官方英文标签和语义明确的简中回退词 → 原始分类/ID。记录名称来源或置信状态，避免把回退翻译包装成官方字串。
- 从路径类别映射分类，从 `name`/定义字段或官方本地化 token 解析部件名称；不要从模糊文件名猜零件语义。若 `name` 是 `@@token@@` 且未解析，显示可读文件名作为回退并标记来源。官方 extractor 在本机无法提取 `locale.scs`（报无 root directory），所以当前应依据 `info` 等已验证字段和明确的名称规则工作；短期内无法依赖它解析全部本地化 token。
- 功率数据必须随数值保存单位和来源：`rated_power_kw`、`display_power_hp` 或类似独立字段；不要把型号中的数字解析为 hp，也不要混同 DAF 额定规格与 wiki 的游戏曲线峰值。
- 保留未知分类/名称的原始字符串，以便后续补充映射，不阻断查看其他可识别配件。

## 来源

- [SCS 官方 ETS2 产品介绍：车辆定制范围](https://eurotrucksimulator2.com/about.php) — cabin size、chassis、engine、transmission、外观附件及 paint jobs。
- [SCS 官方博客：Cabin Accessories Coming](https://blog.scssoft.com/2015/09/cabin-accessories-coming.html) — 官方 cabin accessories 术语。
- [ETS2・steam Wiki：パーツ別性能一覧（配件性能一览）](https://wikiwiki.jp/ets2steam/トラック/パーツ性能一覧) — DAF 引擎游戏内数据及 Wiki 的 PS 单位说明。
- [ETS2・steam Wiki：エンジン性能曲線図一覧（引擎性能曲线）](https://wikiwiki.jp/ets2steam/トラック/パーツ性能一覧/エンジン曲線一覧) — 曲线峰值，页面注明版本 1.58。
- [DAF 官方：PACCAR MX-13 engines 产品资料 PDF](https://www.daf.global/-/media/files/document-library/infosheets/engines/euro-6/mx-13/daf-paccar-mx-13-engines-en-my2025.pdf?rev=3fcc0879d57e4fef8c8265d2fde9c57c) — MX-13 315/355/390 名称对应额定 kW 与 hp。
- [DAF 官方：New Generation DAF XD 规格](https://www.daf.global/en-us/trucks/new-generation-daf-xd/efficiency-xd) — MX-11 220 = 220 kW / 299 hp。
- [Steam 商店简体中文页面](https://store.steampowered.com/app/227300/Euro_Truck_Simulator_2/?l=schinese) — 可确认商店/产品提供简体中文信息；页面不能单独证明上述分类是游戏内官方字面译名。



