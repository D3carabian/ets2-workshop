# 首次配置验证矩阵

首次配置测试不再要求本机有 ETS2、预先建立的缓存或固定数量的配件。Rust 路径测试只接受显式注入的位置；桌面脚本生成合成资源，设置独立 `ETS2_WORKSHOP_DATA_DIR` 和 WebView 数据目录，写入仅在当前工作区 `verification` 下。

## 自动回归

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml
npm run build
cargo build --locked --features custom-protocol --manifest-path src-tauri/Cargo.toml --bin ets2-workshop --bin workshop-cli --bin workshop-launcher
node scripts/onboarding-smoke.mjs
./scripts/probe-runtime.ps1 -PackageDirectory ./src-tauri/target/debug -ExpectedRuntime Present
```

使用 Node.js 22、Rust MSVC 和已安装 WebView2 的 Windows。桌面脚本默认运行当前检出的 debug EXE，也可传入当前工作区内另一个刚构建的 EXE 路径。SCS 下载的真实桌面验收需要联网，离线失败、错误 hash、文件占用等由 Rust 注入测试稳定复现。

| 场景 | 验证方式与预期 |
| --- | --- |
| 新旧 Steam libraryfolders、多库、不同安装名 | `tests/discovery.rs`，按 VDF 结构解析并去重 |
| 已知文档位置、OneDrive、中文/空格、自定义用户目录 | 显式合成输入，日志和 ETS2 LaunchOptions 的 `-homedir` 能被发现 |
| 多 profile、Steam 本地/云存档、多账号 | 各根目录合成 `game.sii` / `info.sii`，重复位置去重 |
| 缺失或损坏 info | 显示错误条目，不能静默隐藏存档 |
| 不可读取 profile、被占用 info | Windows 独占句柄触发实际读取失败，错误含下一步 |
| 确认前、多个候选、手工修改后重新检测 | 实际 WebView；不写设置、不自动猜第一个、不覆盖用户修改 |
| 无效或不可枚举用户目录 | 确认前预检 profiles/profile/save/slot；修复后可重试 |
| 下载失败、ZIP/EXE hash 不符、损坏/有效缓存 | 私有依赖注入和合成 ZIP；验证失败不安装，有效缓存离线复用 |
| 工具缓存不可写、设置最后一步不可写 | 实际文件冲突与 Windows 锁；不留下完成标记，重试成功 |
| 首次解包与中文游戏/缓存路径 | 实际 SCS 解包器和合成 HashFS；隔离 staging 中使用 ASCII 相对参数 |
| 配置持久化与重启 | 实际 WebView 进程退出再启动；目录只有 1 个合成定义，向导不再出现 |
| WebView2 已安装/缺失 | 已安装主机打开真实界面；缺失分支合成测试返回手工安装指引，见 `RUNTIME_VALIDATION.md` |

桌面测试通过 WebView 的 fetch 测试钩子替换“检测候选”和“存档列表”响应；向导本身、设置 RPC、路径预检、官方下载/hash、资源解包、持久化和重启均走真实实现。路径与存档枚举的实际后端由独立 Rust 矩阵覆盖。测试钩子仅存在于脚本中，不写入生产应用。

成功后脚本在唯一 `verification/onboarding-synthetic-*` 目录保留截图和 `result.json`，清理合成游戏、缓存、设置与 WebView 数据。不要上传本机截图或原始报告，它们可能显示本机路径。`scripts/ui-smoke.mjs` 仍用于既有改装/保存手工隔离夹具，不是本阶段首次配置验收的前置条件。

## 完成状态与兼容

首次配置将配件目录保存为独立代文件，再原子保存引用该文件的设置；第二步失败时旧设置和旧配件目录仍匹配。成功后清理前一份受管理目录文件。旧版本的 `catalog.json` 仍能读取，缺少配件目录时重新进入向导。失败消息给出检查权限、空间或网络后重试的步骤。

不再由应用安装 WebView2；这是本阶段用户明确调整的范围。未在缺少 WebView2 的独立 Windows 系统实测弹窗及手工安装后的重启，也未做新游戏版本或所有 DLC 的游戏内兼容验证。
