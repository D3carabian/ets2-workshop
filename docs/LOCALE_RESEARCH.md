# ETS2 本机语言表读取验证

2026-10-03，针对本机 ETS2 1.61 安装进行只读验证。结论：`locale.scs` 能直接按已知路径读取；官方 extractor 的 `Root directory not found` 是因为归档没有空路径根目录，不能据此判断游戏文本不可用。无需全量解包游戏资产，也无需扫描或破解路径哈希。

## 实现与实测证据

生产模块是 `src-tauri/src/game_archive.rs`，API：

```rust
Archive::open(path: &Path) -> Result<Archive, String>
archive.read(path: &str) -> Result<Option<Vec<u8>>, String>
decode_text(data: &[u8]) -> Result<String, String>
```

`read` 对不存在的资源返回 `Ok(None)`；格式、边界或压缩错误返回 `Err`。支持路径开头的单个 `/`，拒绝反斜线、空路径组件、`.`、`..` 和 NUL。只通过 `File::open` 读取源文件，绝不改写游戏归档。文件偏移读取用互斥锁保护，解压在释放锁后执行。完整 2011 CityHash 支持超过 32 字节的路径。

本机输入为 游戏安装目录中的 `locale.scs`：

| 项目 | 实测值 |
| --- | --- |
| 文件大小 | 12,425,811 bytes |
| SHA256 | `2d142705d96839c373a3a380d307058ec03ee561f9e46ef76b995b40bc7c941a` |
| 头部 | `SCS#`, version `2`, salt `0`, hash `CITY` |
| entry 数量 | 522 |
| 压缩类型 | 522 个全部为 zlib（`0x10`） |
| 空路径根目录 | 不存在 |
| `locale` 目录 | 存在，可枚举语言目录 |
| entry index | offset 12,414,928，压缩 5,693 bytes，展开 8,352 bytes |
| metadata index | offset 12,420,624，压缩 5,187 bytes，展开 10,440 bytes |

两种语言当前文件均为 UTF-8 明文，部分文件含 UTF-8 BOM。目标文件没有 `3nK`、`ScsC` 或 `BSII` 编码。无需给应用增加这些解码器。

## 入口与真实路径

`local.sii` 本身只有 6 个有效货币/分隔符键；它通过 include 引入真正的 UI 和配件名称表。目录中的 `mails.sii`、`driver_names.sii` 不属于配件字典。

两种语言的 `local.sii` 都先引用绝对虚拟路径 `/locale/common.sui`，再顺序引用相对路径：`localization.sui`、`photoalbum.sui`、`loading_screens.sui`、`academy.sui`、`tutorial_hints.sui`、`paintjobs.sui`、`accessories.sui`。按入口的 include 顺序展开，每种语言得到 **12,514 个有效唯一 key**。这里没有把 `local.steam.sii` 混入主词表；它是独立的 Steam Input 文本，当前有 451 个有效 key。`local.override.sii` 当前为空（0 个 key）；读取它可为后续版本留出明确扩展点，但本次无法从归档文本证明游戏加载它与所有 DLC/模组的完整优先级。

| 路径 | SCS CityHash64 | 解压 bytes | 有效 key |
| --- | --- | ---: | ---: |
| `locale/common.sui` | `d82d2dfa9fa6bb22` | 23,232 | 356 |
| `locale/en_gb/local.sii` | `a79d5f20eb9c2035` | 608 | 6 |
| `locale/zh_cn/local.sii` | `8f5f2e976fff7fc0` | 608 | 6 |
| `locale/en_gb/localization.sui` | `1403ea054badd153` | 620,118 | 5,747 |
| `locale/zh_cn/localization.sui` | `c65d91e1472b2fb4` | 602,780 | 5,747 |
| `locale/en_gb/accessories.sui` | `11ee88fecc2cf04d` | 277,018 | 3,274 |
| `locale/zh_cn/accessories.sui` | `0733159880fcf154` | 274,941 | 3,274 |
| `locale/en_gb/paintjobs.sui` | `645df936321e519c` | 37,682 | 689 |
| `locale/zh_cn/paintjobs.sui` | `de46757db6cd68c7` | 37,251 | 689 |

这些 hash 可作实机回归向量；实现应计算路径 hash，而非锁定当前目录结构。`localization.sui` 里另有 506 个注释掉的旧 key，不能用不区分注释的全局正则把它们算进字典。早期探查的 6,253 是包括这些注释的错误计数；生产解析应为 5,747。

## HashFS v2 读取要点

下面的字段已由本机归档验证，并与开源 reader 交叉核对；没有从外部下载或执行二进制。

