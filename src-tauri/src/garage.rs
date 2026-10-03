use crate::{
    catalog::{category, model, Catalog},
    sii::{unquote, Document},
    Result,
};
use serde::Serialize;
#[derive(Clone, Debug, Serialize)]
pub struct Accessory {
    pub id: String,
    pub index: usize,
    pub kind: String,
    pub path: String,
    pub category: String,
    pub name: String,
    pub model: String,
    pub refund: String,
    pub raw: String,
    pub definition: Option<crate::catalog::Definition>,
}
#[derive(Clone, Debug, Serialize)]
pub struct Truck {
    pub id: String,
    pub plate: String,
    pub model: String,
    pub current: bool,
    pub accessories: Vec<Accessory>,
    pub location: String,
}
pub fn inventory(doc: &Document, catalog: &Catalog) -> Result<Vec<Truck>> {
    let player = doc
        .units
        .iter()
        .find(|u| u.kind == "player")
        .ok_or("找不到玩家车辆列表")?;
    let owned = player.array("trucks")?;
    let mut current = player.get("assigned_truck").unwrap_or("").to_string();
    if let Some(id) = player.get("assigned_vehicles") {
        if let Ok(u) = doc.unit(id) {
            for f in &u.fields {
                if owned.contains(&f.value) {
                    current = f.value.clone();
                }
                if let Ok(v) = doc.unit(&f.value) {
                    for vf in &v.fields {
                        if owned.contains(&vf.value) {
                            current = vf.value.clone();
                        }
                    }
                }
            }
        }
    }
    let tag = regex::Regex::new(r"<[^>]*>").unwrap();
    let mut trucks = Vec::new();
    for id in owned {
        let u = doc.unit(&id)?;
        if u.kind != "vehicle" {
            return Err("玩家卡车引用类型不正确".into());
        }
        let mut accessories = Vec::new();
        for (n, r) in u.array("accessories")?.iter().enumerate() {
            let a = doc.unit(r)?;
            let path = unquote(a.get("data_path").unwrap_or(""));
            let def = catalog.definitions.get(&path).cloned();
            accessories.push(Accessory {
                id: r.clone(),
                index: n,
                kind: a.kind.clone(),
                category: category(&path),
                name: def.as_ref().map(|d| d.name.clone()).unwrap_or_else(|| {
                    path.rsplit('/')
                        .next()
                        .unwrap_or("未知配件")
                        .trim_end_matches(".sii")
                        .replace('_', " ")
                }),
                model: model(&path),
                refund: a.get("refund").unwrap_or("—").into(),
                raw: doc.text[a.span.clone()].into(),
                path,
                definition: def,
            });
        }
        let truck_model = accessories
            .iter()
            .find(|a| a.category == "vehicle")
            .or_else(|| accessories.iter().find(|a| a.category == "cabin"))
            .map(|a| a.model.clone())
            .unwrap_or_else(|| "未知车型".into());
        let plate = tag
            .replace_all(&unquote(u.get("license_plate").unwrap_or("")), "")
            .split('|')
            .next()
            .unwrap_or("")
            .trim()
            .to_string();
        let mut locations = Vec::new();
        for g in &doc.units {
            if g.kind == "garage" && g.fields.iter().any(|f| f.value == id) {
                locations.push(g.id.clone());
            }
        }
        trucks.push(Truck {
            id: id.clone(),
            plate,
            model: truck_model,
            current: id == current,
            accessories,
            location: locations.join(", "),
        });
    }
    Ok(trucks)
}
