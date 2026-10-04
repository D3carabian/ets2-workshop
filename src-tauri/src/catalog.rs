use crate::{
    hash,
    sii::{unquote, Document},
    Result,
};
use serde::{Deserialize, Serialize};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::{
    collections::{BTreeMap, HashMap},
    path::{Path, PathBuf},
    process::Command,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Definition {
    pub path: String,
    pub kind: String,
    pub unit: String,
    pub name: String,
    pub category: String,
    pub model: String,
    pub source: String,
    pub metrics: BTreeMap<String, String>,
    pub suitable: Vec<String>,
    pub conflicts: Vec<String>,
    pub requires: Vec<String>,
}
#[derive(Clone, Default, Serialize, Deserialize)]
pub struct Catalog {
    pub definitions: HashMap<String, Definition>,
    pub warnings: Vec<String>,
    pub signature: String,
    #[serde(default)]
    pub archives: Vec<(String, u64, u64)>,
}
fn display_tokens(s: &str) -> String {
    let mut result = s.to_string();
    for (a, b) in [
        ("@@hp@@", "hp"),
        ("@@kw@@", "kW"),
        ("@@nm@@", "Nm"),
        ("@@rpm@@", "rpm"),
        ("@@dg@@", ","),
        ("@@liters@@", "L"),
    ] {
        result = result.replace(a, b);
    }
    result
}
pub fn category(path: &str) -> String {
    for cat in [
        "engine",
        "transmission",
        "chassis",
        "cabin",
        "interior",
        "tank",
        "paint_job",
        "f_tire",
        "r_tire",
        "f_disc",
        "r_disc",
        "f_hub",
        "r_hub",
        "f_nuts",
        "r_nuts",
    ] {
        if path.contains(&format!("/{cat}/")) {
            return cat.into();
        }
    }
    if let Some(s) = path.split("/accessory/").nth(1) {
        return s.split('/').next().unwrap_or("unknown").into();
    }
    if path.ends_with("/data.sii") {
        return "vehicle".into();
    }
    if let Some(s) = path.split("/truck/").nth(1) {
        if let Some(c) = s.split('/').nth(1) {
            return c.into();
        }
    }
    "unknown".into()
}
pub fn model(path: &str) -> String {
    path.split("/truck/")
        .nth(1)
        .and_then(|p| p.split('/').next())
        .unwrap_or("通用")
        .into()
}
fn values(u: &crate::sii::Unit, key: &str) -> Vec<String> {
    u.fields
        .iter()
        .filter(|f| f.key == format!("{key}[]") || f.key.starts_with(&format!("{key}[")))
        .map(|f| unquote(&f.value))
        .collect()
}
fn expand(path: &Path, root: &Path, depth: usize) -> Result<String> {
    if depth > 16 {
        return Err("定义 include 过深".into());
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let re = regex::Regex::new(r#"(?m)^\s*@include\s+"([^"]+)"[^\r\n]*"#).unwrap();
    let mut out = String::new();
    let mut end = 0;
    for cap in re.captures_iter(&text) {
        let m = cap.get(0).unwrap();
        out.push_str(&text[end..m.start()]);
        let p = if cap[1].starts_with('/') {
            root.join(cap[1].trim_start_matches('/'))
        } else {
            path.parent().unwrap().join(&cap[1])
        };
        let resolved = p.canonicalize().map_err(|e| e.to_string())?;
        let base = root.canonicalize().map_err(|e| e.to_string())?;
        if !resolved.starts_with(&base) {
            return Err("include 超出资源目录".into());
        }
        out.push_str(&expand(&resolved, root, depth + 1)?);
        end = m.end();
    }
    out.push_str(&text[end..]);
    Ok(out)
}
impl Catalog {
    pub fn fresh(&self) -> Result<()> {
        for (path, len, modified) in &self.archives {
            let m = std::fs::metadata(path).map_err(|_| "游戏资源已移动，请重新建立目录")?;
            let time = m
                .modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|t| t.as_secs())
                .unwrap_or(0);
            if m.len() != *len || time != *modified {
                return Err("游戏资源已更新，请重新建立配件目录后再修改".into());
            }
        }
        Ok(())
    }
    pub fn scan(roots: &[(PathBuf, String)]) -> Result<Self> {
        let mut out = Self::default();
        let mut sig = String::new();
        for (root, label) in roots {
            let def = root.join("def/vehicle");
            if !def.exists() {
                continue;
            }
            for e in walkdir::WalkDir::new(&def)
                .follow_links(false)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if !e.file_type().is_file()
                    || e.path().extension().and_then(|s| s.to_str()) != Some("sii")
                {
                    continue;
                }
                let path = format!(
                    "/{}",
                    e.path()
                        .strip_prefix(root)
                        .unwrap()
                        .to_string_lossy()
                        .replace('\\', "/")
                );
                let text = match expand(e.path(), root, 0) {
                    Ok(t) => t,
                    Err(_) => continue,
                };
                let doc = match Document::parse(text) {
                    Ok(d) => d,
                    Err(_) => continue,
                };
                let Some(u) = doc.units.iter().find(|u| u.kind.starts_with("accessory_")) else {
                    continue;
                };
                let mut metrics = BTreeMap::new();
                for k in [
                    "type",
                    "price",
                    "torque",
                    "secondary_torque",
                    "rpm_limit",
                    "rpm_limit_neutral",
                    "volume",
                    "consumption_coef",
                    "differential_ratio",
                    "retarder",
                    "tank_size",
                    "fuel_tank_size",
                    "adblue_tank_size",
                    "roll_resistance",
                    "wet_grip",
                    "noise_volume",
                    "radius",
                ] {
                    if let Some(v) = u.get(k) {
                        metrics.insert(k.into(), v.into());
                    }
                }
                for k in ["info", "torque_curve", "ratios_forward", "ratios_reverse"] {
                    let v = values(u, k);
                    if !v.is_empty() {
                        metrics.insert(k.into(), display_tokens(&v.join(" · ")));
                    }
                }
                if u.kind == "accessory_engine_data" && !metrics.contains_key("rpm_limit") {
                    metrics.insert("rpm_limit".into(), "2500（引擎默认值）".into());
                }
                let raw_name = unquote(u.get("name").unwrap_or(""));
                let name = if raw_name.is_empty() || raw_name.contains("@@") {
                    e.path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .replace('_', " ")
                } else {
                    raw_name
                };
                let item = Definition {
                    path: path.clone(),
                    kind: u.kind.clone(),
                    unit: u.id.clone(),
                    name,
                    category: category(&path),
                    model: model(&path),
                    source: label.clone(),
                    metrics,
                    suitable: values(u, "suitable_for"),
                    conflicts: values(u, "conflict_with"),
                    requires: values(u, "require"),
                };
                if out.definitions.contains_key(&path) {
                    out.warnings
                        .push(format!("定义重复，以后扫描的包为准：{path}"));
                }
                sig.push_str(&path);
                sig.push_str(&hash(doc.text.as_bytes()));
                out.definitions.insert(path, item);
            }
        }
        out.signature = hash(sig.as_bytes());
        if out.definitions.is_empty() {
            return Err("未发现配件定义，请检查资源目录".into());
        }
        Ok(out)
    }
}
pub fn build(game: &Path, extractor: &Path, cache: &Path) -> Result<Catalog> {
    if !extractor.is_file() {
        return Err("请选择 scs_extractor.exe".into());
    }
    let base = game.join("def.scs");
    if !base.is_file() {
        return Err("游戏目录缺少 def.scs".into());
    }
    let mut packs = vec![base];
    let tags = [
        "daf", "volvo", "scania", "man_tgx", "iveco", "renault", "actros", "michelin", "goodyear",
        "rims", "toys", "schoch", "raven", "rocket", "holland", "dutch", "griffin",
    ];
    let mut dlcs: Vec<_> = std::fs::read_dir(game)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            let n = p.file_name().unwrap().to_string_lossy();
            n.starts_with("dlc_") && n.ends_with(".scs") && tags.iter().any(|t| n.contains(t))
        })
        .collect();
    dlcs.sort();
    packs.extend(dlcs);
    let mut roots = Vec::new();
    let mut archives = Vec::new();
    for pack in packs {
        let meta = std::fs::metadata(&pack).map_err(|e| e.to_string())?;
        archives.push((
            pack.to_string_lossy().into(),
            meta.len(),
            meta.modified()
                .ok()
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|t| t.as_secs())
                .unwrap_or(0),
        ));
        let canonical = pack.canonicalize().map_err(|e| e.to_string())?;
        let canonical_text = canonical.to_string_lossy();
        let stable_path = canonical_text
            .strip_prefix("\\\\?\\")
            .unwrap_or(&canonical_text);
        let key = hash(format!("{}:{}:{:?}", stable_path, meta.len(), meta.modified()).as_bytes());
        let root = cache.join("extracted").join(&key[..16]);
        if !root.join(".complete").exists() {
            // The official extractor interprets non-ASCII command-line paths incorrectly.
            // Use ASCII relative arguments inside an isolated staging directory instead.
            let stage = root.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
            std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
            let extraction = (|| -> Result<()> {
                let input = stage.join("input.scs");
                if std::fs::hard_link(&pack, &input).is_err() {
                    std::fs::copy(&pack, &input).map_err(|e| e.to_string())?;
                }
                let mut cmd = Command::new(extractor.canonicalize().map_err(|e| e.to_string())?);
                cmd.current_dir(&stage).arg("input.scs").arg("output");
                #[cfg(windows)]
                cmd.creation_flags(0x08000000);
                let output = cmd.output().map_err(|e| e.to_string())?;
                if !output.status.success() {
                    return Err(format!(
                        "解包失败 {}: {}",
                        pack.display(),
                        String::from_utf8_lossy(&output.stderr)
                    ));
                }
                let extracted = stage.join("output");
                // Do not cache an empty/unreadable base archive as a completed extraction.
                if pack.file_name().is_some_and(|n| n == "def.scs") {
                    Catalog::scan(&[(extracted.clone(), "def.scs".into())])?;
                }
                std::fs::write(extracted.join(".complete"), b"ok").map_err(|e| e.to_string())?;
                if root.exists() {
                    std::fs::remove_dir_all(&root).map_err(|e| e.to_string())?;
                }
                std::fs::rename(extracted, &root).map_err(|e| e.to_string())?;
                Ok(())
            })();
            let _ = std::fs::remove_dir_all(&stage);
            extraction?;
        }
        roots.push((root, pack.file_name().unwrap().to_string_lossy().into()));
    }
    let mut catalog = Catalog::scan(&roots)?;
    catalog.archives = archives;
    catalog.warnings.push(
        "目录包含本体与已识别的官方车型/配件 DLC；Mod 覆盖顺序未导入。未知配件仅供查看。".into(),
    );
    Ok(catalog)
}
