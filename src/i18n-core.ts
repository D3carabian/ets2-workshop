export type Locale = "zh-CN" | "en";
export type TextValues = Record<string, string | number>;
export const LANGUAGE_STORAGE_KEY = "ets2-workshop.language";

const english: Record<string, string> = {
  "需要解包时，自动从 SCS 官方获取工具并校验":
    "When extraction is needed, download and verify the official SCS tool automatically",
  "仅支持原版及官方 DLC，不支持 Mod 存档。直接读取不需要下载工具；遇到需要解包的归档时，请保持联网并预留磁盘空间。":
    "Base game and official DLC only; Mod saves are unsupported. Direct reading needs no tool download. Archives requiring extraction may need internet access and extra disk space.",
  "覆盖或恢复前，请退出游戏并暂停会写入该存档的同步及其他程序。":
    "Before overwriting or restoring, exit the game and pause synchronization or other programs that can write this save.",
  "配件目录需要更新，当前仅供查看。请在设置中重新建立目录后再修改。":
    "The parts library needs updating and is currently view-only. Rebuild it in Settings before editing.",
  "请先保存或移除待保存修改，再更换游戏安装或重新配置":
    "Save or remove pending changes before switching the game installation or running setup again",
  包含自动存档: "Include autosaves",
  "车型、车牌或驾驶员": "Model, plate or driver",
  "驾驶员：玩家": "Driver: Player",
  "雇员：{name}": "Employee: {name}",
  编号未知: "Unknown ID",
  未分配驾驶员: "No driver assigned",
  驾驶员未识别: "Driver unidentified",
  默认显示手动存档和快速存档:
    "Manual saves and quicksaves are shown by default",
  自动存档: "Autosave",
  "当前已打开：{name}": "Currently open: {name}",
  驾驶室: "Cabin",
  底盘: "Chassis",
  发动机: "Engine",
  变速箱: "Transmission",
  内饰: "Interior",
  涂装: "Paint job",
  外观配件: "Exterior accessories",
  内饰配件: "Cabin accessories",
  "其他 / 未识别": "Other / unidentified",
  独立油箱: "Standalone fuel tank",
  车辆基础: "Base vehicle",
  前轮轮胎: "Front tires",
  后轮轮胎: "Rear tires",
  前轮轮盘: "Front discs",
  后轮轮盘: "Rear discs",
  前轮轮毂: "Front hubs",
  后轮轮毂: "Rear hubs",
  前轮轮毂盖: "Front hub covers",
  后轮轮毂盖: "Rear hub covers",
  前轮螺母: "Front nuts",
  后轮螺母: "Rear nuts",
  警示灯: "Beacons",
  车顶灯架: "Roof bar",
  前部防护杆: "Bull bar",
  保险杠灯架: "Bumper bar",
  遮阳板: "Sun visor",
  前大灯: "Headlights",
  车型铭牌: "Model badge",
  前保险杠: "Front bumper",
  后保险杠: "Rear bumper",
  侧裙: "Side skirts",
  侧裙护杆: "Side skirt bars",
  后视镜: "Mirrors",
  前部补盲镜: "Front mirror",
  侧面补盲镜: "Side mirror",
  前挡泥板: "Front mudflaps",
  后挡泥板: "Rear mudflaps",
  后轮翼子板: "Rear fenders",
  左侧排气管: "Left exhaust",
  右侧排气管: "Right exhaust",
  排气管: "Exhaust",
  悬挂饰品: "Hanging toys",
  仪表台饰品: "Dashboard accessories",
  座椅饰品: "Seat accessories",
  卧铺饰品: "Bunk accessories",
  车内饰品: "Cabin accessories",
  车内面板饰品: "Cabin panel accessories",
  方向盘: "Steering wheel",
  前窗帘: "Front curtains",
  靠枕: "Pillow",
  地毯: "Carpet",
  驾驶员名牌: "Driver plate",
  副驾驶名牌: "Codriver plate",
  副驾驶座椅: "Passenger seat",
  仪表台: "Dashboard",
  车内玻璃饰件: "Cabin window accessories",
  车内侧窗饰件: "Side window accessories",
  杯架: "Cup holder",
  车内显示屏: "Cabin display",
  车内照明: "Cabin lighting",
  未识别配件: "Unidentified part",
  "参数未知。该配件尚未索引，原始数据会保留。":
    "Unknown specifications. This part is not indexed; original data will be preserved.",
  "此配件未提供性能参数。": "No specifications provided for this part.",
  "马力与转速范围采用游戏标称值；扭矩采用定义参数。":
    "Horsepower and RPM ranges use game display values; torque comes from the definition.",
  全部定义参数: "All definition parameters",
  数据来源与定义: "Source and definition",
  来自本机游戏定义: "From local game definitions",
  关键属性: "Key specifications",
  候选: "Candidate",
  未知: "Unknown",
  候选配件来源: "Candidate source",
  游戏标称功率: "Game rated power",
  峰值扭矩: "Peak torque",
  附加扭矩: "Additional torque",
  标称扭矩转速范围: "Rated torque RPM range",
  排量: "Displacement",
  转速上限: "RPM limit",
  前进挡: "Forward gears",
  主减速比: "Differential ratio",
  缓速器: "Retarder",
  未标明: "Not specified",
  无: "None",
  前进挡齿比: "Forward ratios",
  倒挡齿比: "Reverse ratios",
  独立油箱容量: "Standalone tank capacity",
  底盘油箱容量: "Chassis tank capacity",
  动力类型: "Power type",
  游戏显示信息: "Game display information",
  扭矩曲线: "Torque curve",
  油耗系数: "Consumption coefficient",
  滚动阻力: "Rolling resistance",
  湿地抓地力: "Wet grip",
  滚动噪声: "Rolling noise",
  半径: "Radius",
  语言: "Language",
  读取本地配置: "Reading local settings",
  车库: "Garage",
  配件目录: "Parts library",
  改装记录: "History",
  设置: "Settings",
  本地改装工作台: "Local truck workshop",
  "本地处理 · Windows x64": "Local processing · Windows x64",
  个配件定义: "part definitions",
  自动备份: "Automatic backups",
  关闭消息: "Dismiss message",
  "每辆卡车，都有自己的配置。": "Every truck has its own setup.",
  "选择车辆，查看配件。修改先进入清单，保存时才会写入存档。":
    "Choose a truck and inspect its parts. Changes remain pending until you save.",
  保存修改: "Save changes",
  选择存档: "Select save",
  选择一个存档: "Select a save",
  读取并解密存档: "Reading and decoding save",
  打开存档: "Open save",
  刷新存档列表: "Refresh saves",
  手动路径: "Manual path",
  "game.sii 路径": "game.sii path",
  "粘贴 game.sii 的完整路径": "Paste the full path to game.sii",
  打开指定文件: "Opening file",
  打开: "Open",
  从你的车库开始: "Start with your garage",
  "打开一个存档，自动解密并读取所有自有卡车。":
    "Open a save to decode it and load all owned trucks.",
  "发动机、油箱、轮胎和外观附件，都在同一个工作台。":
    "Engines, tanks, tires and accessories in one workshop.",
  选择配件: "Choose parts",
  预览并保存: "Preview and save",
  建立本机配件目录: "Build local parts library",
  车库列表: "Garage list",
  我的卡车: "My trucks",
  辆: "trucks",
  搜索卡车: "Search trucks",
  "车型、车牌或编号": "Model, plate or ID",
  上一页: "Previous",
  下一页: "Next",
  本页车辆: "Trucks on this page",
  没有匹配的卡车: "No matching trucks",
  正在驾驶: "Current truck",
  未设置车牌: "No license plate",
  无车牌: "No license plate",
  个配件: "parts",
  "· 完整配件列表": "· All installed parts",
  添加附件: "Add accessory",
  搜索车辆配件: "Search truck parts",
  "搜索名称、类别或路径": "Search name, category or path",
  配件类别: "Part category",
  所有类别: "All categories",
  当前车辆配件: "Installed parts",
  已识别: "Identified",
  "只读 / 未索引": "Read-only / not indexed",
  没有匹配的配件: "No matching parts",
  "个配件 · 未识别字段仍会保留": "parts · Unknown fields are preserved",
  配件详情与替换: "Part details and replacement",
  追加外观附件: "Add exterior accessory",
  配件详情: "Part details",
  选择一个配件: "Choose a part",
  查看参数与替换方案: "View specifications and replacements",
  "只接受同车型、同驾驶室和底盘的供体附件；安装类别不能已被占用。":
    "Donor accessories require the same truck model, cabin and chassis, and an unoccupied installation category.",
  附件类别: "Accessory category",
  " · 禁止追加": " · Cannot add",
  当前: "Current",
  当前配件全部参数: "All current part specifications",
  查看原始字段与定义路径: "Raw fields and definition path",
  "原始名称：": "Raw name: ",
  "· 原始分类：": "· Raw category: ",
  "refund 是存档退款字段；替换时保留原值":
    "refund is the save refund field; its original value is preserved: ",
  选择供体附件: "Choose donor accessory",
  替换为: "Replace with",
  从车库选择: "From garage",
  本机配件库: "Local library",
  搜索候选配件: "Search replacement parts",
  "搜索品牌、型号、马力或配件": "Search brand, model, horsepower or part",
  " · 供体 {detail}": " · Donor {detail}",
  "没有候选配件。可切换配件库，或先建立目录。":
    "No candidates. Switch sources or build the parts library first.",
  共: "Total",
  "项，显示前 150 项。输入品牌、型号或马力缩小范围。":
    "items; showing the first 150. Search by brand, model or horsepower to narrow results.",
  校验改装规则: "Validating modification rules",
  加入变更清单: "Stage change",
  "发动机、变速箱、底盘等核心部件禁止重复追加。所有操作由后台再次校验。":
    "Core parts such as engines, transmissions and chassis cannot be added twice. The backend validates every operation.",
  待保存的修改: "Pending changes",
  尚未写入游戏存档: "Not yet written to the save",
  新增: "Added",
  移除此修改: "Remove this change",
  更新清单: "Updating changes",
  "已加入变更清单，尚未写入存档":
    "Change staged; nothing has been written to the save yet",
  "连接你的游戏。": "Connect your game.",
  "文件保留在这台电脑上。存档解码器已内置，解包工具由程序自动准备。":
    "Files stay on this computer. The save decoder is built in; the extractor is prepared automatically.",
  文件位置: "File locations",
  重新运行配置向导: "Run setup again",
  "ETS2 用户数据目录": "ETS2 user data directory",
  游戏安装目录: "Game installation directory",
  保存设置: "Save settings",
  设置已保存: "Settings saved",
  "读取 def.scs 及官方车型、轮胎与改装 DLC，获得真实定义路径、配件类型和性能参数。首次解包需要一些时间。":
    "Read def.scs and official truck, tire and tuning DLC for definition paths, part types and specifications. The first extraction takes time.",
  已索引定义: "indexed definitions",
  "正在解包并索引游戏定义，首次运行可能需要数分钟":
    "Extracting and indexing game definitions; the first run may take several minutes",
  "已索引 {count} 个配件定义。目录仅涵盖已识别的官方资源。":
    "Indexed {count} part definitions. The library includes recognized official resources only.",
  "建立 / 更新目录": "Build / update library",
  "当前不支持 Mod；检测到 Mod 或未知扩展依赖的存档将拒绝打开。":
    "Mods are not supported. Saves with detected mods or unknown expansion dependencies cannot be opened.",
  本地缓存与备份目录: "Local cache and backup directory",
  "从定义认识性能。": "Explore part specifications.",
  "参数来自本机游戏文件。显示马力与实际动力参数分开，最高速度不作精确预测。":
    "Specifications come from local game files. Displayed horsepower and actual power parameters are separate; top speed is not predicted.",
  更新目录: "Update library",
  搜索配件目录: "Search parts library",
  "搜索品牌、型号、engine、tank…": "Search brand, model, engine, tank…",
  路径与适配条件: "Path and compatibility",
  "定义未指定 suitable_for": "No suitable_for specified",
  "请先在设置中建立配件目录。": "Build the parts library in Settings first.",
  "为保持列表流畅，每次显示前 120 项。输入关键词可缩小范围。":
    "Showing the first 120 items for performance. Search to narrow results.",
  "每次改装，都有据可查。": "A record of every modification.",
  "读取游戏另存的结果，检查配件是否保留。车辆身份变化时会提示手动核对。":
    "Inspect a save made by the game to check whether changes survived. If truck identity changed, manual review is required.",
  刷新记录: "Refreshing history",
  刷新: "Refresh",
  用于复查的游戏存档: "Game save to verify",
  选择游戏另存的结果: "Choose a save made by the game",
  刷新游戏存档: "Refreshing game saves",
  项改装: "modifications",
  "准备未完成，本工具尚未写入目标。备份可能不完整，可清理临时文件后重新保存。":
    "Preparation did not finish; the target has not been written. The backup may be incomplete. Clean temporary files and save again.",
  "写入未确认或已中断。备份保留在下方位置；恢复时会核对目标内容，拒绝覆盖新的进度。":
    "Writing was interrupted or unconfirmed. The backup is below. Restore checks the target and refuses to overwrite newer progress.",
  检查改装是否保留: "Checking retained modifications",
  复查所选存档: "Verify selected save",
  恢复修改前: "Restore original",
  清理临时文件: "Clean temporary files",
  "已清理该记录的临时文件，备份和存档已保留":
    "Temporary files cleaned; backups and saves preserved",
  备份位置: "Backup location",
  "保存第一次改装后，记录将显示在这里。":
    "History appears here after you save your first modification.",
  复查结果: "Verification results",
  "独立油箱与发动机跨品牌替换已实测 · 外观组合仍需游戏内验证":
    "Cross-brand standalone tanks and engines tested · Appearance combinations still require in-game verification",
  "正在读取车库…": "Loading garage…",
  保存这次改装: "Save these modifications",
  关闭: "Close",
  "项变更已通过结构和操作规则检查。保存前自动备份。":
    "changes passed structural and operation checks. A backup will be created before saving.",
  保存方式: "Save mode",
  "另存为新存档（推荐）": "Save as new (recommended)",
  备份并覆盖当前存档: "Back up and overwrite current save",
  新存档名称: "New save name",
  "保存后需要在游戏中手动加载。改装升级可能恢复原厂配件。":
    "Manually load the save in the game. In-game upgrades may restore factory parts.",
  "备份、校验并写入存档": "Backing up, validating and writing save",
  确认保存: "Confirm save",
  "恢复修改前的存档？": "Restore the save before modification?",
  "仅恢复车辆和游戏进度文件 game.sii，不删除存档槽，也不恢复名称、截图或 info.sii。 另存的槽位会保留新名称。恢复前会核对目标内容，新记录也会核对存档信息；发现更新则拒绝恢复。":
    "Only game.sii (vehicles and game progress) is restored. The save slot, name, screenshot and info.sii remain unchanged; a new slot keeps its new name. Restore verifies the target and, for newer records, save metadata. It refuses restoration if newer progress is detected.",
  取消: "Cancel",
  恢复备份: "Restoring backup",
  "已恢复修改前的 game.sii；存档名称和信息保持不变":
    "Original game.sii restored; save name and metadata unchanged",
  恢复: "Restore",
  "已保存。请在游戏中手动加载；备份已保留。":
    "Saved. Load it manually in the game; the backup has been kept.",
  "已保存为 {name}。请在游戏中手动加载；备份已保留。":
    "Saved as {name}. Load it manually in the game; the backup has been kept.",
  "刷新界面失败：{error}。备份：{backup}":
    "Could not refresh the interface: {error}. Backup: {backup}",
  "恢复已完成，刷新界面失败：{error}":
    "Restore completed, but the interface could not refresh: {error}",
  "操作未完成，请查看原始详情。":
    "The operation could not finish. See original details below.",
  原始详情: "Original details",
  游戏文件提示: "Game file message",
  "正在准备，请查看进度详情。": "Preparing; see progress details below.",
  "正在检测 Steam 游戏库与存档目录…":
    "Detecting Steam libraries and save folders…",
  "正在准备本机环境…": "Preparing local setup…",
  首次启动向导: "First-run setup",
  "确认位置，即可开始改装。": "Confirm locations to get started.",
  "存档解码器已经内置。请选择本机的游戏和用户数据目录，工具会自动完成其余准备。":
    "The save decoder is built in. Select your game and user data folders; the remaining setup is automatic.",
  "1 检测路径": "1 Detect folders",
  "2 确认并准备": "2 Confirm and prepare",
  "3 打开车库": "3 Open garage",
  "包含 def.scs 的 Euro Truck Simulator 2 文件夹":
    "Euro Truck Simulator 2 folder containing def.scs",
  已检测到: "Detected",
  "个安装位置，可手动修改。": "installations. You can edit the path manually.",
  选择游戏安装位置: "Select game installation",
  "检测到多个安装位置，请选择或手动填写":
    "Multiple installations found; choose one or enter a path",
  "通常为文档中的 Euro Truck Simulator 2 文件夹":
    "Usually the Euro Truck Simulator 2 folder in Documents",
  "包含 profiles、steam_profiles 或 config.cfg，通常与游戏安装目录不同。":
    "Contains profiles, steam_profiles or config.cfg; usually different from the installation folder.",
  选择用户数据位置: "Select user data folder",
  "检测到多个用户数据位置，请选择或手动填写":
    "Multiple user data folders found; choose one or enter a path",
  "内置存档解码器，无需 Truck Tools":
    "Built-in save decoder; no Truck Tools needed",
  "自动从 SCS 官方获取解包工具并校验":
    "Extractor downloaded from SCS and verified automatically",
  确认前不修改设置或游戏存档:
    "Settings and saves stay unchanged until confirmation",
  "仅支持原版及官方 DLC，不支持 Mod 存档。首次准备需要网络及用于解包缓存的磁盘空间。":
    "Supports the base game and official DLC only, not modded saves. Initial setup needs internet access and disk space for the extraction cache.",
  重新检测: "Detect again",
  确认路径并准备: "Confirm and prepare",
};

