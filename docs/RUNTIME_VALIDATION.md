# WebView2 检测与安装指引验证

启动器只检测 WebView2。缺少时原生提示提供微软官方下载页面，要求用户自行安装 Evergreen Standalone Installer（x64），安装后重新启动。应用不下载、不验证也不执行 WebView2 安装器，因此没有应用内安装、安装进度或安装失败恢复状态。

检测遵循 [Microsoft 官方分发文档](https://learn.microsoft.com/microsoft-edge/webview2/concepts/distribution)：检查当前用户原生注册表位置及本机 32 位视图中的 `pv`，要求四段数字版本且不为 `0.0.0.0`。这证明注册存在；真正能打开界面仍须桌面验收。

## 自动化与只读探测

```powershell
cargo test --locked --manifest-path src-tauri/Cargo.toml --lib runtime::tests
./scripts/probe-runtime.ps1 -PackageDirectory 'C:\Runtime Test\package' -ExpectedRuntime Present
```

合成测试覆盖合法/非法版本、缺少时错误提示包含官方链接、安装包选择与重启步骤，以及安装后检测通过可以继续启动。测试不修改注册表或调用安装器，不提供生产环境的检测绕过开关。探测脚本使用本次包内 `workshop-cli.exe runtime-status`，输出运行时状态、CLI 哈希和系统版本，不写入应用数据、不联网。

## 桌面验收

| 环境 | 操作与预期 |
| --- | --- |
| 已安装 WebView2 | 运行完整解压包中的 `ETS2 Workshop.exe`，直接显示应用界面 |
| 缺少 WebView2 | 提示中显示官方下载页面、Evergreen Standalone Installer（x64）和重启步骤；关闭提示后退出，不生成配置完成状态 |
| 用户自行安装完成 | 重新启动时重新检测，通过后打开应用界面 |

缺失场景可在独立 Windows Sandbox 或 VM 验证；先用探测脚本确认基线，将 `-ExpectedRuntime` 改为 `Missing`。不要卸载日常主机运行组件或伪造注册表。来宾使用本地测试数据目录，例如先设置 `$env:ETS2_WORKSHOP_DATA_DIR = 'C:\Runtime Test\data'` 再启动应用，不映射玩家真实配置或存档。

阶段二开发主机未发现可用的 Windows Sandbox、VirtualBox、VMware 或 Hyper-V 命令，且 `HypervisorPresent=False`。未在缺失运行时的独立 Windows 内实测原生弹窗和用户手工安装后的重启；该边界已记录，不把已删除的自动安装流程列为本阶段验收要求。
