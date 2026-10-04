# P4 配件分类 key 只读核对

2026-10-03。输入为 `verification/p4-names/coverage.json`、`catalog.json`，以及与 `LOCALE_RESEARCH.md` 同 SHA256 的本机 `locale.scs`。本报告没有修改源码或游戏文件。

117 个现有类别中，98 个有同名有效游戏 key；这里包含 `unknown`，它只是通用“未知”文本，不代表某个安装位置。9 个其余类别有明确的部件类型别名，另外10个需要保留产品文案或只使用上层类别，不能声称存在专属游戏位置名。核心部件继续独立分组，内/外饰分组是基于配件位置的应用建议，不声称它本身是游戏语言表里的分类树。

`f_intake_chs` 和 `r_chs_cover` 都有同名 key，简中分别是“前面罩底部”和“底盘防护罩”。无需猜测名称或写成“未识别配件”。`f_intake_cab` 为“前面罩”，`f_inlay_chs` 为“前格栅底部”，两者不能互换。

## 全部现有类别

只列 key 与位置，不复制整库翻译。“有”指中英两份当前有效词表均存在，不含注释行。“别名”只提供有部件类型证据的替代；表中备注明确区分通用家族名称和专属位置名称。

| catalog 类别 | 同名有效 key | 可用 key | 建议组 | 证据或边界 |
| --- | --- | --- | --- | --- |
| `air_tank` | 有 | `air_tank` | 外饰配件 | 当前缺 label |
| `b_detector` | 有 | `b_detector` | 外饰配件 | 当前缺 label |
| `b_grill` | 有 | `b_grill` | 外饰配件 | — |
| `badge` | 有 | `badge` | 外饰配件 | — |
| `badge_a` | 无 | `—` | 外饰配件 | 当前缺 label；无专属 key；仅可用 badge 作为徽标大类，不证明子位置 |
| `badge_b` | 无 | `—` | 外饰配件 | 当前缺 label；无专属 key；仅可用 badge 作为徽标大类，不证明子位置 |
| `badge_c` | 无 | `—` | 外饰配件 | 当前缺 label；无专属 key；仅可用 badge 作为徽标大类，不证明子位置 |
| `beacon` | 有 | `beacon` | 外饰配件 | — |
| `c_badge` | 无 | `—` | 外饰配件 | 当前缺 label；无专属 key；仅可用 badge 作为徽标大类，不能把 c 猜成位置 |
| `c_grill` | 有 | `c_grill` | 外饰配件 | 当前缺 label |
| `cab_door` | 有 | `cab_door` | 外饰配件 | 当前缺 label |
| `cab_doorstep` | 有 | `cab_doorstep` | 外饰配件 | 当前缺 label |
| `cabin` | 有 | `cabin` | 核心/cabin | — |
| `carpet` | 有 | `carpet` | 内饰配件 | — |
| `chassis` | 有 | `chassis` | 核心/chassis | — |
| `chs_badges` | 无 | `—` | 外饰配件 | 当前缺 label；无专属 key；仅可用 badge 作为徽标大类 |
| `codrv_plate` | 有 | `codrv_plate` | 内饰配件 | — |
| `codrv_seat` | 有 | `codrv_seat` | 内饰配件 | — |
| `cor_def` | 有 | `cor_def` | 外饰配件 | 当前缺 label |
| `cup_holder` | 有 | `cup_holder` | 内饰配件 | — |
| `curtain_f` | 有 | `curtain_f` | 内饰配件 | — |
| `decals` | 有 | `decals` | 外饰配件 | 当前缺 label |
| `doorhndl` | 有 | `doorhndl` | 外饰配件 | 当前缺 label |
| `doorstep` | 有 | `doorstep` | 外饰配件 | 当前缺 label |
| `doortrim` | 有 | `doortrim` | 外饰配件 | 当前缺 label |
| `drv_plate` | 有 | `drv_plate` | 内饰配件 | — |
| `engine` | 有 | `engine` | 核心/engine | — |
| `exhaust_l` | 有 | `exhaust_l` | 外饰配件 | — |
| `exhaust_m` | 有 | `exhaust_m` | 外饰配件 | — |
| `exhaust_r` | 有 | `exhaust_r` | 外饰配件 | — |
| `f_badge` | 有 | `f_badge` | 外饰配件 | 当前缺 label |
| `f_bumper` | 有 | `f_bumper` | 外饰配件 | — |
| `f_cab_trim` | 有 | `f_cab_trim` | 外饰配件 | 当前缺 label |
| `f_chs_logo` | 有 | `f_chs_logo` | 外饰配件 | 当前缺 label |
| `f_disc` | 无 | `disc` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `f_equip` | 有 | `f_equip` | 外饰配件 | 当前缺 label |
| `f_fender` | 有 | `f_fender` | 外饰配件 | 当前缺 label |
| `f_fender_cab` | 有 | `f_fender_cab` | 外饰配件 | 当前缺 label |
| `f_fender_chs` | 有 | `f_fender_chs` | 外饰配件 | 当前缺 label |
| `f_fendr_chs` | 有 | `f_fendr_chs` | 外饰配件 | 当前缺 label |
| `f_grill` | 有 | `f_grill` | 外饰配件 | — |
| `f_hub` | 无 | `hub` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `f_inlay_cab` | 有 | `f_inlay_cab` | 外饰配件 | 当前缺 label |
| `f_inlay_chs` | 有 | `f_inlay_chs` | 外饰配件 | 当前缺 label |
| `f_intake_bar` | 有 | `f_intake_bar` | 外饰配件 | 当前缺 label |
| `f_intake_cab` | 有 | `f_intake_cab` | 外饰配件 | 当前缺 label |
| `f_intake_chs` | 有 | `f_intake_chs` | 外饰配件 | 当前缺 label |
| `f_intk_b_cab` | 有 | `f_intk_b_cab` | 外饰配件 | 当前缺 label |
| `f_intk_b_chs` | 无 | `—` | 外饰配件 | 当前缺 label；无确证别名；f_intk_b_cab 指驾驶室件，不能当作同一位置 |
| `f_light_bmp` | 有 | `f_light_bmp` | 外饰配件 | 当前缺 label |
| `f_light_chs` | 有 | `f_light_chs` | 外饰配件 | 当前缺 label |
| `f_light_mid` | 有 | `f_light_mid` | 外饰配件 | 当前缺 label |
| `f_light_top` | 有 | `f_light_top` | 外饰配件 | 当前缺 label |
| `f_logo` | 有 | `f_logo` | 外饰配件 | 当前缺 label |
| `f_mirror` | 有 | `f_mirror` | 外饰配件 | — |
| `f_mudflap` | 有 | `f_mudflap` | 外饰配件 | — |
| `f_nuts` | 无 | `nuts` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `f_tire` | 无 | `tire` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `f_turn_light` | 有 | `f_turn_light` | 外饰配件 | 当前缺 label |
| `f_wnd_frame` | 有 | `f_wnd_frame` | 外饰配件 | 当前缺 label |
| `filter` | 有 | `filter` | 外饰配件 | 当前缺 label |
| `frntglss_mid` | 有 | `frntglss_mid` | 内饰配件 | 当前缺 label；挡风玻璃中央饰品，内外皆可见；建议归内饰 |
| `head_light` | 无 | `head_lights` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `hl_guard` | 有 | `hl_guard` | 外饰配件 | 当前缺 label |
| `int_display` | 无 | `—` | 内饰配件 | 无专属 key；数字显示组件，不能借用设置页 display_t |
| `interior` | 有 | `interior` | 核心/interior | — |
| `intlight_bck` | 有 | `intlight_bck` | 内饰配件 | — |
| `intlight_bgr` | 有 | `intlight_bgr` | 内饰配件 | — |
| `l_horn` | 有 | `l_horn` | 外饰配件 | 当前缺 label |
| `l_pillow` | 有 | `l_pillow` | 内饰配件 | — |
| `mirror` | 有 | `mirror` | 外饰配件 | — |
| `p_decal` | 无 | `—` | 外饰配件 | 当前缺 label；可用 decals 作为贴花大类；此处为100周年涂装附属件 |
| `paint_job` | 有 | `paint_job` | 核心/paint_job | 同名 key 是原厂配色，整个涂装区标题宜保留产品文案 |
| `r_bumper` | 有 | `r_bumper` | 外饰配件 | — |
| `r_chs_cover` | 有 | `r_chs_cover` | 外饰配件 | 当前缺 label |
| `r_deflector` | 有 | `r_deflector` | 外饰配件 | 当前缺 label |
| `r_disc` | 无 | `disc` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `r_fender` | 有 | `r_fender` | 外饰配件 | — |
| `r_fendr_top` | 有 | `r_fendr_top` | 外饰配件 | 当前缺 label |
| `r_grill` | 有 | `r_grill` | 外饰配件 | — |
| `r_horn` | 有 | `r_horn` | 外饰配件 | 当前缺 label |
| `r_hub` | 无 | `hub` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `r_light` | 有 | `r_light` | 外饰配件 | 当前缺 label |
| `r_mudflap` | 有 | `r_mudflap` | 外饰配件 | — |
| `r_nuts` | 无 | `nuts` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `r_tire` | 无 | `tire` | 外饰配件 | 明确部件类型；前/后轴保留在应用位置元数据，不拼造游戏原文 |
| `rear_window` | 有 | `rear_window` | 外饰配件 | 当前缺 label；驾驶室后窗结构件，内外皆可见；建议归外饰 |
| `s_badge` | 有 | `s_badge` | 外饰配件 | 当前缺 label |
| `s_deflector` | 有 | `s_deflector` | 外饰配件 | 当前缺 label |
| `s_equip` | 有 | `s_equip` | 外饰配件 | 当前缺 label |
| `s_guard` | 有 | `s_guard` | 外饰配件 | 当前缺 label |
| `s_mirror` | 有 | `s_mirror` | 外饰配件 | — |
| `s_panel` | 有 | `s_panel` | 外饰配件 | 当前缺 label |
| `s_reflector` | 有 | `s_reflector` | 外饰配件 | 当前缺 label |
| `s_strip` | 有 | `s_strip` | 外饰配件 | 当前缺 label |
| `set_cuphold` | 有 | `set_cuphold` | 内饰配件 | — |
| `set_dashbrd` | 有 | `set_dashbrd` | 内饰配件 | — |
| `set_glass` | 有 | `set_glass` | 内饰配件 | — |
| `set_lglass` | 有 | `set_lglass` | 内饰配件 | — |
| `sideskirt` | 有 | `sideskirt` | 外饰配件 | — |
| `sideskrt_bar` | 有 | `sideskrt_bar` | 外饰配件 | — |
| `steering_w` | 有 | `steering_w` | 内饰配件 | — |
| `sunshield` | 有 | `sunshield` | 外饰配件 | — |
| `sunshld` | 有 | `sunshld` | 外饰配件 | — |
| `tank` | 有 | `tank` | 外饰配件 | — |
| `toyac` | 有 | `toyac` | 内饰配件 | — |
| `toybed` | 有 | `toybed` | 内饰配件 | — |
| `toybig` | 有 | `toybig` | 内饰配件 | — |
| `toyhang` | 有 | `toyhang` | 内饰配件 | — |
| `toypanel` | 有 | `toypanel` | 内饰配件 | — |
| `toyseat` | 有 | `toyseat` | 内饰配件 | — |
| `toystand` | 有 | `toystand` | 内饰配件 | — |
| `transmission` | 有 | `transmission` | 核心/transmission | — |
| `trlr_cables` | 无 | `—` | 外饰配件 | 当前缺 label；trailer_cables_mode 的值是挂车电缆，但它是设置标签，非配件位置 key |
| `unknown` | 有 | `unknown` | 待按路径拆回外饰轮组 | 不得直接盖过可从路径恢复的74个轮组类别 |
| `vehicle` | 无 | `—` | 其他 | 车辆基础记录，不是配件位置；保留产品文案 |
| `windowtrim` | 有 | `windowtrim` | 外饰配件 | 当前缺 label |

