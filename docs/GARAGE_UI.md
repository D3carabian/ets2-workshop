# 车库滚动与配件展示（2026-10-03）

本次基于阶段一提交 `7797147`，分支 `codex/garage-usability`。用户要求等待阶段三合并后再评估性能，因此没有修改 Session、车辆枚举、大列表渲染策略或 Rust 后端。

## 行为与合并边界

- 车库列表、当前车辆配件列表、右侧详情各自在固定高度区域内滚动；选择靠下的车辆保留左侧滚动位置，右侧从新车辆的详情顶部开始。
- 显示品牌所属车型、可识别型号、游戏标称 hp、定义扭矩；变速箱显示挡位、主减速比和缓速器，油箱显示定义容量。搜索同时匹配显示文本和原始标识，缺失值显示未知。
- 右侧默认折叠当前件的详细参数，优先显示候选；选择候选后可对照当前与候选参数。资源包名、文件路径和原始字段可展开查看。
- 分类按驾驶室、底盘、发动机、变速箱、内饰、涂装、外观配件、内饰配件组织；未映射的类别保留在“其他 / 未识别”。只是显示分组，不更改操作传给后端的类别或校验规则。
- 卡车配件库排除挂车、AI、货物定义；保留共享轮胎、轮盘等资源。此筛选只约束界面候选，不代表完成了后端兼容性判断。
- 主要新增 `src/parts.ts`、`src/PartMetrics.tsx`、`src/garage.css`，在 `App.tsx` 局部接入。未修改首次向导、RPC 格式、保存弹窗和恢复实现。合并阶段三时应保留它对保存、历史与恢复状态的修改，再重跑下面的桌面检查。

## 名称与数据边界

来源核对见 [配件研究](PART_LABEL_RESEARCH.md)。数字后缀不作为马力推断依据。DAF MX-13 315 的显示采用本机 `info` 的 428 hp，不用 Wiki 曲线峰值替代。203 个本机卡车引擎定义均能提取显式 hp；这是本机目录检查，不代表所有游戏版本都具备该字段。

Mercedes-Benz New Actros 的 `engine_1842/1845/1848/1851` 经本机 1.61 原始名称 token 核对为 OM 471 Euro VI，`engine_1852/1858/1863` 为 OM 473 Euro VI。回退映射限定精确路径，且要求 `info` 的 kW 与所核对版本一致；原始 token 形如 `@@engine_om_471_euro_vi_375@@`。这避免把 `engine_1851` 当作发动机商业型号，也不将未知编号泛化映射。

官方解包工具未能提取本机 `locale.scs`，所以本次没有实现完整本地化字典。公开来源也不足以逐字核实所有官方简中菜单用字。已有可读名称保留；其余显示部件类别及原始变体，并标注“游戏名称未解析”。后续可在目录层保留原名称 token，再接入经核对的本地化名称。不会把回退译名声称为官方中文原文。

## 验证与复现

使用项目 Node.js 22 环境：

```powershell
node --test scripts/parts.test.mjs
npm run build
node scripts/garage-ui-fixture.mjs
npm run dev
```

在另一个 PowerShell 中运行当前基线的 debug Tauri 程序，指定隔离数据：

```powershell
$env:ETS2_WORKSHOP_DATA_DIR = Join-Path $PWD 'verification/garage-ui/app'
$env:WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS = '--remote-debugging-port=9231'
$env:WEBVIEW2_USER_DATA_FOLDER = Join-Path $PWD 'verification/garage-ui/webview'
Start-Process -FilePath './src-tauri/target/debug/ets2-workshop.exe' -WindowStyle Hidden
node scripts/garage-ui-smoke.mjs
```

需先拥有 debug Tauri 可执行文件；本次复用同一基线已经构建的 debug 后端，页面由当前 worktree 的 Vite 提供，没有重新打包发布版。

验证结果：8 项名称与分类测试、TypeScript/Vite 构建通过。实际 WebView 使用 80 辆完全合成车辆，在 1440×940 和 1080×720 两种视口通过独立滚动、底部选车、马力搜索、候选属性、挂车过滤、分类分组、切车后保留待修改操作检查，无页面异常。预览前后合成 `game.sii` 字节一致，没有调用保存。80 辆是交互样本，不是几千辆车性能验收。

截图与机器可读结果在被 Git 忽略的 `verification/garage-ui/`。用户真实存档和游戏资源不进入测试夹具或版本库。