Object.assign(english, {
  选择检测到的位置: "Select a detected location",
  备份与恢复: "Backups & restore",
  "保存尚未完成，备份状态待确认。请在备份与恢复中查看记录。":
    "Saving is not complete and backup status needs confirmation. Check the record in Backups & restore.",
  原存档已备份: "Original save backed up",
  "最近一次保存前的原存档，可从此恢复。":
    "Restore the original save from before your most recent save.",
  恢复此备份: "Restore this backup",
  查看全部备份: "View all backups",
  "尚未写入，保存时自动备份原存档":
    "No changes written. The original save is backed up when you save.",
  "每次保存前自动备份原存档。选择一条记录，即可恢复到这次修改前。":
    "The original save is backed up before each save. Choose a record to restore the save from before those changes.",
  "保存未完成或写入未确认，备份状态待确认。恢复时会检查备份与目标内容，拒绝覆盖新的进度。":
    "Saving is incomplete or writing is unconfirmed. Backup status needs confirmation. Restore checks the backup and target, and refuses to overwrite newer progress.",
  检查游戏保存后的改装是否保留:
    "Check whether modifications survived an in-game save",
  "在游戏中加载改装存档，再保存一次，然后选择该存档进行检查。此检查只读取配件，不会恢复备份或修改存档。":
    "Load the modified save in the game and save again, then select that save to check it. This only reads parts; it does not restore a backup or change the save.",
  游戏保存后的存档: "Save created after saving in the game",
  检查所选存档: "Check selected save",
  游戏保存检查结果: "In-game save check results",
  还没有备份: "No backups yet",
  "选择配件、加入变更清单时不会创建备份。确认保存后，工具会先备份原存档，再写入改装；备份记录会显示在这里。":
    "Choosing parts and staging changes does not create a backup. When you confirm saving, the original save is backed up before modifications are written. Backup records will appear here.",
  变速器: "Transmission",
  喷漆: "Paint job",
  防撞杠: "Bull bar",
  主后视镜: "Main mirrors",
  "车型：{model}": "Truck: {model}",
  车型未标明: "Truck model unspecified",
  来源未知: "Unknown source",
  "发动机 · 变体 {id}": "Engine · variant {id}",
  "{name} 底盘": "{name} chassis",
  "{name} · 车辆基础": "{name} · base vehicle",
  "变体 {id}": "Variant {id}",
  前进挡: "Forward gears",
  主减速比: "Differential ratio",
  缓速器: "Retarder",
  未标明: "Unspecified",
  无: "None",
  "原配件定义未知，仅可查看":
    "The current part definition is unknown. View only.",
  "此核心部件暂不支持替换，仅可查看与比较":
    "This core component cannot be replaced yet. Viewing and comparison are available.",
  当前仅支持柴油发动机之间替换:
    "Only diesel-to-diesel engine replacement is supported.",
  "此配件类型暂不支持替换，仅可查看":
    "This part type cannot be replaced yet. View only.",
  "此类别不可追加；核心部件不能重复安装":
    "This category cannot be added. Core components cannot be duplicated.",
  "候选配件定义未知，仅可查看":
    "The candidate definition is unknown. View only.",
  候选配件类型与安装位置不匹配:
    "The candidate type or mounting position does not match.",
  此外观件的车型不匹配:
    "This exterior part belongs to a different truck model.",
  请先选择配件: "Select a part first.",
  "共 {count} 项候选": "{count} candidates",
  "共 {count} 项": "{count} items",
  "更新配件目录即可读取游戏内的中英文名称。现有存档不受影响。":
    "Rebuild the parts catalog to load in-game English and Chinese names. Existing saves are unaffected.",
  前往设置: "Open settings",
  目录分类: "Catalog category",
  "车型专属件优先，其次为共享件，最后为未识别车型。共享并不代表适合所有卡车。":
    "Truck-specific parts appear first, followed by shared parts and unidentified models. Shared does not mean compatible with every truck.",
  共享件: "Shared part",
  车型专属: "Truck-specific",
  未识别车型: "Unidentified truck",
  游戏内名称: "In-game name",
  名称待解析: "Name unresolved",
  原始名称: "Original name",
  来自本机游戏定义: "From installed game definitions",
  "没有匹配的配件。请调整搜索词或分类。":
    "No matching parts. Adjust the search or category.",
  游戏名称未解析: "In-game name unresolved",
  参数未知: "Unknown specifications",
  功率未知: "Power unknown",
  扭矩未知: "Torque unknown",
  标称扭矩转速范围: "Rated torque RPM range",
  附加扭矩: "Secondary torque",
});

