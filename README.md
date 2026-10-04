# ETS2 Workshop

[简体中文](#简体中文) · [English](#english)

## 简体中文

一个用于《欧洲卡车模拟 2》的 Windows 存档改装工具。查看你拥有的卡车和配件，为它们更换发动机、变速箱、油箱等部件。

比如，给 Scania S 换上 Volvo 发动机，或用 DAF 的独立油箱增加容量。能否使用某个配件，取决于你的游戏内容和工具支持的改装范围。

### 开始使用

1. 从 [Releases](https://github.com/D3carabian/ets2-workshop/releases) 下载 Windows x64 发布包，完整解压，运行 `ETS2 Workshop.exe`。请保留解压出来的其他文件。
2. 首次打开时，向导会自动寻找游戏和存档目录。确认路径，或手动修改。
3. 等待程序读取配件。支持的游戏归档可以直接读取；只有需要官方解包工具且本机没有已校验副本时才需要联网下载。日常改装可以离线进行。
4. 打开一个存档，选择卡车和要更换的配件，预览修改后保存。第一次尝试建议另存为新存档。
5. 回到游戏，手动加载修改后的存档。之后可以在工具的改装记录中复查或恢复备份。

不需要另外安装存档解密工具。如果电脑缺少 WebView2（用于显示应用界面的微软组件），启动器会提供[微软官方下载页面](https://developer.microsoft.com/microsoft-edge/webview2/#download-section)。请自行安装 Evergreen Standalone Installer（x64），完成后重新打开程序。

### 能改什么

- 查看所有自有卡车，以及每辆车的配件。
- 中英文界面切换，按游戏名称搜索配件，比较品牌、型号与关键参数。
- 更换柴油发动机、变速箱、独立油箱，以及对应位置的轮胎和轮毂。
- 更换部分外观件，也可以从其他自有卡车复制部分警示灯、格栅和遮阳板。添加时需要车型、驾驶室、底盘相同，且安装位置可用。
- 保存前预览修改，另存为新存档，或自动备份后覆盖原档。

### 目前的限制

目前支持原版和已识别的官方 DLC，**不支持 Mod**。含有 Mod 或无法识别的扩展依赖的存档会被拒绝打开。

发动机等核心部件只能替换，不能再加一个。底盘、驾驶室和复杂外观组合暂不支持改装；不认识的配件只能查看。工具不修改金钱、等级或游戏安装文件。

维修站的改装升级可能把混搭配件换回原厂配置。此前测试中，普通维修和维修界面的部件更换保留了改装，但并非所有组合都经过验证。

已有 ETS2 1.61 的游戏内测试确认：Scania S 可以使用 DAF 的 1465 L 独立油箱和 Volvo 的 780 hp 发动机。更大马力不一定意味着更高极速，工具也不会预测精确极速。

已有用户升级后，可在“设置 → 建立 / 更新目录”中读取游戏名称。没有官方名称的内部组件会保留类别与原始标识。

### 存档与备份

存档列表默认显示手动存档和快速存档；勾选“包含自动存档”即可显示 autosave，程序会记住选择。未变化的名称会复用缓存，刷新仍会检查新存档和改名。

默认另存新档。覆盖或恢复前，请退出游戏并暂停会写入该槽位的同步及其他程序；写前校验和原子替换不能保证与外部程序同时写入时不丢失进度。

查看存档不会改动它。保存前，程序会检查游戏是否已经更新了这个存档，避免覆盖新的进度。备份可以恢复，但如果目标存档后来又被游戏修改，工具会阻止直接覆盖。恢复只还原 `game.sii` 中的车辆和游戏进度，不删除存档槽，也不更改名称、截图或信息文件；另存的槽位会保留新名称。保存中断后，可在改装记录中查看备份位置、尝试恢复或清理该次操作的临时文件。

### 开发与致谢

界面使用 Tauri 和 React，存档处理使用 Rust。构建和发布方法见[开发文档](docs/RELEASING.md)，测试情况见[验证记录](docs/REVIEW.md)。

存档解码基于 [DecryptTruck](https://github.com/CoffeSiberian/DecryptTruck)，读取游戏配件采用按需读取，遇到不支持的归档时使用 [SCS 官方解包工具](https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Extractor)。相关许可见[第三方声明](THIRD_PARTY_NOTICES.md)。本项目是非官方工具。

## English

A Windows save editor for Euro Truck Simulator 2. Browse your trucks and their parts, then swap engines, transmissions, fuel tanks, and more.

For example, you can fit a Volvo engine to a Scania S, or increase its fuel capacity with an independent DAF tank. Available parts depend on your installed game content and the modifications supported by the tool.

### Getting started

1. Download the Windows x64 package from [Releases](https://github.com/D3carabian/ets2-workshop/releases), extract the entire archive, and run `ETS2 Workshop.exe`. Keep the other extracted files alongside it.
2. On first launch, the setup wizard looks for your game and save folders. Confirm the paths or adjust them manually.
3. Wait while the app reads the available parts. Supported archives are read directly; an internet connection is needed only when the official extractor is required and no verified local copy is available. Normal editing works offline.
4. Open a save, choose a truck and replacement parts, then review and save your changes. Saving to a new slot is recommended for your first attempt.
5. Return to the game and load the edited save manually. You can later check your changes or restore a backup from the app's history page.

No separate save decryption tool is needed. If WebView2, Microsoft's component for displaying the app interface, is missing, the launcher will direct you to [Microsoft's official download page](https://developer.microsoft.com/microsoft-edge/webview2/#download-section). Install the Evergreen Standalone Installer (x64) yourself, then reopen the app.

### What you can do

- Browse all owned trucks and every part fitted to them.
- Switch between Chinese and English, search in-game part names, and compare brands, models, and key specifications.
- Replace diesel engines, transmissions, independent fuel tanks, and tires and rims in their corresponding positions.
- Replace some appearance parts, or copy certain beacons, grilles, and sunshields from another owned truck. Adding parts requires the same truck model, cabin, and chassis, with an available mounting position.
- Preview changes, save to a new slot, or overwrite a save after an automatic backup.

### Current limits

The app supports the base game and recognized official DLC. **Mods are not supported.** Saves that depend on Mods or unrecognized extensions will be refused.

Core parts such as engines can only be replaced; you cannot add a second one. Chassis, cabins, and complex appearance combinations cannot currently be edited. Unrecognized parts are view-only. The app does not change money, levels, or game installation files.

Customizing your truck at a service station may reset mixed-brand parts to factory options. In earlier tests, normal repairs and parts replacement through the repair menu preserved the modifications, but not every combination has been tested.

In-game tests with ETS2 1.61 confirmed that a Scania S could use an independent DAF 1,465 L tank and a Volvo 780 hp engine. More horsepower does not always mean a higher top speed, and the app does not predict exact top speeds.

After upgrading, use Settings → Build / update library to load in-game names. Internal components without a game name keep their category and original identifier.

### Saves and backups

The list shows manual saves and quicksaves by default. Enable “Include autosaves” to show autosave slots; the app remembers your choice. Unchanged names are cached, while refreshing still checks for new or renamed saves.

Saving to a new slot is the default. Before overwriting or restoring, exit the game and pause synchronization or other programs that can write that slot. Pre-write checks and atomic replacement do not guarantee safe concurrent writes by external programs.

Browsing a save does not change it. Before writing, the app checks whether the game has updated the save, to avoid overwriting newer progress. Backups can be restored, but the app blocks a direct restore if the game has since changed the destination save. Restore replaces only the vehicles and game progress in `game.sii`; it keeps the slot, name, screenshot, and information file. A new slot keeps its new name. After an interrupted save, use the history page to locate the backup, attempt a restore, or clean up that operation’s temporary files.

### Development and credits

The interface uses Tauri and React, with Rust handling saves. See the [build and release guide](docs/RELEASING.md) and [validation notes](docs/REVIEW.md), currently in Chinese.

Save decoding is based on [DecryptTruck](https://github.com/CoffeSiberian/DecryptTruck). Game parts are read selectively, with the [official SCS archive extractor](https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Extractor) used for unsupported archives. See [third-party notices](THIRD_PARTY_NOTICES.md) for licenses. This is an unofficial project.
