# ETS2 Workshop

Windows x64 的本地 ETS2 存档改装工具。技术栈为 Tauri 2、React / TypeScript、Rust。

## 使用

从 GitHub Release 下载 Windows x64 ZIP，完整解压后运行 `ETS2 Workshop.exe`。不要只复制单个 EXE：同目录中的 `workshop-app.exe` 是主程序。

首次启动会自动检测 Steam 多游戏库、文档和 OneDrive 中的路径。确认游戏安装目录与 ETS2 用户数据目录后，程序从 SCS 官方下载并校验解包器，再建立配件目录。存档解码器已编译进程序，不需要安装 Truck Tools、Rust、Node 或 Python。

首次准备需要联网；下载并建立目录后，正常改装可以离线使用。若缺少 WebView2，启动器会先询问是否从微软下载并安装，完成后再打开向导。下载失败可以重试，不会改动游戏存档。

1. 建立本机配件目录。首次解包本体与已识别的官方车型、轮胎、改装 DLC 需要数分钟；再次运行使用文件时间和大小标识的缓存。
2. 打开存档；自动在独立子进程中解密。界面列出玩家拥有的所有卡车和其所有配件。查看操作不写入游戏文件。
3. 选择配件，从其他车辆或配件目录选替换件，加入变更清单。性能参数来自游戏定义；不预测精确极速。
4. 预览后另存为新存档，或备份并覆盖原档。保存时重新检查源文件哈希；如果游戏已更新源档，必须重新打开。
5. 在游戏里手动加载。另存结果后，在改装记录页复查配件是否保留。恢复备份仅在目标仍是本工具写入的版本时允许。

## 改装范围

- 柴油发动机、变速箱、独立油箱和轮胎/轮毂位置之间的同类替换。核心部件保留配件 ID、退款和车辆状态，仅替换 `data_path`。
- 外观件同车型、符合 `suitable_for` / `conflict_with` / `require` 时可替换。
- 外观追加目前开放 beacon、r_grill、f_grill、sunshld。必须从已有自有车辆选择供体；供体与目标车型、驾驶室和底盘一致；目标安装类别未占用；不携带子挂件或关联对象。
- 禁止追加发动机、变速箱、底盘、驾驶室、内饰、油箱等核心单例。底盘、驾驶室等复杂结构首版只读。
- 未知定义只读。官方资源目录不能代表 Mod 的覆盖顺序；检测到 Mod 或未知扩展依赖的存档将拒绝打开。
- 维修站的改装升级可能恢复原厂配件。实测普通维修及维修界面的部件更换可以保留此前测试的改装，但不代表所有组合均兼容。
- 不修改游戏安装文件，不生成 Mod，不修改货币和等级；输出明文 SII，已在本机 1.61 存档试验中验证可加载。

## 已有游戏内证据

Scania S 原独立 1000 L 油箱替换成 `/def/vehicle/truck/daf.2021/accessory/tank/4x2_1465.sii`：加载、驾驶、游戏再次保存后仍保留，用户报告续航 3112 → 4576。

Scania S 发动机替换为 `/def/vehicle/truck/volvo.fh_2024/engine/d17a780.sii`：用户确认可驾驶。追加第二底盘曾造成车轮模型初始化失败，因此程序硬性禁止此操作。

这类游戏内证据来自先前脚本试验。桌面程序使用相同字段操作，但新组合仍需游戏内验证。unit ID 会在游戏重存后变化，复查按车型、车牌定位；存在歧义时明确提示，不能假装匹配成功。

## 开发

需要 Node.js 22、Rust MSVC、Visual Studio C++ Build Tools 和 WebView2。运行：

```powershell
npm ci
npm run build
cargo test --manifest-path src-tauri/Cargo.toml
npm run tauri dev
```

发布构建：`pwsh -File scripts/build.ps1`，ZIP 输出到 `release/artifacts/`。仅本机编译需要以上依赖；成品不依赖 Python 或 Node。

源码：`src` 为界面，`src-tauri/src` 为解析、目录、改装、解密和保存核心。早期本机实验脚本、玩家存档和生成缓存均不进入公开源码或 Release。

CLI：`workshop-cli inspect <game.sii> [catalog.json]` 只读检查；`workshop-cli index <游戏目录> <scs_extractor.exe> <缓存目录>` 建目录。

`ETS2_WORKSHOP_DATA_DIR` 可以将设置、缓存、历史与备份隔离到指定目录，供自动化测试使用。默认使用 Windows 本地应用数据目录。

## 依赖来源

存档解码器基于 MIT 许可的 DecryptTruck 1.3.7，固定到 commit `4b6a167d7b35a5234bb3dde17ac1532740860791` 并随源码提供。局部修改与更新步骤见 `src-tauri/vendor/decrypt-truck/UPSTREAM.md`。正式包不再依赖或携带 SII_Decrypt.dll。

官方解包器不随发布包再分发，向导直接从 SCS 官方获取，并同时校验压缩包和 EXE 的固定 SHA256。上游文件变化时明确停止，需维护者核实后更新应用中的清单。

第三方声明见 `THIRD_PARTY_NOTICES.md`；发布与 CI 说明见 `docs/RELEASING.md`。

- https://github.com/CoffeSiberian/DecryptTruck
- https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Extractor
- https://modding.scssoft.com/wiki/Documentation/Engine/Units/accessory_data

## 测试原则

自动化仅使用合成存档或复制到工作区的存档。不得对玩家正在使用的存档运行写入测试。结构验证不等于游戏模型兼容性验证。
