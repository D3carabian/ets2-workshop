# 游戏配件目录性能（2026-10-03）

以下首轮数据对应 `6945e06` / `b13e0c7`；外部审查后的当前实现与新增定义见 [后续修复记录](EXTERNAL_REVIEW_FIXES.md)。

首轮优化保持 P4 配件范围、官方 DLC 扫描顺序、属性和双语名称不变。安装在本机的 24 个原版／已识别 DLC 归档共得到 26,382 个定义；新实现与优化前官方完整解包结果逐字段比较完全相同。

## 实测

同一台 Windows 机器、同一份已安装游戏、Rust debug 构建。下列“冷”指新的应用目录缓存，不代表操作系统文件缓存已清空；没有写游戏、玩家存档或生产应用缓存。

| 操作 | 用时 | 说明 |
| --- | ---: | --- |
| 优化前，已有完整解包缓存，重新扫描并解析名称 | 69.05 秒 | 不含官方解包器初次解包时间 |
| 优化后，全新应用缓存，从原始归档建立目录 | 13.80 秒 | 24 个包全部走只读内存路径；没有解包文件落盘 |
| 优化后，游戏资源未变化，再次建立／更新 | 0.263 秒 | 核对资源清单并读取已解析目录 |

原来的扫描本身就有两处反复编译正则：每个定义的 `@include` 和每个 `Document::parse` 的字段正则。现在使用相同表达式的 `OnceLock`，避免数万次重复编译。首次读取也不再为不使用的模型／贴图调用全量解包器，或落盘数万份小文件再读回来。

## 冷、热与资源变化行为

- 新缓存：有界读取 HashFS v2 的 `def`、`locale` 目录及定义引用到的其他文件；在内存中展开 include、解析属性与名称，只把完整目录结果原子写入本地缓存。
- 已有解包缓存但没有新版结果缓存：复用原来的 `.complete` 解包树，用相同的改进后解析器重建一次，之后使用结果缓存。
- 热更新：根据完整包清单、归档规范路径、长度和完整修改时间（不是整秒）检查缓存。`def.scs`、已识别 DLC 或 `locale.scs` 改变，以及 DLC 增加／删除，都会重建；更新过程中资源再次发生变化则拒绝保存本次结果。
- 语言读取失败：本次仍可按回退名称查看，但不缓存失败的目录结果；解除文件占用后点击更新会重新加载名称。
- 缓存损坏或版本不符：重新建立，不返回损坏结果。缓存版本独立于可读取的 `Catalog` 格式，旧目录依然能打开。独立审查后构建缓存版本升至 2，避免复用旧解析器可能漏掉嵌套引用的结果。
- HashFS v1、未知压缩／元数据、缺失目录项、非法目录或大小超限：该包回退到现有官方解包器，并报告具体原因。官方解包器的获取和校验流程没有绕过；只有实际回退时才获取并校验工具；原生读取和热缓存无需工具存在。

没有引入依赖，也没有把游戏定义或文本添加到仓库。当前选择范围仍是原版和既有识别规则下的官方 DLC，未扩展 Mod 支持。资源版本判断依赖文件系统长度与完整修改时间；人为修改内容后刻意还原相同时间和长度不在自动失效保证内。

## 正确性证据与复现

合成测试覆盖：存储／zlib 文件、无关模型纹理元数据、选中资源压缩或元数据不支持、缺失目录项、目录穿越、外部 include、已在内存中的 `.inc` 再引用树外文件、短暂语言文件锁定后的重试、DLC 本地语言覆盖、内存／磁盘两条路径的全字段一致性、冷／热构建、资源改变、DLC 增删、损坏缓存、`locale.scs` 增加、同一秒内修改时间变化。

实际游戏验收 `installed_catalog_performance_and_equivalence` 使用 P4 旧扫描得到的 `verification/p4-names/catalog.json` 作为基准。比较整个 `definitions` 对象，包括每一项路径、kind、unit、原始名称、双语名称、别名、类别、车型、来源、参数和兼容约束；同时比较冷／热完整目录相等。所有 26,382 个定义通过，不以数量相同代替字段一致。

```powershell
$env:ETS2_GAME_DIR = 'D:/Games/Euro Truck Simulator 2' # replace with your installed game
$env:ETS2_EXTRACTOR = 'D:/Tools/scs_extractor.exe' # verified official extractor
$env:ETS2_BASELINE_CATALOG = (Join-Path (Get-Location) 'verification/p4-names/catalog.json')
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib catalog::name_tests::installed_catalog_performance_and_equivalence -- --ignored --nocapture
```

验收创建独立 `verification/catalog-perf/selective-*` 目录，结束后自动清理，仅保留 `verification/catalog-perf/result.json` 的计时和等价结果。此性能用例默认忽略，普通 CI 不需要安装游戏。首次配置桌面回归使用合成 HashFS v2 直接读取；损坏头触发官方工具下载校验及失败重试。未把该测试称为 HashFS v1 成功解包验证。

格式核对来源：[SCS 官方解包器说明](https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Extractor)、[HashFS v2 目录格式参考](https://github.com/sk-zk/TruckLib.HashFs/blob/master/TruckLib.HashFs/HashFsV2Reader.cs)、[目录标记与常量](https://github.com/sk-zk/TruckLib.HashFs/blob/master/TruckLib.HashFs/HashFsV2/Consts.cs)。保留原 CityHash 许可归属。
