# 外部审查后续修复

基线：`b13e0c7`。本轮按读取与缓存、配置与编辑状态、内部简化三批处理外部审查反馈。保留备份、原子替换、写前/写后校验、恢复记录、解码隔离与工具完整性校验，没有扩展 Mod 或改装类别。

## 问题与验收对应

| 外部审查问题 | 当前行为 | 回归入口 |
| --- | --- | --- |
| DLC 语言目录枚举失败被忽略 | 可选目录不存在可跳过，其他目录/文件/目录项失败显示来源；语言失败的结果不进入热缓存，修复后可重试 | `catalog::name_tests::partial_disk_sources_report_errors_and_retry_without_hot_cache`、`disk_read_and_directory_locks_report_then_recover` |
| 注释进入字段、注释/字符串里的 include 被执行 | 一个扫描器识别字符串、转义、三种注释和逻辑行；字段与 include 使用同一边界。按原始偏移修改，保留注释、未知字段和 CRLF | `sii::tests`、`localization::tests::comments_and_multiline_strings_do_not_request_missing_includes`、catalog 内存/磁盘等价用例 |
| 重建与编辑前过期标准不同 | 两者共用完整可识别包集合、canonical 路径、长度及精确修改时间；目录记录解析版本 4 和完整性。旧/缺元数据目录可查看，后端拒绝修改并提示重建 | `selective_build_cache_handles_hot_changed_added_removed_and_corrupt_inputs`、真实桌面旧目录只读 |
| 配置/目录更新绕过可靠发布 | setup、游戏切换和 index 共用新目录代际→原子切换 settings→清理旧代际。任何发布失败前不改变 Engine | setup 事务/锁定/发布失败测试、`review-state-smoke.mjs` |
| 切换游戏后保留旧会话与操作 | 有待保存操作时前后端拒绝切换、重新配置和重建；成功切换同时清空会话与编辑状态。仅改扫描目录保留原编辑会话 | main settings 状态测试、桌面两项待保存操作与失败/成功切换 |
| 附件 ID 随无关队列位置变化 | ID 使用源文档、目标车辆和供体，不含队列位置。移除被后续操作依赖的追加项时说明需先移除依赖项 | `fleet_scale::removing_unrelated_operation_keeps_added_accessory_identity` |
| 文件全读完才检查大小 | 文件打开、解码 worker、Session 创建及恢复备份使用同一 LIMIT+1 受限读取；Session 创建前核对 game/info | `decoder::tests::bounded_reader_consumes_at_most_limit_plus_one`、storage 元信息读取/竞争测试 |

## 小范围简化

- `Session` 不再含不可能为真的 `mods`，入口继续拒绝 Mod；运行中 `info_hash` 必需，历史 `Receipt` 的 Option/default 兼容保留。
- `Source` 统一内存与磁盘读取，include 当前包优先，再逆序查此前来源；当前包有读取错误时不回退到旧内容。跨包树外文件仍按精确路径、有界读取。
- 内部编辑 action 先转成两种有效形状，适配判定复用同一规则；没有引入新的 RPC 框架或规则引擎。
- 存档发现仅要求所选根目录可读；次级 Steam、profile、slot 局部失败返回可用列表和警告。再次刷新清除已恢复的警告。
- 原生归档读取和热缓存不准备解包器，实际回退时才下载、验证并执行官方工具。

覆盖/恢复仍要求游戏、同步与其他程序停止写入该槽；默认另存新档。README 中英文及操作界面明确这个边界，不声称文件 hash 检查能消除跨程序并发窗口。

## 真实资源验证的发现

把旧的静默跳过改成可见诊断后，首次本机扫描暴露 86 个定义读取问题：DLC 引用了本体公共文件，以及正式内饰定义重复引用仪表字段。已补齐跨包 include；只读定义解析保留重复字段，但所有实际消费的名称、指标、外观资源及兼容参数若重复且值不同仍拒绝。存档解析与任何 patch 仍拒绝重复字段，未猜测冲突值的 first/last-wins 语义。

同一份已安装资源最终得到 **26,460** 条定义，比此前 26,382 条补回 78 条可识别配件定义；其余原先失败文件不是配件定义。`scan_complete=true`，双语名称 schema 完整，唯一提示为原有支持范围说明。新旧计数不再被用作“必须完全相等”的验收，新增定义是修复发现的结果。

## 验证与独立审查

独立上下文 reviewer 阅读原外部报告、当前补丁和测试，逐项核对后结论 **PASS**；没有以先前的三项问题复审代替本轮验收。该 reviewer 的本轮验收为源码复审，实际运行由主 agent 完成。

已执行实际桌面：`review-state-smoke.mjs`（旧目录只读、两项操作阻切、settings 最后发布失败保留旧数据和会话、UI 真正切换后清会话）、`p4-native-smoke.mjs`（配件选择、语言和保存）、`save-performance-smoke.mjs`（筛选、缓存、会话与偏好）、`onboarding-smoke.mjs`（官方工具下载校验、坏包拒绝、v2 直接读取、配置失败重试/重启）。`language-ui-smoke.mjs` 验证语言切换及待保存状态。更新后的 `recovery-smoke.mjs` 通过 5,000 辆合成车库的另存/恢复、待确认历史及路径选择验证（打开约 832 ms，界面只渲染 50 张卡片）。

脚本先构建 custom-protocol EXE 和开发用 `synthetic_catalog` example。该 example 只接受工作区 `verification` 下有固定合成安装标记的 fixtures，用生产指纹 API 绑定元数据；不添加生产环境绕过 freshness 的开关，不进入发布包。

完整统一检查 `scripts/build.ps1 -CheckOnly` 已通过：89 项 Rust 测试、15 项前端规则测试、10 项 Node 选择器与 21 项打包检查；4 个依赖本机安装的 Rust 用例默认忽略。来源许可和完整依赖许可校验通过，依赖锁文件未变化。

本机当前 CLI 从原始归档建立完整目录约 12.57 秒，重复调用约 0.68 秒（含进程启动与输出目录文件写入，操作系统文件缓存未清空）；存档重复刷新 58 ms，默认排除 autosave 11 ms，隐藏再显示 62 ms。真实资源扫描只读原游戏文件；所有配置、存档写入与失败注入只针对隔离合成数据。测试产物在被 Git 忽略的 `verification`。