1. 52-byte 小端 header：magic 位于 0，version u16 位于 4，salt u16 位于 6，hash method 四字节位于 8；entry_count/u32、entry_compressed_size/u32、metadata_word_count/u32、metadata_compressed_size/u32 分别在 12/16/20/24；两个 u64 索引偏移在 28/36。44 处的 security offset 不参与资源读取。
2. 两张索引分别按 zlib 展开，严格检查展开长度：`entry_count * 16` 与 `metadata_word_count * 4`。
3. 每个 16-byte entry 是 hash/u64、metadata_word_index/u32、part_count/u16、flags/u16。对于每个 metadata descriptor/u32，低 24 位是后续 metadata header 的 **u32 word 偏移**；高字节标示类型，bit `0x80` 表示数据描述。
4. 16-byte 数据描述：第一个 u32 的低 **28** 位是压缩长度，高 4 位是压缩方法；第二个 u32 的低 28 位是解压长度，其高 4 位另作 flags；第 12 字节起的 u32 乘以 16 得到 payload 绝对偏移。注意 `scs_tools` 文档/实现只取低 24 位，不能原样搬用尺寸逻辑；[TruckLib 的 MainMetadata](https://github.com/sk-zk/TruckLib.HashFs/blob/master/TruckLib.HashFs/HashFsV2/MainMetadata.cs)明确保留第 4 字节低 4 位。本机语言文件均小于 16 MiB，因此两种算法在本样本上看似一致。
5. 对已知路径（正斜线、无起始 `/`）计算 SCS 使用的 **2011 CityHash64**，通过索引查 payload。现代通用 CityHash 实现不能默认互换。
6. 只读这次请求涉及的 payload。`0x00` 原样读取，`0x10` zlib；其他方法明确拒绝。目录树完全不是按已知路径读取的前提。若需要目录枚举，本机 listing 格式为 u32 child_count、每个 child 的 u8 字节长度、拼接的名称；名称以 `/` 开头表示子目录。

生产模块给 archive 长度设 128 GiB 上限、entry_count 2,000,000 上限、压缩及展开索引各 128 MiB 上限、单文件压缩与展开数据各 32 MiB 上限；每次 range 都检查文件长度。zlib 用声明长度加一的有界 reader，长度不匹配就失败，不会无限解压。`GDeflate`、带盐 hash、HashFS v1、ZIP 和编码 SII 都不在此模块的支持范围。主程序应保留明确回退状态，不能把拒绝的格式当成空词表。

## 名称解析接入

先保留配件定义的 `name` 原文和 token。运行时读取安装目录的 `locale.scs`，分别建立 `en_gb` 与 `zh_cn` 字典；英文作为简中不存在时的后备。`@@token@@` 可以嵌在普通字符串中，也可以出现在字典值中，因此要有界递归替换并检测循环，不能只匹配整串。当前主词表中有 720 个英文、721 个简中值包含 `@@`。

SII 解析需处理 `key[]` / `val[]` 以及有索引的数组形式、注释、引号转义和 `\xHH`（字节转义，先组装字节再解 UTF-8）。include 需相对当前虚拟目录解析；根路径从归档根解析；设置深度、文件数量、累计展开字节上限，拒绝路径越界和循环。入口已经给出需要的文件，不必把所有语言都提取到磁盘。

以游戏给出的文字为准：简中表也有未译的品牌/风格词，例如 `Aero`、`Duty`、`Individual Lion S`；它们依然属于游戏内名称，不能因文字是英文就当作未解析。缺 key 或未支持的归档应保留可审查的 fallback/source 字段，不能把文件名推断声称为官方原文。缓存需要包含游戏路径、`locale.scs` 指纹和解析版本，否则游戏更新后会继续显示旧翻译。完整 mod/DLC 覆盖优先级需要独立实现游戏挂载顺序；本次验证的是已安装本体语言归档，不能声称完全复现所有 mod 文本。

## 已核对的官方简中词

下表来自上述本机归档，不是从日文 wiki 翻译所得。

| key | English | 简体中文 |
| --- | --- | --- |
| `engine` | Engine | 发动机 |
| `transmission` | Transmission | 变速器 |
| `chassis` | Chassis | 底盘 |
| `cabin` | Cabin | 驾驶室 |
| `interior` | Interior | 内饰 |
| `accessory_t` | ACCESSORIES | 配件 |
| `sideskirt` | Sideskirts | 侧裙 |
| `sunshield` | Sun Visor | 遮阳板 |
| `f_bumper` / `r_bumper` | Front / Rear Bumper | 前保险杠 / 后保险杠 |
| `f_grill` | Bull Bar | 防撞杠 |
| `exhaust` | Exhaust | 排气管 |
| `mirror` | Main Mirrors | 主后视镜 |
| `f_wheel` / `r_wheel` | Front / Rear Wheels | 前轮 / 后轮 |
| `rim` | Rim | 轮辋 |
| `tire` | Tyre | 轮胎 |
| `steering_w` | Steering Wheel | 方向盘 |
| `paint_job` | Stock colour | 原厂配色 |
| `paint_cat` | Paint | 喷漆 |
| `paintjob_t` | BLUEPRINTS | 蓝图 |
| `sideskirt_painted` | Painted | 喷漆 |
| `sunshield_standard` | Standard | 标准 |

类别 token 与定义目录名不总是一一对应。例如 `f_rim` 不在主词表中，而 `rim` 存在；`f_grill` 的实际游戏词是“防撞杠”，不能机械译为“前格栅”。`paint_job` 的实际文本是“原厂配色”，不应不加语境地用作整个涂装区的标题。

## 外部交叉核对与许可证

- [SCS 官方 Game Archive Packer](https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Packer)：确认游戏 1.50 引入 HashFS v2，官方 packer 支持 v1/v2 解包。其公开文档没有提供逐字段格式规范；本研究没有下载或执行新工具。
- [scs_tools 格式说明](https://github.com/Wilps93/scs_tools/blob/e147e755778579e78f80a406a53c64515113d7a1/FORMAT.md)和[CityHash 源码](https://github.com/Wilps93/scs_tools/blob/e147e755778579e78f80a406a53c64515113d7a1/src/cityhash.rs)：定位无根目录与旧版 hash 变体。生产模块仅移植 CityHash；该仓库 MIT 许可和底层 Google CityHash MIT 通知存于 `licenses/locale-cityhash-MIT.txt`。不要把其整个 reader 直接作为精确格式规范（28-bit size 差异已记录）。
- [TruckLib.HashFs](https://github.com/sk-zk/TruckLib.HashFs)：交叉核对数据描述的 28-bit 长度。仓库许可证为 GPL-2.0；本次只核对格式，没有复制它的代码或引入依赖。
- [scs_tools 第三方来源](https://github.com/Wilps93/scs_tools/blob/e147e755778579e78f80a406a53c64515113d7a1/THIRD_PARTY_NOTICES.md)列出 Archive::SCS、NVIDIA GDeflate、SII_Decrypt 等；它们未纳入本次实现。通用解码器会扩大依赖与许可范围，当前明文/zlib样本没有这个必要。
- 用户指定的[ETS2・steam Wiki](https://wikiwiki.jp/ets2steam/)是日文社区资料。[底盘说明](https://wikiwiki.jp/ets2steam/%E3%83%88%E3%83%A9%E3%83%83%E3%82%AF)可辅助核对 4x2 等底盘语义；[配件性能表](https://wikiwiki.jp/ets2steam/%E3%83%88%E3%83%A9%E3%83%83%E3%82%AF/%E3%83%91%E3%83%BC%E3%83%84%E6%80%A7%E8%83%BD%E4%B8%80%E8%A6%A7)当前自述按 Ver.1.58 整理，可辅助核对车型、发动机、变速器等资料，不能替代本机 1.61 的中文原文或保证所有配件兼容性。

## 验证和复现

`verification/locale_probe.py` 是只用 Python 标准库的只读研究原型。它支持本任务所需不超过 32 字节的 hash 路径，递归展开入口并报告 hash/长度/key 数；生产 Rust 实现没有此路径长度限制。默认只打印统计，不落地游戏文本。可用 `--out` 将字典写到调用者指定位置供本机调试，不应提交或随应用打包完整游戏词表。

```powershell
python verification/locale_probe.py "$env:ETS2_LOCALE_ARCHIVE"

# 在 src-tauri 下运行；需要项目要求的 Rust 工具链
cargo test --lib game_archive
$env:ETS2_LOCALE_ARCHIVE = Join-Path $GameDirectory 'locale.scs' # GameDirectory 指向本机游戏安装目录
cargo test --lib game_archive::tests::installed_locale_matches_known_paths -- --ignored --nocapture
```

已通过：5 个合成/算法测试（另 1 个实机测试默认忽略），以及显式运行的 1 个本机归档测试。实机测试读取两种语言四个主要文件及 common，并验证明文 `key[]` 结构；没有提交解压游戏资源。合成测试覆盖无根目录按路径读取、store/zlib、UTF-8 BOM、缺路径、错误版本/盐/偏移/长度、未支持压缩、错误展开长度、有界解压和编码文本拒绝。已有 `decrypt_truck` 依赖的四个 warning 与本模块无关。