## 别名的具体依据

`f_disc` / `r_disc` → `disc`，`f_hub` / `r_hub` → `hub`，`f_nuts` / `r_nuts` → `nuts`，`f_tire` / `r_tire` → `tire`：前缀表示前后轴安装位置，词表的对应类型项后有 Wheel rim / Wheel hub / Wheel nuts / Wheel tyre 注释。可以将轴位置保存在类别元数据，类型名称取游戏词表；不要声称手工拼成的“前轮轮辋”是一个游戏原始字符串。`disc` 的官方简中为“轮辋”，不能继续写“轮盘”。

`head_light` → `head_lights`：`head_light` 目录下定义类型为 `accessory_head_lights_data`，其已解析变体使用 `head_light_halogen_lamps` / `head_light_xenon_lamps` / `head_light_led`；有效类别 key `head_lights` 对应 Headlights / 前大灯。这里有安装目录、定义类型和变体 token 三重一致证据。

`badge_a`、`badge_b`、`badge_c`、`c_badge`、`chs_badges` 都没有专属类别 key；它们的定义多数也没有 `name`。可用现存 `badge` 显示“徽标”家族并保留原始 slot，不把 a/b/c 推断成前/中/后。`p_decal` 的两项通过 `suitable[]` 指向 Volvo FH6 100 周年 `paint_job`，可用 `decals` 标记贴花家族；这仍不是专属 slot 别名。`trlr_cables` 的定义类型为 `accessory_addon_trailer_cables_data`，确实是挂车电缆组件；词表有 `trailer_cables_mode`，但其注释明确用于设置下拉框，宜只把“挂车电缆”作为有游戏用词依据的产品类别，而不要复用设置项语义。

