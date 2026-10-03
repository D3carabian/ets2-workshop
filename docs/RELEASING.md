# 构建与发布

目标平台为 Windows 10/11 x64。构建机需要 PowerShell 7、Node.js 22、Rust MSVC、Visual Studio C++ Build Tools 和 Windows SDK。GitHub Actions 使用 `windows-2022`，依赖安装使用 `npm ci` 与 Cargo `--locked`。

## 本地构建

```powershell
./scripts/build.ps1
```

脚本检查 Rust 格式、编译前端、执行 Rust 测试、编译桌面程序和 CLI，再生成 `release/artifacts/ets2-workshop-<version>-windows-x64.zip` 与同名 `.sha256`。

只运行 CI 检查：`./scripts/build.ps1 -CheckOnly`。已经完成构建后，仅重打包：`./scripts/package.ps1`。打包脚本不验证二进制是否来自最新源码，因此正式发布必须使用完整构建流程。

打包采用全新临时目录，仅复制以下文件；不会递归复制已有 `release`，也不会加入本机目录缓存、设置、存档、备份、DLL 或官方解包器：

- `ETS2 Workshop.exe`（运行时检测启动器）
- `workshop-app.exe`（Tauri 主程序）
- `workshop-cli.exe`
- `README.md`
- `THIRD_PARTY_NOTICES.md`
- `licenses/` 中的许可文本

第三方声明或许可文本缺失时，打包失败。解码器随程序编译，官方解包器由首次配置流程下载；用户首次建目录需要联网。

## GitHub Actions

`ci.yml` 在分支 push、pull request 和手动触发时检查格式、前端构建和 Rust 测试，不读取真实游戏存档。

`release.yml` 在推送 `v*` 标签或手动指定已有版本标签时构建、测试、打包，并发布 ZIP 与 SHA-256。手动运行仍然检出指定标签，不能把当前分支伪装成另一版本。所有 Action 固定到提交 SHA；升级时审核来源并更新 SHA。当前 CI 使用 stable Rust，依赖版本由提交的锁文件固定；这不保证不同日期的编译器产物逐字节相同。

发布步骤：

1. 同步 `package.json`、`package-lock.json`（包括根 package）、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 的版本，更新 Cargo.lock，并提交所有改动。
2. 核对第三方声明、许可、支持范围和变更说明。在本机运行完整构建。
3. 创建并推送与版本一致的标签，例如 `v0.1.0`。标签与清单不匹配将导致打包失败。
4. 检查 Actions 完成、下载 Release ZIP、核对 SHA-256，并在没有旧设置的 Windows 用户环境中测试首次向导。

包含 `-` 的版本标签发布为 prerelease。同一标签重新执行流程会替换同名附件，因此应保持标签不可变，修复使用新版本号。构建失败不会运行发布步骤。不要提交真实存档或本机缓存。

## 用户运行依赖

ZIP 完整解压后运行 `ETS2 Workshop.exe`，不要单独移动 EXE；无需安装 Node、Rust 或 Truck Tools。Tauri 界面需要 **Microsoft Edge WebView2 Evergreen Runtime**。启动器检测该运行时，缺少时通过原生提示引导下载 Microsoft 官方安装器、验证签名并安装，再启动 `workshop-app.exe`。该流程需要网络，不能保证首次离线可用。也可手动安装 [Microsoft 官方 WebView2 Evergreen Runtime](https://developer.microsoft.com/microsoft-edge/webview2/#download-section)。

目前 ZIP 不包含代码签名；Windows 的来源提示与存档兼容性是不同的问题。首次配置与游戏内修改验证仍需要人工检查；CI 通过不代表所有游戏版本和配件组合均已验证。
