<p align="center">
  <img src="https://raw.githubusercontent.com/D3carabian/ets2-workshop/main/assets/branding/logo-c.png" alt="ETS2 Workshop truck badge" width="160" height="160">
</p>

<h1 align="center">ETS2 Workshop</h1>

<p align="center">
  Make your truck your own.
</p>

<p align="center">
  <a href="#简体中文">简体中文</a> ·
  <a href="#english">English</a> ·
  <a href="https://github.com/D3carabian/ets2-workshop/releases/latest">下载 / Download</a>
</p>

<p align="center"><sub>Windows x64 · Public Preview</sub></p>

## 简体中文

给《欧洲卡车模拟 2》用的存档改装工具，Windows 版。打开存档就能看到你名下所有卡车和车上的配件，然后把发动机、变速箱、油箱换成别家的。

比如让 Scania S 用上 Volvo 的 780 hp 发动机，再挂一个 DAF 的 1465 L 独立油箱。这个组合在 ETS2 1.61 里实测能用。（马力大了极速不一定更高，工具也不帮你算极速。）具体能装哪些配件，要看你装了哪些游戏内容，以及工具目前支持到哪一步。

### 怎么用

1. 去 [Releases](https://github.com/D3carabian/ets2-workshop/releases) 下载 Windows x64 压缩包，整个解压，然后运行 `ETS2 Workshop.exe`。旁边那些文件它都要用，别单独把 exe 拎出来。
2. 第一次打开会有向导自动找游戏和存档目录，确认一下就行，找错了可以手动改。
3. 等它把配件读完。大部分游戏归档可以直接读；只有遇到必须用官方解包工具、本机又没有校验过的副本时，才会联网下载一次。之后改装全程离线。
4. 打开存档，选车，选要换的配件，预览没问题就保存。第一次用建议另存成新存档。
5. 回游戏手动加载这个存档。想撤销可以去「备份与恢复」；在游戏里重新存档后，也可以回来看看配件还在不在。

存档解密已经内置，不用另外装工具。如果电脑上缺 WebView2（微软用来显示程序界面的组件），启动器会打开[微软官方下载页](https://developer.microsoft.com/microsoft-edge/webview2/#download-section)，自己装一下 Evergreen Standalone Installer (x64)，再重新打开程序就好。

### 能做什么

- 查看所有自有卡车，以及每辆车上的配件
- 中英文界面随时切换，用游戏里的名字搜配件，对比品牌、型号和主要参数
- 更换柴油发动机、变速箱、独立油箱，还有对应位置的轮胎和轮毂
- 更换一部分外观件；也可以从你另一辆卡车上复制部分警示灯、格栅和遮阳板，前提是两辆车的车型、驾驶室、底盘相同，并且安装位置空着
- 保存前预览改动，可以另存新档，也可以自动备份后覆盖原档

### 用之前要知道的

只支持原版和能识别的官方 DLC，**不支持 Mod**。存档只要依赖了 Mod 或认不出的扩展内容，工具就不会打开它。

发动机这类核心部件只能换，不能多装一个。底盘、驾驶室和复杂的外观组合暂时改不了，认不出的配件只能看。金钱、等级和游戏安装文件，工具都不会动。

在维修站做改装升级，混搭的配件可能会被换回原厂。之前测试时，普通维修和在维修界面换零件都没影响改装，不过不是每种组合都试过。

如果是从旧版本升级上来的，到「设置 → 建立 / 更新目录」里重新读一次，配件就会显示游戏里的名字。有些内部组件游戏本身就没起名，这种会显示类别和原始标识。

### 存档和备份

存档列表默认只列手动存档和快速存档。想看自动存档就勾上「包含自动存档」，程序会记住这个选择。名字没变的存档直接用缓存，点刷新还是会检查有没有新存档或改过名的。

只是打开存档看看，不会改动它。

默认是另存新档。覆盖或恢复之前，请先退出游戏，并暂停云同步等会写这个存档槽的程序。工具写入前会做校验，也用了原子替换，但如果别的程序同时在写，它没办法保证进度不丢。

保存前，工具会确认游戏有没有在你预览之后又存过这个档，免得把新进度盖掉。恢复备份也一样：目标存档后来被游戏改过的话，工具会拦住，不让直接覆盖。恢复只还原 `game.sii` 里的车辆和游戏进度，存档槽、名称、截图和信息文件都保持不变；另存出来的槽位会保留新名字。

万一保存中途断了，去「备份与恢复」可以看到备份放在哪，试着恢复，或者清掉那次操作留下的临时文件。

### 开发

界面用 Tauri 和 React，存档处理用 Rust。构建和发布流程见[开发文档](docs/RELEASING.md)，测试情况见[验证记录](docs/REVIEW.md)。

### 致谢

存档解码基于 [DecryptTruck](https://github.com/CoffeSiberian/DecryptTruck)。读取游戏配件时按需读取，遇到读不了的归档才调用 [SCS 官方解包工具](https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Extractor)。相关许可见[第三方声明](THIRD_PARTY_NOTICES.md)。

这是非官方工具，和 SCS Software 没有关系。

### 许可证

本项目采用 [MIT 许可证](LICENSE)。第三方组件保留各自的许可证，详见[第三方声明](THIRD_PARTY_NOTICES.md)。

## English

A save editor for Euro Truck Simulator 2 on Windows. Open a save, see every truck you own and the parts fitted to it, then swap in engines, transmissions, and fuel tanks from other brands.

Want a Scania S with a Volvo 780 hp engine and a 1,465 L independent DAF tank? That exact setup was tested in ETS2 1.61 and works. (More horsepower doesn't always mean a higher top speed, and the app won't predict top speed for you.) Which parts you can use depends on your installed game content and what the app currently supports.

### Getting started

1. Grab the Windows x64 package from [Releases](https://github.com/D3carabian/ets2-workshop/releases), extract the whole thing, and run `ETS2 Workshop.exe`. It needs the other files next to it, so don't move the exe out on its own.
2. On first launch, a setup wizard looks for your game and save folders. Confirm them, or fix the paths if it guessed wrong.
3. Let it finish reading parts. Most game archives are read directly. It only goes online if it needs the official extractor and doesn't have a verified local copy. After that, editing works offline.
4. Open a save, pick a truck and the parts to swap, check the preview, and save. For your first try, save to a new slot.
5. Load the edited save in the game yourself. To undo, use Backups & restore. You can also come back after saving in-game to check that the parts stuck.

Save decryption is built in, so there's nothing else to install. If WebView2 (the Microsoft component that draws the app's window) is missing, the launcher opens [Microsoft's download page](https://developer.microsoft.com/microsoft-edge/webview2/#download-section). Install the Evergreen Standalone Installer (x64), then start the app again.

### What it does

- Lists every truck you own and every part on it
- Switches between Chinese and English, searches parts by their in-game names, and compares brand, model, and main specs
- Swaps diesel engines, transmissions, independent fuel tanks, and tires and rims in matching positions
- Swaps some appearance parts, and can copy certain beacons, grilles, and sunshields from another truck you own, as long as both trucks share the same model, cabin, and chassis and the mounting spot is free
- Previews changes before saving, then either saves to a new slot or backs up and overwrites the original

### Things to know first

It works with the base game and recognized official DLC. **Mods are not supported.** If a save depends on a mod or anything the app can't identify, it won't open.

Core parts like engines can be replaced, but you can't add a second one. Chassis, cabins, and complex appearance combos can't be edited yet, and parts the app doesn't recognize are view-only. It never touches money, levels, or game install files.

Upgrading your truck at a service station may reset mixed-brand parts to factory options. In earlier tests, regular repairs and swapping parts through the repair menu kept the changes, but not every combination has been tried.

If you're upgrading from an older version, go to Settings → Build / update library once so parts show their in-game names. Some internal components have no name in the game itself; those show their category and original ID instead.

### Saves and backups

By default the save list shows manual saves and quicksaves. Tick "Include autosaves" to see autosave slots too, and the app will remember that. Saves whose names haven't changed are cached, and Refresh still picks up new or renamed saves.

Just opening a save to look around doesn't change it.

Saving to a new slot is the default. Before you overwrite or restore, quit the game and pause cloud sync or anything else that might write to that slot. The app checks before writing and replaces files atomically, but it can't protect your progress if another program writes at the same time.

Before saving, the app checks whether the game has saved over that slot since you opened it, so newer progress doesn't get overwritten. Restores work the same way: if the game has changed the target save since the backup, the app won't let you restore over it directly. A restore only puts back the vehicles and game progress in `game.sii`. The slot, its name, screenshot, and info file stay as they are, and a new slot keeps its new name.

If a save gets interrupted, open Backups & restore to find the backup, try restoring it, or clean up the temporary files from that run.

### Development

The UI is Tauri + React, and saves are handled in Rust. See the [build and release guide](docs/RELEASING.md) and [validation notes](docs/REVIEW.md) (both in Chinese for now).

### Credits

Save decoding is based on [DecryptTruck](https://github.com/CoffeSiberian/DecryptTruck). Game parts are read on demand, and the [official SCS archive extractor](https://modding.scssoft.com/wiki/Documentation/Tools/Game_Archive_Extractor) is only used for archives the app can't read itself. License details are in [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

This is an unofficial tool and isn't affiliated with SCS Software.

### License

This project is licensed under the [MIT License](LICENSE). Third-party components retain their own licenses; see [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).