Object.assign(english, {
  "旧名称键缺失；名称已按同模型、同图标的游戏配件核对。":
    "The old name key is missing; the name was verified against a game part with the same model and icon.",
  目录提示: "Catalog notices",
  "请先保存或移除待保存修改，再更新目录。":
    "Save or remove the staged changes before rebuilding the catalog.",
  "匹配 {count} 辆，请选择目标车辆":
    "{count} trucks match. Choose the target truck.",
  前轮轮辋: "Front rims",
  后轮轮辋: "Rear rims",
  前轮: "Front wheels",
  后轮: "Rear wheels",
  徽标: "Badge",
  底盘徽标: "Chassis badges",
  贴花: "Decal",
  挂车电缆: "Trailer cables",
});

/** Unknown game text stays intact instead of guessing a translation. */
export function translate(
  locale: Locale,
  text: string,
  values: TextValues = {},
): string {
  const template = locale === "en" ? (english[text] ?? text) : text;
  return template.replace(/\{(\w+)\}/g, (match, key: string) =>
    String(values[key] ?? match),
  );
}

export function storedLocale(storage: Pick<Storage, "getItem">): Locale {
  try {
    return storage.getItem(LANGUAGE_STORAGE_KEY) === "en" ? "en" : "zh-CN";
  } catch {
    return "zh-CN";
  }
}
