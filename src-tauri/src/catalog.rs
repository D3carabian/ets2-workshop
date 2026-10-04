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
    sync::OnceLock,
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
pub(crate) type MemorySources = HashMap<PathBuf, BTreeMap<String, Vec<u8>>>;

fn include_regex() -> &'static regex::Regex {
    static INCLUDE: OnceLock<regex::Regex> = OnceLock::new();
    INCLUDE.get_or_init(|| regex::Regex::new(r#"(?m)^\s*@include\s+"([^"]+)"[^\r\n]*"#).unwrap())
}

fn definition_include_path(path: &str, relative: &str) -> Result<String> {
    let joined = if relative.starts_with('/') {
        relative.trim_start_matches('/').to_owned()
    } else {
        format!(
            "{}/{}",
            path.rsplit_once('/')
                .map(|(parent, _)| parent)
                .unwrap_or(""),
            relative
        )
    };
    let mut components = Vec::new();
    for part in joined.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                components.pop().ok_or("include 超出资源目录")?;
            }
            _ if part.contains(['\\', ':', '\0']) => return Err("无效定义引用".into()),
            _ => components.push(part),
        }
    }
    Ok(components.join("/"))
}

fn expand_memory(path: &str, files: &BTreeMap<String, Vec<u8>>, depth: usize) -> Result<String> {
    if depth > 16 {
        return Err("定义 include 过深".into());
    }
    let text = std::str::from_utf8(
        files
            .get(path)
            .ok_or_else(|| format!("缺少定义引用 {path}"))?,
    )
    .map_err(|error| error.to_string())?;
    let mut out = String::new();
    let mut end = 0;
    for cap in include_regex().captures_iter(text) {
        let matched = cap.get(0).unwrap();
        out.push_str(&text[end..matched.start()]);
        let child = definition_include_path(path, &cap[1])?;
        out.push_str(&expand_memory(&child, files, depth + 1)?);
        end = matched.end();
    }
    out.push_str(&text[end..]);
    Ok(out)
}
fn expand(path: &Path, root: &Path, depth: usize) -> Result<String> {
    if depth > 16 {
        return Err("定义 include 过深".into());
    }
    let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
    let re = include_regex();
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

fn archive_source(pack: &Path) -> Result<BTreeMap<String, Vec<u8>>> {
    let archive = crate::game_archive::Archive::open(pack)?;
    let mut files: BTreeMap<_, _> = archive.catalog_files()?.into_iter().collect();
    // Definitions can include files outside def/. Fetch those exact references
    // too, preserving the official extractor's complete-archive semantics.
    let mut pending: Vec<_> = files
        .keys()
        .filter(|path| path.ends_with(".sii") || path.ends_with(".sui"))
        .cloned()
        .collect();
    let mut attempted = std::collections::HashSet::new();
    let mut bytes: usize = files.values().map(Vec::len).sum();
    while let Some(path) = pending.pop() {
        let Ok(text) = std::str::from_utf8(&files[&path]) else {
            continue;
        };
        let includes = include_regex()
            .captures_iter(text)
            .map(|cap| definition_include_path(&path, &cap[1]))
            .collect::<Result<Vec<_>>>()?;
        for child in includes {
            if files.contains_key(&child) || !attempted.insert(child.clone()) {
                continue;
            }
            if let Some(data) = archive.read(&child)? {
                bytes = bytes
                    .checked_add(data.len())
                    .ok_or("Catalog size overflow")?;
                if bytes > 512 * 1024 * 1024 {
                    return Err("Catalog extraction exceeds size limit".into());
                }
                files.insert(child.clone(), data);
                pending.push(child);
            }
        }
    }
    Ok(files)
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
        Self::scan_sources(roots, languages, &HashMap::new())
    }
    fn scan_sources(
        roots: &[(PathBuf, String)],
        languages: &crate::localization::Languages,
        memory: &MemorySources,
    ) -> Result<Self> {
        let mut out = Self::default();
        let mut sig = String::new();
        let mut appearances = HashMap::<String, Vec<Option<String>>>::new();
        for (root, label) in roots {
            let inputs: Vec<(String, Result<String>)> = if let Some(files) = memory.get(root) {
                files
                    .keys()
                    .filter(|path| path.starts_with("def/vehicle/") && path.ends_with(".sii"))
                    .map(|path| (format!("/{path}"), expand_memory(path, files, 0)))
                    .collect()
            } else {
                let def = root.join("def/vehicle");
                if !def.exists() {
                    continue;
                }
                walkdir::WalkDir::new(&def)
                    .follow_links(false)
                    .into_iter()
                    .filter_map(|entry| entry.ok())
                    .filter(|entry| {
                        entry.file_type().is_file()
                            && entry.path().extension().and_then(|name| name.to_str())
                                == Some("sii")
                    })
                    .map(|entry| {
                        (
                            format!(
                                "/{}",
                                entry
                                    .path()
                                    .strip_prefix(root)
                                    .unwrap()
                                    .to_string_lossy()
                                    .replace('\\', "/")
                            ),
                            expand(entry.path(), root, 0),
                        )
                    })
                    .collect()
            };
            for (path, text) in inputs {
                let text = match text {
                    Ok(text) => text,
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
                    Path::new(&path)
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
fn game_packs(game: &Path) -> Result<Vec<PathBuf>> {
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
    Ok(packs)
}

// Separate from the public catalog schema: parser/selection changes explicitly
// invalidate this rebuild cache without making existing catalogs unreadable.
const BUILD_CACHE_VERSION: u32 = 1;
#[derive(Serialize, Deserialize)]
struct BuildCache {
    version: u32,
    fingerprint: String,
    catalog: Catalog,
}

fn build_fingerprint(game: &Path, packs: &[PathBuf]) -> Result<String> {
    let mut inputs = Vec::new();
    for path in packs
        .iter()
        .chain(std::iter::once(&game.join("locale.scs")))
    {
        let meta = match std::fs::metadata(path) {
            Ok(meta) => meta,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && path.file_name().is_some_and(|name| name == "locale.scs") =>
            {
                inputs.push(format!("{}:missing", path.display()));
                continue;
            }
            Err(error) => return Err(error.to_string()),
        };
        let modified = meta.modified().map_err(|error| error.to_string())?;
        inputs.push(format!(
            "{}:{}:{modified:?}",
            path.canonicalize()
                .map_err(|error| error.to_string())?
                .display(),
            meta.len()
        ));
    }
    Ok(hash(inputs.join("\n").as_bytes()))
}

pub fn build(game: &Path, extractor: &Path, cache: &Path) -> Result<Catalog> {
    build_with_progress(game, extractor, cache, &|_| {})
}

pub fn build_with_progress(
    game: &Path,
    extractor: &Path,
    cache: &Path,
    progress: &dyn Fn(&str),
) -> Result<Catalog> {
    if !extractor.is_file() {
        return Err("请选择 scs_extractor.exe".into());
    }
    let packs = game_packs(game)?;
    let fingerprint = build_fingerprint(game, &packs)?;
    let cache_file = cache.join("parts-build-cache-v1.json");
    if let Some(cached) = std::fs::read(&cache_file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<BuildCache>(&bytes).ok())
    {
        if cached.version == BUILD_CACHE_VERSION
            && cached.fingerprint == fingerprint
            && !cached.catalog.definitions.is_empty()
        {
            progress("游戏资源未变化，复用已建立的配件目录");
            return Ok(cached.catalog);
        }
    }
    let mut roots = Vec::new();
    let mut memory = MemorySources::new();
    let mut archives = Vec::new();
    for (index, pack) in packs.iter().enumerate() {
        progress(&format!(
            "准备游戏定义 {}/{}：{}",
            index + 1,
            packs.len(),
            pack.file_name().unwrap().to_string_lossy()
        ));
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
            match archive_source(pack) {
                Ok(files) => {
                    memory.insert(root.clone(), files);
                    if pack.file_name().is_some_and(|name| name == "def.scs") {
                        Catalog::scan_sources(
                            &[(root.clone(), "def.scs".into())],
                            &crate::localization::Languages::new(),
                            &memory,
                        )?;
                    }
                    roots.push((root, pack.file_name().unwrap().to_string_lossy().into()));
                    continue;
                }
                Err(error) => progress(&format!(
                    "{} 使用官方解包器（{error}）",
                    pack.file_name().unwrap().to_string_lossy()
                )),
            }
            // The official extractor interprets non-ASCII command-line paths incorrectly.
            // Use ASCII relative arguments inside an isolated staging directory instead.
            let stage = root.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
            std::fs::create_dir_all(&stage).map_err(|e| e.to_string())?;
            let extraction = (|| -> Result<()> {
                let input = stage.join("input.scs");
                if std::fs::hard_link(pack, &input).is_err() {
                    std::fs::copy(pack, &input).map_err(|e| e.to_string())?;
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
    progress("读取游戏中英文文本与配件属性");
    let (languages, name_warnings) = crate::localization::load_sources(game, &roots, &memory);
    let mut catalog = Catalog::scan_sources(&roots, &languages, &memory)?;
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
    if fingerprint != build_fingerprint(game, &game_packs(game)?)? {
        return Err("建立目录期间游戏资源发生变化，请等待游戏更新完成后重试".into());
    }
    std::fs::create_dir_all(cache).map_err(|error| error.to_string())?;
    crate::setup::write_atomic(
        &cache_file,
        &serde_json::to_vec(&BuildCache {
            version: BUILD_CACHE_VERSION,
            fingerprint,
            catalog: catalog.clone(),
        })
        .map_err(|error| error.to_string())?,
    )?;
    Ok(catalog)
}

#[cfg(test)]
mod name_tests {
    use super::*;
    use std::time::Instant;

    #[test]
    fn memory_sources_preserve_external_includes_and_localized_overrides() {
        use crate::game_archive::tests::{listing, tree_fixture};
        let tmp = tempfile::tempdir().unwrap();
        let root = tmp.path().join("root");
        let files = BTreeMap::<String, Vec<u8>>::from([
            ("def/vehicle/engine.sii".into(), b"SiiNunit {\naccessory_engine_data : .test {\n@include \"/shared/engine.inc\"\n}\n}\n".to_vec()),
            ("shared/engine.inc".into(), b"name: \"@@test@@\"\n@include \"../parameters/torque.inc\"\n".to_vec()),
            ("parameters/torque.inc".into(), b"torque: 2500\n".to_vec()),
            ("locale/en_gb/local.test.sii".into(), b"SiiNunit {\nlocalization_db : .l {\nkey[]: \"test\"\nval[]: \"DLC engine\"\n}\n}\n".to_vec()),
        ]);
        for (path, bytes) in &files {
            let target = root.join(path);
            std::fs::create_dir_all(target.parent().unwrap()).unwrap();
            std::fs::write(target, bytes).unwrap();
        }
        let root_listing = listing(&["/def", "/locale"]);
        let def_listing = listing(&["/vehicle"]);
        let vehicle_listing = listing(&["engine.sii"]);
        let locale_listing = listing(&["/en_gb"]);
        let language_listing = listing(&["local.test.sii"]);
        let mut entries = vec![
            ("", 0x81, root_listing.as_slice(), 0),
            ("def", 0x81, def_listing.as_slice(), 0),
            ("def/vehicle", 0x81, vehicle_listing.as_slice(), 0),
            ("locale", 0x81, locale_listing.as_slice(), 0),
            ("locale/en_gb", 0x81, language_listing.as_slice(), 0),
        ];
        entries.extend(
            files
                .iter()
                .map(|(path, data)| (path.as_str(), 0x80, data.as_slice(), 0x10)),
        );
        let pack = tmp.path().join("def.scs");
        std::fs::write(&pack, tree_fixture(&entries)).unwrap();
        let extracted = archive_source(&pack).unwrap();
        assert_eq!(
            extracted, files,
            "Includes outside listed definition/locale roots must still be read"
        );
        let roots = [(root.clone(), "fixture".into())];
        let memory = HashMap::from([(root, extracted)]);
        let (disk_languages, _) = crate::localization::load(tmp.path(), &roots);
        let (memory_languages, _) = crate::localization::load_sources(tmp.path(), &roots, &memory);
        assert_eq!(disk_languages, memory_languages);
        let disk = Catalog::scan_localized(&roots, &disk_languages).unwrap();
        let cached = Catalog::scan_sources(&roots, &memory_languages, &memory).unwrap();
        assert_eq!(
            serde_json::to_value(disk).unwrap(),
            serde_json::to_value(cached).unwrap()
        );
        assert!(definition_include_path("def/engine.sii", "../../escape.sui").is_err());
    }

    #[test]
    fn selective_build_cache_handles_hot_changed_added_removed_and_corrupt_inputs() {
        use crate::game_archive::tests::{listing, tree_fixture};
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let cache = tmp.path().join("cache");
        std::fs::create_dir_all(&game).unwrap();
        let extractor = tmp.path().join("unused-extractor.exe");
        std::fs::write(&extractor, b"not executed on the supported fast path").unwrap();
        let pack = |name: &str| {
            let root = listing(&["/def"]);
            let def = listing(&["/vehicle"]);
            let vehicle = listing(&["engine.sii"]);
            let text = format!(
                "SiiNunit {{\naccessory_engine_data : .test {{\nname: \"{name}\"\n}}\n}}\n"
            );
            tree_fixture(&[
                ("", 0x81, &root, 0x10),
                ("def", 0x81, &def, 0),
                ("def/vehicle", 0x81, &vehicle, 0),
                ("def/vehicle/engine.sii", 0x80, text.as_bytes(), 0x10),
            ])
        };
        std::fs::write(game.join("def.scs"), pack("first")).unwrap();
        let cold = build(&game, &extractor, &cache).unwrap();
        let key = "/def/vehicle/engine.sii";
        assert_eq!(cold.definitions[key].name, "first");
        let hot_messages = std::cell::RefCell::new(Vec::new());
        let hot = build_with_progress(&game, &extractor, &cache, &|message| {
            hot_messages.borrow_mut().push(message.to_owned())
        })
        .unwrap();
        assert_eq!(hot.signature, cold.signature);
        assert_eq!(hot_messages.borrow().len(), 1);
        assert!(hot_messages.borrow()[0].contains("复用"));
        std::fs::write(game.join("def.scs"), pack("changed name")).unwrap();
        assert_eq!(
            build(&game, &extractor, &cache).unwrap().definitions[key].name,
            "changed name"
        );
        std::fs::write(game.join("dlc_daf.scs"), pack("DLC override")).unwrap();
        assert_eq!(
            build(&game, &extractor, &cache).unwrap().definitions[key].name,
            "DLC override"
        );
        std::fs::remove_file(game.join("dlc_daf.scs")).unwrap();
        std::fs::write(cache.join("parts-build-cache-v1.json"), b"broken cache").unwrap();
        assert_eq!(
            build(&game, &extractor, &cache).unwrap().definitions[key].name,
            "changed name"
        );
        let before = build_fingerprint(&game, &game_packs(&game).unwrap()).unwrap();
        std::fs::write(game.join("locale.scs"), b"new language archive").unwrap();
        assert_ne!(
            before,
            build_fingerprint(&game, &game_packs(&game).unwrap()).unwrap()
        );
        let file = std::fs::OpenOptions::new()
            .write(true)
            .open(game.join("def.scs"))
            .unwrap();
        let time = std::time::UNIX_EPOCH + std::time::Duration::new(1_000_000, 100);
        file.set_times(std::fs::FileTimes::new().set_modified(time))
            .unwrap();
        let first = build_fingerprint(&game, &game_packs(&game).unwrap()).unwrap();
        file.set_times(
            std::fs::FileTimes::new().set_modified(time + std::time::Duration::from_nanos(500)),
        )
        .unwrap();
        assert_ne!(
            first,
            build_fingerprint(&game, &game_packs(&game).unwrap()).unwrap(),
            "Same-second changes must invalidate the build cache"
        );
    }

    #[test]
    #[ignore = "Requires installed game, extractor, and baseline JSON; isolated output only"]
    fn installed_catalog_performance_and_equivalence() {
        let game = PathBuf::from(std::env::var("ETS2_GAME_DIR").unwrap());
        let extractor = PathBuf::from(std::env::var("ETS2_EXTRACTOR").unwrap());
        let baseline = PathBuf::from(std::env::var("ETS2_BASELINE_CATALOG").unwrap());
        let expected: Catalog = serde_json::from_slice(&std::fs::read(baseline).unwrap()).unwrap();
        let output = Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("verification/catalog-perf");
        std::fs::create_dir_all(&output).unwrap();
        let cache = tempfile::Builder::new()
            .prefix("selective-")
            .tempdir_in(&output)
            .unwrap();
        let started = Instant::now();
        let cold = build_with_progress(&game, &extractor, cache.path(), &|message| {
            eprintln!("{message}")
        })
        .unwrap();
        let cold_seconds = started.elapsed().as_secs_f64();
        let started = Instant::now();
        let hot = build(&game, &extractor, cache.path()).unwrap();
        let hot_seconds = started.elapsed().as_secs_f64();
        assert_eq!(serde_json::to_value(&cold.definitions).unwrap(), serde_json::to_value(&expected.definitions).unwrap(), "Selective extraction must preserve every definition, translated name, metric and source");
        assert_eq!(
            serde_json::to_value(&cold).unwrap(),
            serde_json::to_value(&hot).unwrap()
        );
        let mut files = 0u64;
        let mut bytes = 0u64;
        for entry in walkdir::WalkDir::new(cache.path().join("extracted"))
            .into_iter()
            .flatten()
            .filter(|entry| entry.file_type().is_file())
        {
            files += 1;
            bytes += entry.metadata().unwrap().len();
        }
        let result = serde_json::json!({"cold_seconds":cold_seconds,"hot_seconds":hot_seconds,"definitions":cold.definitions.len(),"extracted_files":files,"extracted_bytes":bytes,"all_definitions_equal":true});
        eprintln!("{result}");
        std::fs::write(
            output.join("result.json"),
            serde_json::to_vec_pretty(&result).unwrap(),
        )
        .unwrap();
    }
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
