# 构建与发布

目标平台为 Windows 10/11 x64。构建机需要 PowerShell 7、Node.js 22、Rust MSVC、Visual Studio C++ Build Tools 和 Windows SDK。GitHub Actions 使用 `windows-2022`，依赖安装使用 `npm ci` 与 Cargo `--locked`。

## 本地构建

```powershell
./scripts/build.ps1
```

脚本先检查版本、许可原始字节和打包规则，再检查 Rust 格式、编译前端、执行 Rust 测试、编译桌面程序和 CLI，验证 ZIP 内容后生成 `release/artifacts/ets2-workshop-<version>-windows-x64.zip` 与同名 `.sha256`。

Node 选择顺序为：显式 `-NodeDirectory`，否则依次寻找 PATH 中可用的 Node.js 22，再查找项目 `.tools/node-v22*-win-x64`。所选目录必须同时包含 `node.exe` 和 `npm.cmd`。例如 `./scripts/build.ps1 -NodeDirectory 'D:/tools/node-v22.16.0-win-x64'`。其他版本不会遮蔽后面的有效候选；显式指定错误目录则直接报错。

只运行 CI 检查：`./scripts/build.ps1 -CheckOnly`。已经完成构建后，仅重打包：`./scripts/package.ps1`。打包脚本不验证二进制是否来自最新源码，因此正式发布必须使用完整构建流程。

`-CheckOnly` 包括完整依赖许可收集、版本及打包负面测试，但不编译发布 EXE。普通 CI 运行完整构建，额外验证实际发布 ZIP。

从当前源码复现干净 Windows 检出（包括尚未提交且未被忽略的修复）：

```powershell
./scripts/verify-clean-checkout.ps1 -ExpectedTag v0.3.0
```

该脚本使用独立 Git 索引和 Windows 换行规则，在空目录执行 `-CheckOnly` 与完整构建，再用此次构建的 CLI 解码两份合成样本。它不修改当前索引或分支，不复制旧依赖目录或成品，最终仅保留验证后的 ZIP 和校验文件，清理临时检出。也支持 `-NodeDirectory`；版本变化后应使用对应标签。

打包采用全新临时目录，仅复制以下文件；不会递归复制已有 `release`，也不会加入本机目录缓存、设置、存档、备份、DLL 或官方解包器：

- `ETS2 Workshop.exe`（运行时检测启动器）
- `workshop-app.exe`（Tauri 主程序）
- `workshop-cli.exe`
- `README.md`
- `THIRD_PARTY_NOTICES.md`
- `licenses/` 中的许可文本

第三方声明或许可文本缺失时，打包失败。解码器随程序编译，官方解包器仅在归档需要回退解包时下载并校验；原生读取和有效缓存不要求联网。

## GitHub Actions

`ci.yml` 在分支 push、pull request 和手动触发时检查版本一致性、全部许可来源与依赖许可、Node 选择和打包失败用例、格式、前端构建和 Rust 测试，并编译、打包和检查真实 ZIP，不读取真实游戏存档。错误标签、版本漂移、许可缺失或原始字节变化会在发布前失败。

ZIP 检查限定入口、说明、许可清单及其声明的文件，核验许可内容，要求中英文启动说明，并扫描文本与 EXE 中的 Windows/macOS/Linux 用户目录路径。发布编译会重映射源码路径；路径扫描是针对常见个人路径的防线，不代表对任意隐私数据的自动识别。

`release.yml` 在推送 `v*` 标签或手动指定已有版本标签时构建、测试、打包，并发布 ZIP 与 SHA-256。手动运行仍然检出指定标签，不能把当前分支伪装成另一版本。所有 Action 固定到提交 SHA；升级时审核来源并更新 SHA。当前 CI 使用 stable Rust，依赖版本由提交的锁文件固定；这不保证不同日期的编译器产物逐字节相同。

发布步骤：

1. 同步 `package.json`、`package-lock.json`（包括根 package）、`src-tauri/Cargo.toml` 和 `src-tauri/tauri.conf.json` 的版本，更新 Cargo.lock，并提交所有改动。
2. 核对第三方声明、许可、支持范围和变更说明。在本机运行完整构建。
3. 创建并推送与版本一致的标签，例如 `v0.3.0`。标签与清单不匹配将导致打包失败。
4. 检查 Actions 完成、下载 Release ZIP、核对 SHA-256，并在没有旧设置的 Windows 用户环境中测试首次向导。

包含 `-` 的版本标签发布为 prerelease。同一标签重新执行流程会替换同名附件，因此应保持标签不可变，修复使用新版本号。构建失败不会运行发布步骤。不要提交真实存档或本机缓存。

## 用户运行依赖

ZIP 完整解压后运行 `ETS2 Workshop.exe`，不要单独移动 EXE；无需安装 Node、Rust 或 Truck Tools。Tauri 界面需要 **Microsoft Edge WebView2 Evergreen Runtime**。启动器检测该运行时，缺少时显示[微软官方下载页面](https://developer.microsoft.com/microsoft-edge/webview2/#download-section)，提示用户自行安装 Evergreen Standalone Installer（x64），完成后重新打开程序。应用不下载或执行运行时安装器；已安装运行时的启动不需要联网。检测与提示的验证方法见 [WebView2 验证](RUNTIME_VALIDATION.md)。

目前 ZIP 不包含代码签名；Windows 的来源提示与存档兼容性是不同的问题。首次配置与游戏内修改验证仍需要人工检查；CI 通过不代表所有游戏版本和配件组合均已验证。

## 项目许可证待决策

整个项目的许可证尚待维护者选择。第三方 MIT 等许可只适用于对应依赖，不能据此声明整个项目采用 MIT。发布检查保留全部第三方声明和原文，本阶段不替维护者选择项目许可证。