`int_display` 在当前样本中为 Volvo FH6 的 `digital.sii` / `empty.sii`，数字件类型是 `accessory_addon_int_ui_data`，含车内模型；可归内饰，但没有已核实的专属类别 key。`f_intk_b_chs` 当前是 Scania 2016 外部附属件，无 name、无同名 key；`f_intk_b_cab` 虽然名称相似，但明确指另一位置，不能作为已证实的等价 key。

## 74 个 unknown 其实可从路径恢复

覆盖报告的卡车可选范围里，`unknown` 的 74 条全部属于轮组：57 条 `accessory_rim_data`、17 条 `accessory_wheel_data`。`catalog::category` 固定列表漏掉了 `f/r_cover`、`f/r_rim` 和 `f/r_wheel`，而这些路径位于共享 `/def/vehicle/` 下，不会进入 `/truck/` 后备分支。

| 真实路径类别 | 名称 key | 位置/分组 |
| --- | --- | --- |
| `f_cover` / `r_cover` | `cover` | 前/后轴毂盖，外饰轮组 |
| `f_rim` / `r_rim` | `rim` | 前/后轴轮辋，外饰轮组 |
| `f_wheel` / `r_wheel` | `f_wheel` / `r_wheel` | 前轮 / 后轮，外饰轮组 |

