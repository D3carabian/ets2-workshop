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
    #[serde(default)]
    pub raw_name: String,
    #[serde(default)]
    pub names: BTreeMap<String, String>,
    #[serde(default)]
    pub category_names: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub name_alias: Option<String>,
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
    #[serde(default)]
    pub name_schema: u32,
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
        "f_cover",
        "r_cover",
        "f_rim",
        "r_rim",
        "f_wheel",
        "r_wheel",
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
        Self::scan_localized(roots, &crate::localization::Languages::new())
    }
    pub fn scan_localized(
        roots: &[(PathBuf, String)],
        languages: &crate::localization::Languages,
    ) -> Result<Self> {
        let mut out = Self::default();
        let mut sig = String::new();
        let mut appearances = HashMap::<String, Vec<Option<String>>>::new();
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
                let names = crate::localization::names(&raw_name, languages);
                let name = if let Some(english) = names.get("en") {
                    english.clone()
                } else if raw_name.is_empty() || raw_name.contains("@@") {
                    e.path()
                        .file_stem()
                        .unwrap()
                        .to_string_lossy()
                        .replace('_', " ")
                } else {
                    raw_name.clone()
                };
                let category = category(&path);
                let category_key = match category.as_str() {
                    "f_disc" | "r_disc" => "disc",
                    "f_hub" | "r_hub" => "hub",
                    "f_nuts" | "r_nuts" => "nuts",
                    "f_tire" | "r_tire" => "tire",
                    "f_cover" | "r_cover" => "cover",
                    "f_rim" | "r_rim" => "rim",
                    "head_light" => "head_lights",
                    "paint_job" => "paint_cat",
                    other => other,
                };
                let category_names = languages
                    .iter()
                    .filter_map(|(language, dictionary)| {
                        dictionary
                            .get(category_key)
                            .map(|value| (language.clone(), value.clone()))
                    })
                    .collect();
                if path.contains("/accessory/r_grill/dutch_lightbox_") {
                    appearances.insert(
                        path.clone(),
                        [
                            "exterior_model",
                            "interior_model",
                            "icon",
                            "look",
                            "variant",
                        ]
                        .iter()
                        .map(|key| u.get(key).map(unquote))
                        .collect(),
                    );
                }
                let item = Definition {
                    path: path.clone(),
                    kind: u.kind.clone(),
                    unit: u.id.clone(),
                    name,
                    raw_name,
                    names,
                    category_names,
                    name_alias: None,
                    category,
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
        // Four old DLC name keys have a proven current equivalent using the same
        // model, icon, look and variant. Revalidate those resources on every scan.
        for (model, file, original) in [
            (
                "volvo.fh16_2012",
                "dutch_lightbox_modern_roofbar_01",
                "Pure Dutch Steel Modern Roofbar",
            ),
            (
                "volvo.fh16_2012",
                "dutch_lightbox_modern_roofbar_02",
                "Dutch Holland On Wheels Modern Roofbar",
            ),
            ("volvo.fh16_2012", "dutch_lightbox_retro_02", "Dutch Power"),
            ("volvo.fh_2021", "dutch_lightbox_retro_02", "Dutch Power"),
        ] {
            let target = format!("/def/vehicle/truck/{model}/accessory/r_grill/{file}.sii");
            let reference =
                format!("/def/vehicle/truck/volvo.fh_2024/accessory/r_grill/{file}.sii");
            let same_resource = appearances.get(&target).is_some_and(|fields| {
                fields[0].is_some() && Some(fields) == appearances.get(&reference)
            });
            if !same_resource {
                continue;
            }
            let Some(reference_names) =
                out.definitions.get(&reference).map(|def| def.names.clone())
            else {
                continue;
            };
            if let Some(def) = out.definitions.get_mut(&target) {
                if def.raw_name != format!("@@{original}@@") {
                    continue;
                }
                let before = def.names.len();
                for (language, value) in reference_names {
                    def.names.entry(language).or_insert(value);
                }
                if def.names.len() > before {
                    def.name_alias = Some(reference);
                    if let Some(name) = def.names.get("en") {
                        def.name = name.clone();
                    }
                }
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
    let (languages, name_warnings) = crate::localization::load(game, &roots);
    let mut catalog = Catalog::scan_localized(&roots, &languages)?;
    catalog.name_schema = u32::from(
        languages.get("en").is_some_and(|words| !words.is_empty())
            && languages
                .get("zh_cn")
                .is_some_and(|words| !words.is_empty()),
    );
    catalog.warnings.extend(name_warnings);
    let locale_file = game.join("locale.scs");
    if let Ok(meta) = std::fs::metadata(&locale_file) {
        archives.push((
            locale_file.to_string_lossy().into(),
            meta.len(),
            meta.modified()
                .ok()
                .and_then(|time| time.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|time| time.as_secs())
                .unwrap_or(0),
        ));
    }
    catalog.archives = archives;
    catalog.warnings.push(
        "目录包含本体与已识别的官方车型/配件 DLC；Mod 覆盖顺序未导入。未知配件仅供查看。".into(),
    );
    Ok(catalog)
}

#[cfg(test)]
mod name_tests {
    use super::*;
    #[test]
    fn names_retain_raw_keys_and_do_not_fabricate_missing_translations() {
        let root = tempfile::tempdir().unwrap();
        let folder = root.path().join("def/vehicle/truck/example/engine");
        std::fs::create_dir_all(&folder).unwrap();
        std::fs::write(folder.join("one.sii"), "SiiNunit {\naccessory_engine_data : .one {\n name: \"@@test_engine@@\"\n info[]: \"428 @@hp@@\"\n}\n}\n").unwrap();
        let languages = BTreeMap::from([
            (
                "en".into(),
                BTreeMap::from([("test_engine".into(), "Example Engine".into())]),
            ),
            (
                "zh_cn".into(),
                BTreeMap::from([("test_engine".into(), "测试发动机".into())]),
            ),
        ]);
        let roots = [(root.path().to_path_buf(), "synthetic".into())];
        let catalog = Catalog::scan_localized(&roots, &languages).unwrap();
        let def = catalog.definitions.values().next().unwrap();
        assert_eq!(def.raw_name, "@@test_engine@@");
        assert_eq!(def.names["zh_cn"], "测试发动机");
        assert_eq!(def.names["en"], "Example Engine");
        assert_eq!(def.metrics["info"], "428 hp");
        let legacy = Catalog::scan(&roots).unwrap();
        assert!(legacy.definitions.values().next().unwrap().names.is_empty());
        let serialized = serde_json::to_value(def).unwrap();
        let mut legacy_value = serialized;
        legacy_value.as_object_mut().unwrap().remove("names");
        legacy_value.as_object_mut().unwrap().remove("raw_name");
        assert!(serde_json::from_value::<Definition>(legacy_value).is_ok());
    }

    #[test]
    fn old_dlc_name_alias_requires_identical_resources_and_preserves_fixed_names() {
        let root = tempfile::tempdir().unwrap();
        let write = |model: &str, name: &str, icon: &str| {
            let folder = root
                .path()
                .join(format!("def/vehicle/truck/{model}/accessory/r_grill"));
            std::fs::create_dir_all(&folder).unwrap();
            std::fs::write(folder.join("dutch_lightbox_retro_02.sii"), format!("SiiNunit {{\naccessory_addon_data : .fixture {{\n name: \"{name}\"\n exterior_model: \"/synthetic/shared.pmd\"\n icon: \"{icon}\"\n}}\n}}\n")).unwrap();
        };
        let languages = BTreeMap::from([(
            "en".into(),
            BTreeMap::from([("known".into(), "Verified name".into())]),
        )]);
        let roots = [(root.path().to_path_buf(), "synthetic".into())];
        let path =
            "/def/vehicle/truck/volvo.fh16_2012/accessory/r_grill/dutch_lightbox_retro_02.sii";
        write("volvo.fh16_2012", "@@Dutch Power@@", "same");
        write("volvo.fh_2024", "@@known@@", "same");
        let catalog = Catalog::scan_localized(&roots, &languages).unwrap();
        assert_eq!(catalog.definitions[path].names["en"], "Verified name");
        assert!(catalog.definitions[path].name_alias.is_some());
        write("volvo.fh16_2012", "@@Dutch Power@@", "different");
        assert!(Catalog::scan_localized(&roots, &languages)
            .unwrap()
            .definitions[path]
            .names
            .is_empty());
        write("volvo.fh16_2012", "New game name", "same");
        let updated = Catalog::scan_localized(&roots, &languages).unwrap();
        assert_eq!(updated.definitions[path].names["en"], "New game name");
        assert!(updated.definitions[path].name_alias.is_none());
        assert_eq!(
            category("/def/vehicle/f_cover/front_hub_cover_03.sii"),
            "f_cover"
        );
        assert_eq!(category("/def/vehicle/r_wheel/3.sii"), "r_wheel");
    }

    #[test]
    #[ignore = "Requires installed game and its existing extracted catalog; read-only assets"]
    fn installed_catalog_name_coverage() {
        let game = PathBuf::from(std::env::var("ETS2_GAME_DIR").unwrap());
        let cache = PathBuf::from(std::env::var("ETS2_EXISTING_CACHE").unwrap());
        let existing: Catalog =
            serde_json::from_slice(&std::fs::read(cache.join("catalog.json")).unwrap()).unwrap();
        let roots: Vec<_> = existing
            .archives
            .iter()
            .filter(|(path, _, _)| !path.ends_with("locale.scs"))
            .map(|(path, _, _)| {
                let pack = Path::new(path);
                let meta = std::fs::metadata(pack).unwrap();
                let canonical = pack.canonicalize().unwrap();
                let text = canonical.to_string_lossy();
                let stable = text.strip_prefix("\\\\?\\").unwrap_or(&text);
                let key =
                    hash(format!("{}:{}:{:?}", stable, meta.len(), meta.modified()).as_bytes());
                let root = cache.join("extracted").join(&key[..16]);
                assert!(
                    root.join(".complete").exists(),
                    "Current extracted game archive missing: {}",
                    pack.file_name().unwrap().to_string_lossy()
                );
                (
                    root,
                    pack.file_name().unwrap().to_string_lossy().into_owned(),
                )
            })
            .collect();
        let (languages, warnings) = crate::localization::load(&game, &roots);
        assert!(warnings.is_empty(), "{warnings:?}");
        let mut catalog = Catalog::scan_localized(&roots, &languages).unwrap();
        catalog.name_schema = 1;
        let mut categories = BTreeMap::<String, [usize; 3]>::new();
        let mut unresolved = Vec::new();
        for def in catalog.definitions.values().filter(|def| {
            def.path.contains("/vehicle/truck/")
                || def.path.contains("/vehicle/f_")
                || def.path.contains("/vehicle/r_")
        }) {
            let counts = categories.entry(def.category.clone()).or_default();
            counts[0] += 1;
            counts[1] += usize::from(def.names.contains_key("en"));
            counts[2] += usize::from(def.names.contains_key("zh_cn"));
            if !def.names.contains_key("zh_cn") {
                unresolved.push(serde_json::json!({"path":def.path,"raw_name":def.raw_name}));
            }
        }
        assert!(categories["engine"][2] > 150);
        let output = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("verification/p4-names");
        std::fs::create_dir_all(&output).unwrap();
        std::fs::write(
            output.join("catalog.json"),
            serde_json::to_vec(&catalog).unwrap(),
        )
        .unwrap();
        let audit = serde_json::json!({"definitions": catalog.definitions.len(), "categories_total_en_zh":categories,"unresolved":unresolved});
        std::fs::write(
            output.join("coverage.json"),
            serde_json::to_vec_pretty(&audit).unwrap(),
        )
        .unwrap();
        eprintln!(
            "{}",
            serde_json::to_string(&audit["categories_total_en_zh"]).unwrap()
        );
    }
}
