use crate::{
    catalog::Catalog,
    garage::inventory,
    sii::{quoted, Document},
    Result,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Operation {
    pub truck_id: String,
    pub action: String,
    pub accessory_id: Option<String>,
    pub candidate_path: String,
    pub donor_accessory: Option<String>,
}
enum EditAction<'a> {
    Replace(&'a str),
    Add(&'a str),
}
impl Operation {
    fn validated_action(&self) -> Result<EditAction<'_>> {
        match self.action.as_str() {
            "replace" => self
                .accessory_id
                .as_deref()
                .filter(|id| !id.is_empty())
                .map(EditAction::Replace)
                .ok_or_else(|| "未选择被替换配件".into()),
            "add" if self.accessory_id.is_none() => self
                .donor_accessory
                .as_deref()
                .filter(|id| !id.is_empty())
                .map(EditAction::Add)
                .ok_or_else(|| "追加外观件必须从现有车辆选择供体，以验证安装配置".into()),
            "add" => Err("追加操作不能同时指定被替换配件".into()),
            _ => Err("未知操作".into()),
        }
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Change {
    pub truck_id: String,
    pub model: String,
    pub plate: String,
    pub category: String,
    pub action: String,
    pub before: String,
    pub after: String,
    #[serde(default)]
    pub position: std::collections::BTreeMap<String, String>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Preview {
    pub changes: Vec<Change>,
    pub warnings: Vec<String>,
    pub trucks: Vec<crate::garage::Truck>,
}
const REPLACE: &[&str] = &[
    "engine",
    "transmission",
    "tank",
    "f_tire",
    "r_tire",
    "f_disc",
    "r_disc",
    "f_hub",
    "r_hub",
    "f_nuts",
    "r_nuts",
];
const SINGLE: &[&str] = &[
    "engine",
    "transmission",
    "chassis",
    "cabin",
    "interior",
    "vehicle",
    "tank",
    "paint_job",
];
const ADD: &[&str] = &["beacon", "r_grill", "f_grill", "sunshld", "sunshield"];
fn position(doc: &Document, id: &str) -> std::collections::BTreeMap<String, String> {
    let mut result = std::collections::BTreeMap::new();
    if let Ok(u) = doc.unit(id) {
        for f in &u.fields {
            if f.key == "offset" || f.key == "wheel_index" || f.key.starts_with("slot_name[") {
                result.insert(f.key.clone(), f.value.clone());
            }
        }
    }
    result
}
fn check_existing(
    truck: &crate::garage::Truck,
    old_names: &[String],
    new_names: &[String],
    replaced: Option<&str>,
) -> Result<()> {
    for a in &truck.accessories {
        if Some(a.id.as_str()) == replaced {
            continue;
        }
        if let Some(d) = &a.definition {
            let old_violations = violations(d, old_names);
            if violations(d, new_names)
                .difference(&old_violations)
                .next()
                .is_some()
            {
                return Err(format!(
                    "该操作会破坏现有配件 {} 的适配关系或触发冲突",
                    a.name
                ));
            }
        }
    }
    Ok(())
}
fn violations(
    def: &crate::catalog::Definition,
    units: &[String],
) -> std::collections::BTreeSet<String> {
    let mut out = std::collections::BTreeSet::new();
    if !def.suitable.is_empty()
        && !def
            .suitable
            .iter()
            .any(|p| units.iter().any(|u| wildcard(p, u)))
    {
        out.insert("suitable_for".into());
    }
    for p in &def.conflicts {
        for u in units {
            if wildcard(p, u) {
                out.insert(format!("conflict:{p}:{u}"));
            }
        }
    }
    for r in &def.requires {
        if !units.iter().any(|u| u.rsplit('.').next() == Some(r)) {
            out.insert(format!("require:{r}"));
        }
    }
    out
}
fn wildcard(pattern: &str, value: &str) -> bool {
    let regex = format!("^{}$", regex::escape(pattern).replace("\\*", ".*"));
    regex::Regex::new(&regex)
        .map(|r| r.is_match(value))
        .unwrap_or(false)
}
fn compatible(def: &crate::catalog::Definition, units: &[String]) -> bool {
    violations(def, units).is_empty()
}
pub fn apply(
    doc: &Document,
    catalog: &Catalog,
    operations: &[Operation],
) -> Result<(Document, Preview)> {
    if operations.len() > 100 {
        return Err("一次最多修改 100 项".into());
    }
    let mut result = doc.clone();
    let mut changes = Vec::new();
    let mut warnings = Vec::new();
    let mut trucks = inventory(&result, catalog)?;
    let mut truck_indices: HashMap<String, Vec<usize>> = HashMap::new();
    let mut accessory_owner = HashMap::new();
    for (index, truck) in trucks.iter().enumerate() {
        truck_indices
            .entry(truck.id.clone())
            .or_default()
            .push(index);
        for accessory in &truck.accessories {
            accessory_owner.entry(accessory.id.clone()).or_insert(index);
        }
    }
    for op in operations {
        let action = op.validated_action()?;
        let indices = truck_indices
            .get(&op.truck_id)
            .ok_or("目标不属于玩家车库")?;
        let truck = &trucks[indices[0]];
        let def = catalog
            .definitions
            .get(&op.candidate_path)
            .ok_or("目录中找不到该配件。请先建立当前游戏的配件目录；未知配件只读")?;
        let cat = &def.category;
        let unit_names: Vec<_> = truck
            .accessories
            .iter()
            .filter_map(|a| a.definition.as_ref().map(|d| d.unit.clone()))
            .collect();
        let mut before = String::new();
        let mut location = std::collections::BTreeMap::new();
        if let EditAction::Replace(id) = action {
            let old =
                truck.accessories.iter().find(|a| a.id == id).ok_or(
                    "该配件不属于目标卡车；若它来自待保存的追加操作，请先移除依赖它的修改",
                )?;
            if old.category != *cat {
                return Err("只能替换相同类别与安装位置的配件".into());
            }
            let old_def = old.definition.as_ref().ok_or("原配件定义未知，禁止修改")?;
            if old_def.kind != def.kind {
                return Err("配件定义类型不匹配".into());
            }
            let core = REPLACE.contains(&cat.as_str());
            let mut after_names: Vec<_> = truck
                .accessories
                .iter()
                .filter(|a| a.id != id)
                .filter_map(|a| a.definition.as_ref().map(|d| d.unit.clone()))
                .collect();
            after_names.push(def.unit.clone());
            if !core
                && !(old.kind == "vehicle_addon_accessory"
                    && def.model == truck.model
                    && compatible(def, &after_names))
            {
                return Err("该类别暂不支持替换，或外观件车型/依赖不匹配".into());
            }
            if !core {
                check_existing(truck, &unit_names, &after_names, Some(id))?;
            }
            if SINGLE.contains(&cat.as_str()) && !core {
                return Err("首版只开放发动机、变速箱、独立油箱的核心部件替换".into());
            }
            if cat == "engine"
                && (def
                    .metrics
                    .get("type")
                    .map(String::as_str)
                    .unwrap_or("diesel")
                    != "diesel"
                    || old_def
                        .metrics
                        .get("type")
                        .map(String::as_str)
                        .unwrap_or("diesel")
                        != "diesel")
            {
                return Err("首版仅支持柴油发动机之间替换".into());
            }
            if result
                .units
                .iter()
                .flat_map(|u| &u.fields)
                .filter(|f| f.value == *id)
                .count()
                != 1
            {
                return Err("该配件被多个对象引用，不能直接修改".into());
            }
            if core && !compatible(def, &unit_names) {
                warnings.push(format!(
                    "{}：跨品牌定义不符合原厂适配规则；结构可保存，仍需游戏内验证",
                    def.name
                ));
            }
            before = old.path.clone();
            location = position(&result, id);
            result = result.replace(id, "data_path", quoted(&op.candidate_path)?)?;
        } else if let EditAction::Add(donor_id) = action {
            if SINGLE.contains(&cat.as_str()) {
                return Err(format!(
                    "禁止追加 {cat}：核心配件只能替换，重复底盘已在实测中导致崩溃"
                ));
            }
            if !ADD.contains(&cat.as_str()) {
                return Err(
                    "首版只支持经供体验证的 beacon、r_grill、f_grill、sunshld 外观类别追加".into(),
                );
            }
            if truck.accessories.iter().any(|a| a.category == *cat) {
                return Err("该安装类别已被占用，请使用替换".into());
            }
            let donor = &trucks[*accessory_owner
                .get(donor_id)
                .ok_or("供体配件不属于自有车辆")?];
            let source = donor.accessories.iter().find(|a| a.id == donor_id).unwrap();
            if source.path != op.candidate_path || source.kind != "vehicle_addon_accessory" {
                return Err("供体路径或类型不匹配".into());
            }
            if donor.model != truck.model
                || def.model != truck.model
                || !compatible(def, &unit_names)
            {
                return Err("外观件要求相同车型，且满足定义中的安装依赖".into());
            }
            let mut after_names = unit_names.clone();
            after_names.push(def.unit.clone());
            check_existing(truck, &unit_names, &after_names, None)?;
            for c in ["cabin", "chassis"] {
                let a = truck
                    .accessories
                    .iter()
                    .find(|a| a.category == c)
                    .map(|a| &a.path);
                let b = donor
                    .accessories
                    .iter()
                    .find(|a| a.category == c)
                    .map(|a| &a.path);
                if a.is_none() || a != b {
                    return Err("追加外观件要求供体与目标驾驶室、底盘完全相同".into());
                }
            }
            let src = result.unit(donor_id)?;
            if src.fields.iter().any(|f| {
                f.value.contains("_nameless.")
                    || ((f.key == "slot_name" || f.key == "slot_hookup") && f.value != "0")
            }) {
                return Err("该附件携带子挂件或关联对象，首版暂不支持追加此模板".into());
            }
            let seed = crate::hash(
                format!(
                    "{}:{}:{}",
                    crate::hash(doc.text.as_bytes()),
                    op.truck_id,
                    donor_id
                )
                .as_bytes(),
            );
            let mut new_id = format!("_nameless.{}", &seed[..12]);
            let mut collision = 0;
            while result.index.contains_key(&new_id) {
                collision += 1;
                new_id = format!("_nameless.{}.{collision:x}", &seed[..12]);
            }
            let raw = &result.text[src.span.clone()];
            let cloned = raw.replacen(&format!(": {donor_id}"), &format!(": {new_id}"), 1);
            if cloned == raw {
                return Err("无法复制配件头".into());
            }
            let target = result.unit(&op.truck_id)?;
            let count = target.array("accessories")?.len();
            let count_field = target.field("accessories")?;
            let newline = if result.text.contains("\r\n") {
                "\r\n"
            } else {
                "\n"
            };
            let pos = if count > 0 {
                target
                    .field(&format!("accessories[{}]", count - 1))?
                    .span
                    .end
            } else {
                count_field.span.end
            };
            result = result.patch(vec![
                (count_field.span.clone(), (count + 1).to_string()),
                (
                    pos..pos,
                    format!("{newline} accessories[{count}]: {new_id}"),
                ),
                (
                    result.closing..result.closing,
                    format!("{cloned}{newline}{newline}"),
                ),
            ])?;
            accessory_owner.insert(new_id, indices[0]);
            warnings.push("外观件的供体与安装结构已校验；实际外观与碰撞仍需进游戏检查".into());
        }
        changes.push(Change {
            truck_id: op.truck_id.clone(),
            model: truck.model.clone(),
            plate: truck.plate.clone(),
            category: cat.clone(),
            action: op.action.clone(),
            before,
            after: op.candidate_path.clone(),
            position: location,
        });
        // Only this vehicle changes; later operations must see its updated definitions.
        let mut updated =
            crate::garage::truck(&result, catalog, &truck.id, truck.current, &truck.location)?;
        updated.driver = truck.driver.clone();
        for &index in indices {
            trucks[index] = updated.clone();
        }
    }
    result.validate_vehicles()?;
    if !operations.is_empty() {
        warnings.push("维修站的改装升级可能恢复原厂配件；实测普通维修及维修界面的部件更换可保留改装。升级后请用复查功能确认。".into());
    }
    Ok((
        result,
        Preview {
            changes,
            warnings,
            trucks,
        },
    ))
}

#[derive(Serialize)]
pub struct Verification {
    pub category: String,
    pub expected: String,
    pub status: String,
    pub matches: Vec<String>,
}
pub fn verify(doc: &Document, catalog: &Catalog, changes: &[Change]) -> Result<Vec<Verification>> {
    let trucks = inventory(doc, catalog)?;
    let mut out = Vec::new();
    for c in changes {
        let candidates: Vec<_> = trucks
            .iter()
            .filter(|t| t.model == c.model && t.plate == c.plate)
            .collect();
        if candidates.len() != 1 {
            out.push(Verification {
                category: c.category.clone(),
                expected: c.after.clone(),
                status: "车辆无法唯一匹配：车型/车牌已变化，请在车库手动核对".into(),
                matches: vec![],
            });
            continue;
        }
        let matching: Vec<_> = candidates[0]
            .accessories
            .iter()
            .filter(|a| a.category == c.category)
            .filter(|a| c.position.is_empty() || position(doc, &a.id) == c.position)
            .collect();
        let paths: Vec<_> = matching.iter().map(|a| a.path.clone()).collect();
        out.push(Verification {
            category: c.category.clone(),
            expected: c.after.clone(),
            status: if matching.len() > 1 {
                "安装位置无法唯一匹配，请手动核对"
            } else if paths.contains(&c.after) {
                "已保留"
            } else {
                "未保留 / 已被替换"
            }
            .into(),
            matches: paths,
        });
    }
    Ok(out)
}