证据样本：`/def/vehicle/f_cover/front_hub_cover_03.sii` 为 `accessory_rim_data`，name token 为 `fcover_pacific_solid`；`/def/vehicle/f_wheel/3.sii` 为 `accessory_wheel_data`，name token 为 `fwheel_absolute_fury`。两者本名都已成功解析，不该再显示“未识别配件”。这个结论只针对报告的卡车可选范围；原始完整 catalog 还含 AI、货物和挂车的 unknown，不可一律归外饰。

## 内外饰分组的边界

`frntglss_mid` 是挡风玻璃中央的装饰/LED 物品，样本有 interior_model 与 exterior_model，放内饰配件更便于用户寻找；它并非因为类型为 `accessory_addon_data` 就必然属于外饰。同样 `intlight_bck`、`intlight_bgr` 两者可在内外视角看见，但属于车内灯板/背景灯。`rear_window` 是驾驶室后窗结构件，含两套视角模型，建议归外饰；这是一项产品分组选择。`vehicle` 是基础记录，不应伪装成一个可选装饰位置。

当前还有几项已有 label 与游戏原文有出入：`exhaust_m` 是 Rear Exhaust / 后排气管；`f_mudflap`、`r_mudflap` 是前/后挡泥帘，`f_fender`、`r_fender` 才是前/后挡泥板；`b_grill` 是下格栅护栏；`f_grill` 是防撞杠；`r_grill` 是灯架。若保持某些通俗产品标签，来源上应与游戏原文区分。

## 20 条非空未解析 name 的调查

20 条对应 10 个不同 raw token，所有这些原 token 在本机中英文主字典中均不存在，也未在检查过的 `localization.sui` / `accessories.sui` 注释区找到原 token。下面列出存在的对应候选，但把“同资源直接证据”和“同一家族/语义候选”分开；没有把相似拼写当作游戏已声明的 alias。

