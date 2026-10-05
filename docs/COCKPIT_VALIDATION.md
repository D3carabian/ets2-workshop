# Cockpit 界面与批量保存流程验收

基于 `571d417` 保留车库与驾驶员改进，在 `ui/cockpit` 实施；本轮按用户授权合并 main 并推送 GitHub，不发布 Release。

## 最终行为

- 深色 Cockpit 工作台、自定义标题栏、图标导航、三栏独立滚动与配件仪表。日间版采用白色工作区、深蓝候选/按钮与钢蓝当前参数；轨道加深以提高区分度。
- 设置卡片等高，上下边缘对齐。外观提供跟随系统、日间、夜间，首次默认跟随系统；系统主题变化即时生效，手工模式不被系统覆盖。本机 localStorage 记忆偏好，存储受限时仍可切换。
- 车库不再常驻变更清单。标题栏保存按钮显示待保存数量；悬停或键盘聚焦时以短动画显示最多三项预览，点击进入宽幅独立确认页。
- 确认页展示全部车辆和配件的前后变化，可移除单项，选择另存或备份覆盖，统一确认保存。失败时保留清单、手工名称与可读错误；空清单不能保存。
- 标题栏关闭和原生关闭请求都检查未保存状态。返回、关闭提醒、按 Esc 或点击遮罩均进入变更确认页；只有明确选择不保存并退出才丢弃。处理操作时禁止退出，避免中断写入。保存成功不会自动退出。
- 静态文本禁止选择，图片和链接禁止网页式拖动；输入框仍支持正常选择、复制和编辑。
- 保留驾驶员识别、姓名搜索、稳定分组、时间命名、自动存档防误触、候选再次点击取消，以及既有保存/备份/恢复保护。

不修改 Rust、Cargo 或配件/驾驶员业务规则。本轮新增窗口 destroy 权限只用于明确的不保存退出。没有加入新的游戏兼容承诺。

## 验证环境与边界

Windows / WebView2、Node.js 22.16.0、Rust MSVC。所有存档测试使用工作区 `verification/` 中的独立合成文件与 `ETS2_WORKSHOP_DATA_DIR`，不读写真实存档。原生测试隐藏启动，通过 CDP 操作；没有操作用户桌面的鼠标、系统快捷键或前台窗口。

| 检查 | 结果 |
|---|---|
| `scripts/build.ps1 -CheckOnly` | PASS：前端构建、18 项规则、98 项 Rust 测试（6 项忽略）、23 项打包与 10 项 Node 选择检查、许可来源和收集 |
| 依赖许可收集 | PASS，387 个依赖、626 份文本；三种本地字体锁定 5.3.0，包含 OFL 声明 |
| `cockpit-ui-smoke.mjs` | PASS：无底栏、候选键盘暂存、悬停预览、确认页、退出取消路径、两主题两尺寸、卡片对齐、系统变化/手工覆盖/重载记忆、文本选择、存储受限、英文标签 |
| `cockpit-native-theme.mjs` | PASS：真实 WebView 的主题/字体/状态保留、1080×720 / 1440×940、原生关闭请求被拦截、Esc 回清单、合成源文件不变 |
| `garage-usability-smoke.mjs` | PASS：1,000 / 5,000 车与驾驶员、分组搜索、候选与预览身份、时间命名、失败后名称及修改保持 |
| `garage-ui-smoke.mjs` | PASS：80 车两尺寸、独立滚动、换车搜索和跨页面暂存保持 |
| `p4-native-smoke.mjs` | PASS：已适配独立确认页；非法核心件操作说明和无写入、对比与原生布局 |
| `ui-smoke.mjs` | PASS：5 辆合成车、非法核心追加拒绝、另存、游戏存档复查、恢复；本轮使用合成明文，不把它记为二进制解密验收 |
| `recovery-smoke.mjs` | PASS：5,000 车、50 卡片分页、另存、未确认历史、临时文件、恢复和路径选择 |
| `language-ui-smoke.mjs` | PASS：主题之外的中英文切换继续保留清单、搜索、向导与手工名称 |
| `onboarding-smoke.mjs` / `review-state-smoke.mjs` | PASS：首次配置、官方工具校验、失败重试、旧目录只读、待修改期间禁止换游戏、设置写入故障保全 |

第一次 CheckOnly 因开发服务器占用 esbuild、第二次因原生测试进程占用 EXE 未完成；关闭本轮进程后完整重跑通过。原生脚本首次连接过早未发现页面，初始化后重跑通过。未用降低断言掩盖这些失败。

既有 smoke 原先有两处过时断言（非法核心件要求按钮禁用、提示必须包含“禁止追加”）；已改为核对实际拒绝提示及未新增待保存操作。保存页定位按用户本轮新流程更新，数据保护断言保留。

## 复现

```powershell
./scripts/build.ps1 -CheckOnly
cargo build --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml --bin ets2-workshop --example synthetic_catalog
# 以下两条在独立终端运行；纯合成预览不会连接真实后端。
npm run dev
node scripts/cockpit-ui-smoke.mjs

node scripts/garage-usability-smoke.mjs
node scripts/recovery-smoke.mjs
node scripts/language-ui-smoke.mjs
node scripts/p4-native-smoke.mjs
node scripts/onboarding-smoke.mjs
node scripts/review-state-smoke.mjs
```

原生主题/退出检查需要先运行 `garage-ui-fixture.mjs`，隐藏启动本次 EXE，以 `verification/garage-ui/app` 为数据目录、独立 WebView 用户目录和 CDP 9231；再运行 `cockpit-native-theme.mjs`。其他脚本按各自头注准备隔离实例。编译前关闭本轮测试 EXE，避免 Windows 文件占用。

原设计转换脚本及截图是用户提供的本地参考，不是重置最终代码的工具。日间/系统模式和独立确认页已按后续指令替代原稿部分要求；再次运行 apply-cockpit 会覆盖后续修复，不能用它复原当前实现。

## 独立审查与限制

全新上下文 reviewer 独立审查 PASS，无剩余 P1/P2/P3 finding。亲自后台验证原生关闭/丢弃、保存 busy 期间禁止丢弃、真实合成另存、合成源文件变化引起的保存失败后清单与名字保持、移除最后一项后禁止保存、系统主题实时变化与记忆，以及 1080×720 中英文深浅设置/确认页。

审查指出的模态键盘边界已修复并复查：标题栏语言获焦后，Tab/Shift+Tab 回到模态，Esc 仍按对应关闭规则处理。另补并复查恢复确认→退出提醒→返回变更单时清除旧恢复确认，避免两个模态叠加。截图与报告位于忽略的 `verification/independent-final-review/`；不提交存档、截图或本机路径。

原生窗口拖动、八方向边缘缩放和最大化仍未做前台手工验证；原生 close 事件已实测。部分窗口管理细节不包含在后台布局验收范围内。此前临时验证文件清理被自动审批以 `blocked by policy` 拒绝，未改用其他方式重试。