| 原 token（省略 @@） | 条数 | 有效候选 key | 证据强度与处理建议 |
| --- | ---: | --- | --- |
| `r_bumper_thor` | 4 | `bumper_thor` | 有效值 Thor；多个其他车型后保险杠使用此 key。当前4条没有找到相同模型+icon的已解析定义，属于同家族候选，非游戏已声明别名 |
| `r_bumper_centurion` | 4 | `bumper_centurion` | 有效值 Centurion；Scania S 2016 的 `rearbumper_mg_02.sii`、DAF XF 的 `chrome.sii` 使用此 key。同系列后保险杠候选，不等于当前4条的精确资源证据 |
| `r_bumper_diva` | 2 | `bumper_diva` | 有效值 Diva；Renault T、Volvo FH2012 等后保险杠使用此 key。当前两条没有同模型+icon的已解析对照 |
| `r_bumper_legatus` | 2 | `bumper_legatus` | 字典有效值 Legatus，但当前 catalog 没有任何定义直接使用新 key；只能列为候选，不能称已确认迁移关系 |
| `exhaust_stock` | 1 | `exhaust_l_stock` | Stock / 原厂；未解析件位于 DAF XF 左排气管 `shape14.sii`，Iveco 左排气管同系列使用新 key，但不是同一个模型资源。不能仅因 Stock 通用就改成直接解析成功 |
| `Holland Style Retro paint` | 1 | `r_grill_holland_style_lightbox` | 有效新 key 在其他车型 `dutch_lightbox_retro_01.sii` 使用；MAN 当前模型独立，未发现相同资源的已解析定义。可列窄范围候选 |
| `Pure Dutch Steel Modern Roofbar` | 1 | `r_grill_pure_dutch_steel_lightbox_roofbar` | **同 exterior_model、icon 的已解析对照存在**：Volvo FH2021、FH2024；有确证可作该具体路径的兼容 alias |
| `Dutch Holland On Wheels Modern Roofbar` | 1 | `r_grill_holland_on_wheels_lightbox_roofbar` | **同 exterior_model、icon 的已解析对照存在**：Volvo FH2021、FH2024；有确证可作该具体路径的兼容 alias |
| `Dutch Power` | 3 | `r_grill_dutch_power_lightbox` | Volvo FH2012 与 FH2021 两条有 FH2024 同模型+icon对照；MAN 一条仅是同系列/名称候选。不能把同一个 raw token 的3条都算同证据级别 |
| `Dutch Holland On Wheels Modern` | 1 | `r_grill_holland_on_wheels_lightbox` | 对应各车型 Modern 第二外观灯箱的有效名称；MAN 独立模型未找到同资源已解析对照，保留候选状态 |

同资源核对除了 exterior_model、icon，也检查显式 `look`、`variant` 字段；上面的 4 条 Volvo 强证据样本中它们一致（均未显式填写）。这不代表所有不同车型相同文件名都同物品：例如 Volvo FH2012 的 `retro_01` 已用 Dutch Power，而若干其他车型的 `retro_01` 使用 Holland Style；因此不能建立只看文件 stem 的统一修复规则。

### 4 条可确认的精确路径 alias

共同前缀 `/def/vehicle/truck/`，下面每条只应在原始 raw token 仍与表中一致时生效，并记录兼容别名来源；不要覆盖未来游戏更新已经修复的 name。

| 路径后缀 | 原 token | 新 key |
| --- | --- | --- |
| `volvo.fh16_2012/accessory/r_grill/dutch_lightbox_modern_roofbar_01.sii` | `Pure Dutch Steel Modern Roofbar` | `r_grill_pure_dutch_steel_lightbox_roofbar` |
| `volvo.fh16_2012/accessory/r_grill/dutch_lightbox_modern_roofbar_02.sii` | `Dutch Holland On Wheels Modern Roofbar` | `r_grill_holland_on_wheels_lightbox_roofbar` |
| `volvo.fh16_2012/accessory/r_grill/dutch_lightbox_retro_02.sii` | `Dutch Power` | `r_grill_dutch_power_lightbox` |
| `volvo.fh_2021/accessory/r_grill/dutch_lightbox_retro_02.sii` | `Dutch Power` | `r_grill_dutch_power_lightbox` |

对照定义位于 `volvo.fh_2024/accessory/r_grill/` 的同名文件；第一、二条还可与 FH2021 同名文件交叉核对。它们使用 `/vehicle/truck/volvo_fh16_2012/accessory/r_grill/` 下同一模型，icon 的 `look2` 差异也严格匹配。其余16条没有达到这个强度，保留未解析标记与候选研究结果更准确；本报告没有用人工译名填补它们。

## Wiki 核对结果

已检查用户指定 wiki 的 [Mighty Griffin 页面](https://wikiwiki.jp/ets2steam/DLC/MightyGriffinTuningPack)与 [XF Tuning 页面](https://wikiwiki.jp/ets2steam/DLC/DAF-TuningPack)，它们确认后保险杠、底盘罩、排气、格栅等配件范围，但所读取正文没有 Thor/Centurion/Diva/Legatus 的逐个游戏菜单名称证据。不能据此升级上述候选为已验证别名。

[2026 年新闻页的 Holland Style 发布段落](https://wikiwiki.jp/ets2steam/%E3%83%87%E3%83%BC%E3%82%BF/NEWS2026-2)可核对该 DLC 确有灯箱与内外装配件，没有把本报告的旧 token 与新 key 逐条对应；独立 HollandTuningPack 页面本次未能读取。故这20条名称的主要可复现证据来自用户本机安装的定义、模型引用和语言表，Wiki 只作配件范围核对。
