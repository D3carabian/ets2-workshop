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
    #[serde(default)]
    pub driver_names: HashMap<String, String>,
    #[serde(default)]
    pub driver_names_schema: u32,
    pub definitions: HashMap<String, Definition>,
    pub warnings: Vec<String>,
    pub signature: String,
    #[serde(default)]
    pub archives: Vec<(String, u64, u64)>,
    #[serde(default)]
    pub name_schema: u32,
    #[serde(default)]
    pub parser_version: u32,
    #[serde(default)]
    pub game_path: String,
    #[serde(default)]
    pub source_fingerprint: String,
    #[serde(default)]
    pub scan_complete: bool,
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
const SCALAR_METRICS: &[&str] = &[
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
];
const ARRAY_METRICS: &[&str] = &["info", "torque_curve", "ratios_forward", "ratios_reverse"];
const COMPATIBILITY_ARRAYS: &[&str] = &["suitable_for", "conflict_with", "require"];
const APPEARANCE_FIELDS: &[&str] = &[
    "exterior_model",
    "interior_model",
    "icon",
    "look",
    "variant",
];

fn values(u: &crate::sii::Unit, key: &str) -> Vec<String> {
    let anonymous = format!("{key}[]");
    let prefix = format!("{key}[");
    let mut indexed = std::collections::HashSet::new();
    u.fields
        .iter()
        .filter(|f| f.key.starts_with(&prefix))
        // Explicit repeated indices represent one value after conflict checking;
        // anonymous [] fields retain their original append semantics.
        .filter(|f| f.key == anonymous || indexed.insert(f.key.as_str()))
        .map(|f| unquote(&f.value))
        .collect()
}
/// Sources stay in archive override order for both disk and native readers.
#[derive(Clone)]
pub(crate) enum Source {
    Disk {
        root: PathBuf,
        label: String,
    },
    Memory {
        files: BTreeMap<String, Vec<u8>>,
        label: String,
        archive: Option<PathBuf>,
    },
}
impl Source {
    pub(crate) fn label(&self) -> &str {
        match self {
            Self::Disk { label, .. } | Self::Memory { label, .. } => label,
        }
    }
    pub(crate) fn read(&self, path: &str, limit: usize) -> Result<Option<Vec<u8>>> {
        match self {
            Self::Memory { files, archive, .. } => {
                let bytes = if let Some(bytes) = files.get(path) {
                    bytes.clone()
                } else if !path.starts_with("def/") && !path.starts_with("locale/") {
                    // A later package can be the first user of an earlier pack's
                    // external include. Selected trees are already complete, so
                    // only exact paths outside those trees need an archive read.
                    let Some(archive) = archive else {
                        return Ok(None);
                    };
                    let Some(bytes) = crate::game_archive::Archive::open(archive)
                        .and_then(|archive| archive.read(path))
                        .map_err(|e| format!("{}:{path}: {e}", archive.display()))?
                    else {
                        return Ok(None);
                    };
                    bytes
                } else {
                    return Ok(None);
                };
                if bytes.len() > limit {
                    return Err(format!("{path}: 文件过大"));
                }
                Ok(Some(bytes))
            }
            Self::Disk { root, .. } => {
                let file = root.join(path);
                let meta = match std::fs::metadata(&file) {
                    Ok(meta) => meta,
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(None),
                    Err(error) => return Err(format!("{}: {error}", file.display())),
                };
                let resolved = file
                    .canonicalize()
                    .map_err(|e| format!("{}: {e}", file.display()))?;
                let base = root
                    .canonicalize()
                    .map_err(|e| format!("{}: {e}", root.display()))?;
                if !resolved.starts_with(base) {
                    return Err(format!("{}: 文件路径越界", file.display()));
                }
                if meta.len() > limit as u64 {
                    return Err(format!("{}: 文件过大", file.display()));
                }
                use std::io::Read;
                let mut bytes = Vec::new();
                std::fs::File::open(&file)
                    .and_then(|f| f.take(limit as u64 + 1).read_to_end(&mut bytes))
                    .map_err(|e| format!("{}: {e}", file.display()))?;
                if bytes.len() > limit {
                    return Err(format!("{}: 文件过大", file.display()));
                }
                Ok(Some(bytes))
            }
        }
    }
    pub(crate) fn paths(
        &self,
        directory: &str,
        recursive: bool,
        warnings: &mut Vec<String>,
    ) -> Vec<String> {
        let mut paths = Vec::new();
        match self {
            Self::Memory { files, .. } => {
                let prefix = format!("{directory}/");
                paths.extend(
                    files
                        .keys()
                        .filter(|p| {
                            p.strip_prefix(&prefix)
                                .is_some_and(|tail| recursive || !tail.contains('/'))
                        })
                        .cloned(),
                );
            }
            Self::Disk { root, .. } => {
                let dir = root.join(directory);
                match std::fs::metadata(&dir) {
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => return paths,
                    Err(e) => {
                        warnings.push(format!("{}: {e}", dir.display()));
                        return paths;
                    }
                    Ok(meta) if !meta.is_dir() => {
                        warnings.push(format!("{}: 资源目录不是文件夹", dir.display()));
                        return paths;
                    }
                    Ok(_) => {}
                }
                let walker = walkdir::WalkDir::new(&dir)
                    .follow_links(false)
                    .max_depth(if recursive { usize::MAX } else { 1 });
                for entry in walker {
                    match entry {
                        Ok(entry) if entry.file_type().is_file() => paths.push(
                            entry
                                .path()
                                .strip_prefix(root)
                                .unwrap()
                                .to_string_lossy()
                                .replace('\\', "/"),
                        ),
                        Ok(_) => {}
                        Err(e) => {
                            warnings.push(format!("{}: {e}", e.path().unwrap_or(&dir).display()))
                        }
                    }
                }
            }
        }
        paths.sort();
        paths
    }
}
pub(crate) fn disk_sources(roots: &[(PathBuf, String)]) -> Vec<Source> {
    roots
        .iter()
        .map(|(root, label)| Source::Disk {
            root: root.clone(),
            label: label.clone(),
        })
        .collect()
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

fn expand_source(
    path: &str,
    sources: &[Source],
    depth: usize,
    budget: &mut usize,
) -> Result<String> {
    if depth > 16 {
        return Err(format!("{path}: 定义 include 过深"));
    }
    // Current package wins, then previously mounted packages in reverse order.
    // Keep this view for nested includes too, so a DLC can override a helper
    // referenced by a shared base include. Real read errors never fall through.
    let mut found = None;
    for source in sources.iter().rev() {
        if let Some(bytes) = source.read(path, 16 * 1024 * 1024)? {
            found = Some(bytes);
            break;
        }
    }
    let bytes = found.ok_or_else(|| format!("缺少定义引用 {path}"))?;
    let text = std::str::from_utf8(&bytes).map_err(|e| format!("{path}: {e}"))?;
    *budget = budget.checked_add(text.len()).ok_or("定义文本过大")?;
    if *budget > 16 * 1024 * 1024 {
        return Err(format!("{path}: 定义文本过大"));
    }
    let mut out = String::new();
    let mut end = 0;
    for (matched, relative) in crate::sii::include_directives(text)? {
        out.push_str(&text[end..matched.start]);
        let child = definition_include_path(path, &relative)?;
        out.push_str(&expand_source(&child, sources, depth + 1, budget)?);
        end = matched.end;
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
    let mut scheduled: std::collections::HashSet<_> = pending.iter().cloned().collect();
    let mut bytes: usize = files.values().map(Vec::len).sum();
    while let Some(path) = pending.pop() {
        let Ok(text) = std::str::from_utf8(&files[&path]) else {
            continue;
        };
        let includes = crate::sii::include_directives(text)
            .map_err(|e| format!("{path}: {e}"))?
            .into_iter()
            .map(|(_, relative)| definition_include_path(&path, &relative))
            .collect::<Result<Vec<_>>>()?;
        for child in includes {
            if !scheduled.insert(child.clone()) {
                continue;
            }
            if !files.contains_key(&child) {
                let Some(data) = archive.read(&child)? else {
                    continue;
                };
                bytes = bytes
                    .checked_add(data.len())
                    .ok_or("Catalog size overflow")?;
                if bytes > 512 * 1024 * 1024 {
                    return Err("Catalog extraction exceeds size limit".into());
                }
                files.insert(child.clone(), data);
            }
            // Already-loaded .inc (or other extension) files can themselves
            // reference resources outside the selected trees.
            pending.push(child);
        }
    }
    Ok(files)
}
impl Catalog {
    pub fn rebuild_reason(&self) -> Option<String> {
        self.fresh().err()
    }
    pub fn fresh(&self) -> Result<()> {
        if self.parser_version != BUILD_CACHE_VERSION
            || self.game_path.is_empty()
            || self.source_fingerprint.is_empty()
            || self.archives.is_empty()
        {
            return Err("配件目录需要按当前解析规则重新建立后再修改".into());
        }
        if !self.scan_complete {
            return Err("配件目录读取不完整，请重新建立后再修改".into());
        }
        let game = Path::new(&self.game_path);
        let current = current_source_fingerprint(game)
            .map_err(|e| format!("无法验证游戏资源，请重新建立目录：{e}"))?;
        if current != self.source_fingerprint {
            return Err("游戏资源已更新，请重新建立配件目录后再修改".into());
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
        Self::scan_sources(&disk_sources(roots), languages)
    }
    fn scan_sources(
        sources: &[Source],
        languages: &crate::localization::Languages,
    ) -> Result<Self> {
        let mut out = Self {
            scan_complete: true,
            ..Self::default()
        };
        let mut sig = String::new();
        let mut appearances = HashMap::<String, Vec<Option<String>>>::new();
        for (index, source) in sources.iter().enumerate() {
            let before = out.warnings.len();
            let paths = source.paths("def/vehicle", true, &mut out.warnings);
            if source.label() == "def.scs" && !paths.iter().any(|p| p.ends_with(".sii")) {
                out.warnings
                    .push("def.scs:def/vehicle 未找到基础配件定义".into());
            }
            if before != out.warnings.len() {
                out.scan_complete = false;
            }
            for input in paths.into_iter().filter(|p| p.ends_with(".sii")) {
                let path = format!("/{input}");
                let doc = match expand_source(&input, &sources[..=index], 0, &mut 0)
                    .and_then(Document::parse_definition)
                {
                    Ok(doc) => doc,
                    Err(error) => {
                        out.scan_complete = false;
                        out.warnings
                            .push(format!("定义 {}:{input} 未加载：{error}", source.label()));
                        continue;
                    }
                };
                let Some(u) = doc.units.iter().find(|u| u.kind.starts_with("accessory_")) else {
                    continue;
                };
                let validated = [
                    &["name"][..],
                    SCALAR_METRICS,
                    ARRAY_METRICS,
                    COMPATIBILITY_ARRAYS,
                    APPEARANCE_FIELDS,
                ]
                .into_iter()
                .try_for_each(|keys| u.validate_unique_values(keys));
                if let Err(error) = validated {
                    out.scan_complete = false;
                    out.warnings
                        .push(format!("定义 {}:{input} 未加载：{error}", source.label()));
                    continue;
                }
                let mut metrics = BTreeMap::new();
                for &key in SCALAR_METRICS {
                    if let Some(value) = u.get(key) {
                        metrics.insert(key.into(), value.into());
                    }
                }
                for &key in ARRAY_METRICS {
                    let values = values(u, key);
                    if !values.is_empty() {
                        metrics.insert(key.into(), display_tokens(&values.join(" · ")));
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
                        APPEARANCE_FIELDS
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
                    source: source.label().to_owned(),
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
            return Err(format!(
                "未发现配件定义，请检查资源目录{}",
                if out.warnings.is_empty() {
                    String::new()
                } else {
                    format!("：{}", out.warnings.join("；"))
                }
            ));
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
        .collect::<std::io::Result<Vec<_>>>()
        .map_err(|e| format!("{}: {e}", game.display()))?
        .into_iter()
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
pub const BUILD_CACHE_VERSION: u32 = 4;
#[derive(Serialize, Deserialize)]
struct BuildCache {
    version: u32,
    fingerprint: String,
    catalog: Catalog,
}

fn build_fingerprint(game: &Path, packs: &[PathBuf]) -> Result<String> {
    let game = game
        .canonicalize()
        .map_err(|e| format!("{}: {e}", game.display()))?;
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

pub fn current_source_fingerprint(game: &Path) -> Result<String> {
    build_fingerprint(game, &game_packs(game)?)
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
    build_lazy(game, cache, progress, &|| Ok(extractor.to_owned()))
}

pub fn build_lazy(
    game: &Path,
    cache: &Path,
    progress: &dyn Fn(&str),
    extractor_provider: &dyn Fn() -> Result<PathBuf>,
) -> Result<Catalog> {
    let packs = game_packs(game)?;
    let fingerprint = build_fingerprint(game, &packs)?;
    let cache_file = cache.join("parts-build-cache-v1.json");
    if let Some(cached) = std::fs::read(&cache_file)
        .ok()
        .and_then(|bytes| serde_json::from_slice::<BuildCache>(&bytes).ok())
    {
        if cached.version == BUILD_CACHE_VERSION
            && cached.catalog.driver_names_schema == crate::localization::DRIVER_NAMES_SCHEMA
            && cached.fingerprint == fingerprint
            && !cached.catalog.definitions.is_empty()
            && cached.catalog.fresh().is_ok()
        {
            progress("游戏资源未变化，复用已建立的配件目录");
            return Ok(cached.catalog);
        }
    }
    let mut sources = Vec::new();
    let mut extractor_path: Option<PathBuf> = None;
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
                    let source = Source::Memory {
                        files,
                        label: pack.file_name().unwrap().to_string_lossy().into(),
                        archive: Some(pack.clone()),
                    };
                    sources.push(source);
                    continue;
                }
                Err(error) => progress(&format!(
                    "{} 使用官方解包器（{error}）",
                    pack.file_name().unwrap().to_string_lossy()
                )),
            }
            if extractor_path.is_none() {
                let path = extractor_provider()?;
                crate::setup::validate_extractor(&path)?;
                extractor_path = Some(path);
            }
            let extractor = extractor_path.as_ref().unwrap();
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
                    let scanned = Catalog::scan(&[(extracted.clone(), "def.scs".into())])?;
                    if !scanned.scan_complete {
                        return Err(scanned.warnings.join("；"));
                    }
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
        sources.push(Source::Disk {
            root,
            label: pack.file_name().unwrap().to_string_lossy().into(),
        });
    }
    progress("读取游戏中英文文本与配件属性");
    let (languages, name_warnings) = crate::localization::load_sources(game, &sources);
    let cacheable_names = name_warnings.is_empty();
    let mut catalog = Catalog::scan_sources(&sources, &languages)?;
    let (driver_names, driver_warnings, driver_names_complete) =
        crate::localization::load_driver_names(game, &sources);
    catalog.driver_names = driver_names;
    catalog.driver_names_schema = if driver_names_complete {
        crate::localization::DRIVER_NAMES_SCHEMA
    } else {
        0
    };
    catalog.warnings.extend(driver_warnings);
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
    catalog.parser_version = BUILD_CACHE_VERSION;
    catalog.game_path = game
        .canonicalize()
        .map_err(|e| e.to_string())?
        .to_string_lossy()
        .into_owned();
    catalog.source_fingerprint = fingerprint.clone();
    catalog.warnings.push(
        "目录包含本体与已识别的官方车型/配件 DLC；Mod 覆盖顺序未导入。未知配件仅供查看。".into(),
    );
    if fingerprint != build_fingerprint(game, &game_packs(game)?)? {
        return Err("建立目录期间游戏资源发生变化，请等待游戏更新完成后重试".into());
    }
    // A temporary language read failure must be retried by Build / update.
    // The partial result remains usable, but must not become a hot-cache hit.
    if cacheable_names && driver_names_complete && catalog.scan_complete {
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
    }
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
            ("def/vehicle/engine.sii".into(), b"SiiNunit {\naccessory_engine_data : .test {\n@include \"../common.inc\"\n}\n}\n".to_vec()),
            ("def/common.inc".into(), b"@include \"/shared/engine.inc\"\n".to_vec()),
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
        let def_listing = listing(&["/vehicle", "common.inc"]);
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
        let sources = vec![Source::Memory {
            files: extracted,
            label: "fixture".into(),
            archive: Some(pack.clone()),
        }];
        let (disk_languages, _) = crate::localization::load(tmp.path(), &roots);
        let (memory_languages, _) = crate::localization::load_sources(tmp.path(), &sources);
        assert_eq!(disk_languages, memory_languages);
        let disk = Catalog::scan_localized(&roots, &disk_languages).unwrap();
        let cached = Catalog::scan_sources(&sources, &memory_languages).unwrap();
        assert_eq!(
            serde_json::to_value(disk).unwrap(),
            serde_json::to_value(cached).unwrap()
        );
        assert!(definition_include_path("def/engine.sii", "../../escape.sui").is_err());
    }

    #[test]
    fn partial_disk_sources_report_errors_and_retry_without_hot_cache() {
        use crate::game_archive::tests::tree_fixture;
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        let cache = tmp.path().join("cache");
        std::fs::create_dir_all(&game).unwrap();
        std::fs::write(game.join("locale.scs"), tree_fixture(&[])).unwrap();
        let pack = game.join("def.scs");
        std::fs::write(&pack, b"synthetic extracted archive").unwrap();
        let meta = std::fs::metadata(&pack).unwrap();
        let canonical = pack.canonicalize().unwrap();
        let canonical = canonical.to_string_lossy();
        let stable = canonical.strip_prefix("\\\\?\\").unwrap_or(&canonical);
        let key = hash(format!("{}:{}:{:?}", stable, meta.len(), meta.modified()).as_bytes());
        let root = cache.join("extracted").join(&key[..16]);
        std::fs::create_dir_all(root.join("def/vehicle")).unwrap();
        std::fs::create_dir_all(root.join("locale")).unwrap();
        std::fs::write(root.join(".complete"), b"ok").unwrap();
        let good = b"SiiNunit {\naccessory_engine_data : .test {\nname: \"Test\"\n}\n}\n";
        std::fs::write(root.join("def/vehicle/good.sii"), good).unwrap();
        std::fs::write(root.join("def/vehicle/bad.sii"), b"SiiNunit { broken").unwrap();
        std::fs::write(
            root.join("def/vehicle/include.sii"),
            b"@include \"missing.inc\"\n",
        )
        .unwrap();
        // A locale tree replaced by a file produces NotADirectory, not absence.
        std::fs::write(root.join("locale/en_gb"), b"temporarily inaccessible tree").unwrap();
        let no_extractor = || Err("must reuse extracted input".into());
        let partial = build_lazy(&game, &cache, &|_| {}, &no_extractor).unwrap();
        assert!(!partial.scan_complete);
        assert!(partial.fresh().is_err());
        for path in ["bad.sii", "include.sii", "en_gb"] {
            assert!(
                partial.warnings.iter().any(|w| w.contains(path)),
                "{path}: {:?}",
                partial.warnings
            );
        }
        assert!(!cache.join("parts-build-cache-v1.json").exists());
        std::fs::remove_file(root.join("def/vehicle/bad.sii")).unwrap();
        std::fs::remove_file(root.join("def/vehicle/include.sii")).unwrap();
        // Even with complete definitions a failed optional directory must not be cached.
        let names_partial = build_lazy(&game, &cache, &|_| {}, &no_extractor).unwrap();
        assert!(names_partial.scan_complete);
        assert!(!cache.join("parts-build-cache-v1.json").exists());
        std::fs::remove_file(root.join("locale/en_gb")).unwrap();
        std::fs::create_dir(root.join("locale/en_gb")).unwrap();
        std::fs::write(
            root.join("locale/en_gb/local.sii"),
            b"SiiNunit {\nlocalization_db : .l {\nkey[]: \"new\"\nval[]: \"Recovered\"\n}\n}\n",
        )
        .unwrap();
        let recovered = build_lazy(&game, &cache, &|_| {}, &no_extractor).unwrap();
        assert!(recovered.scan_complete);
        recovered.fresh().unwrap();
        assert!(cache.join("parts-build-cache-v1.json").exists());
    }

    #[cfg(windows)]
    #[test]
    fn disk_read_and_directory_locks_report_then_recover() {
        use std::os::windows::fs::OpenOptionsExt;
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path();
        std::fs::create_dir_all(root.join("def/vehicle")).unwrap();
        std::fs::create_dir_all(root.join("locale/en_gb")).unwrap();
        let source = Source::Disk {
            root: root.into(),
            label: "fixture".into(),
        };
        let good = b"SiiNunit {\naccessory_engine_data : .test {\nname: \"Test\"\n}\n}\n";
        std::fs::write(root.join("def/vehicle/a.sii"), good).unwrap();
        std::fs::write(root.join("def/vehicle/b.sii"), good).unwrap();
        let file_lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(root.join("def/vehicle/b.sii"))
            .unwrap();
        let partial =
            Catalog::scan_sources(std::slice::from_ref(&source), &BTreeMap::new()).unwrap();
        assert!(!partial.scan_complete);
        assert_eq!(partial.definitions.len(), 1);
        assert!(partial.warnings.iter().any(|w| w.contains("b.sii")));
        drop(file_lock);
        let complete =
            Catalog::scan_sources(std::slice::from_ref(&source), &BTreeMap::new()).unwrap();
        assert!(complete.scan_complete);
        assert_eq!(complete.definitions.len(), 2);
        let locale = root.join("locale/en_gb");
        let directory_lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(0x02000000)
            .open(&locale)
            .unwrap();
        let mut warnings = Vec::new();
        source.paths("locale/en_gb", false, &mut warnings);
        assert!(warnings.iter().any(|w| w.contains("en_gb")), "{warnings:?}");
        drop(directory_lock);
        warnings.clear();
        source.paths("locale/en_gb", false, &mut warnings);
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn cross_package_includes_use_current_then_previous_sources() {
        let temp = tempfile::tempdir().unwrap();
        let base = BTreeMap::from([
            (
                "def/vehicle/truck/common.sui".into(),
                b"name: \"Shared base\"\n@include \"detail.sui\"\n".to_vec(),
            ),
            (
                "def/vehicle/truck/detail.sui".into(),
                b"torque: 1000\n".to_vec(),
            ),
            (
                "def/vehicle/truck/base.sii".into(),
                b"SiiNunit {\naccessory_engine_data : .base {\n@include \"common.sui\"\n}\n}\n"
                    .to_vec(),
            ),
        ]);
        let first = BTreeMap::from([
            (
                "def/vehicle/truck/detail.sui".into(),
                b"torque: 2000\n".to_vec(),
            ),
            (
                "def/vehicle/truck/dlc.sii".into(),
                b"SiiNunit {\naccessory_engine_data : .dlc {\n@include \"common.sui\"\n}\n}\n"
                    .to_vec(),
            ),
        ]);
        let last = BTreeMap::from([
            (
                "def/vehicle/truck/common.sui".into(),
                b"name: \"Final DLC\"\n@include \"detail.sui\"\n".to_vec(),
            ),
            (
                "def/vehicle/truck/dlc.sii".into(),
                b"SiiNunit {\naccessory_engine_data : .last {\n@include \"common.sui\"\n}\n}\n"
                    .to_vec(),
            ),
        ]);
        let memory = vec![
            Source::Memory {
                files: base,
                label: "def.scs".into(),
                archive: None,
            },
            Source::Memory {
                files: first,
                label: "dlc_first.scs".into(),
                archive: None,
            },
            Source::Memory {
                files: last,
                label: "dlc_last.scs".into(),
                archive: None,
            },
        ];
        let mut disk = Vec::new();
        for (index, source) in memory.iter().enumerate() {
            let root = temp.path().join(index.to_string());
            let Source::Memory { files, .. } = source else {
                unreachable!()
            };
            for (path, bytes) in files {
                let target = root.join(path);
                std::fs::create_dir_all(target.parent().unwrap()).unwrap();
                std::fs::write(target, bytes).unwrap();
            }
            disk.push(Source::Disk {
                root,
                label: source.label().into(),
            });
        }
        for sources in [&memory, &disk] {
            let first_two = Catalog::scan_sources(&sources[..2], &BTreeMap::new()).unwrap();
            assert!(first_two.scan_complete, "{:?}", first_two.warnings);
            assert_eq!(
                first_two.definitions["/def/vehicle/truck/base.sii"].metrics["torque"],
                "1000"
            );
            let included = &first_two.definitions["/def/vehicle/truck/dlc.sii"];
            assert_eq!(included.name, "Shared base");
            assert_eq!(included.metrics["torque"], "2000");
            let all = Catalog::scan_sources(sources, &BTreeMap::new()).unwrap();
            assert!(all.scan_complete, "{:?}", all.warnings);
            let overridden = &all.definitions["/def/vehicle/truck/dlc.sii"];
            assert_eq!(overridden.name, "Final DLC");
            assert_eq!(overridden.source, "dlc_last.scs");
            assert_eq!(overridden.unit, ".last");
            // Last package misses detail.sui; newest earlier package supplies it.
            assert_eq!(overridden.metrics["torque"], "2000");
        }
        assert_eq!(
            serde_json::to_value(Catalog::scan_sources(&memory, &BTreeMap::new()).unwrap())
                .unwrap(),
            serde_json::to_value(Catalog::scan_sources(&disk, &BTreeMap::new()).unwrap()).unwrap()
        );
    }

    #[test]
    fn later_package_can_read_an_unselected_external_include_from_base() {
        use crate::game_archive::tests::{listing, tree_fixture};
        let temp = tempfile::tempdir().unwrap();
        let pack = temp.path().join("def.scs");
        let root = listing(&["/def"]);
        let def = listing(&[]);
        std::fs::write(
            &pack,
            tree_fixture(&[
                ("", 0x81, &root, 0),
                ("def", 0x81, &def, 0),
                (
                    "shared/external.inc",
                    0x80,
                    b"name: \"External base helper\"\n",
                    0,
                ),
            ]),
        )
        .unwrap();
        let base = Source::Memory {
            files: archive_source(&pack).unwrap(),
            label: "base".into(),
            archive: Some(pack),
        };
        let dlc = Source::Memory {
            files: BTreeMap::from([("def/vehicle/engine.sii".into(), b"SiiNunit {\naccessory_engine_data : .dlc {\n@include \"/shared/external.inc\"\n}\n}\n".to_vec())]),
            label: "dlc".into(), archive: None,
        };
        let catalog = Catalog::scan_sources(&[base, dlc], &BTreeMap::new()).unwrap();
        assert!(catalog.scan_complete, "{:?}", catalog.warnings);
        assert_eq!(
            catalog.definitions["/def/vehicle/engine.sii"].name,
            "External base helper"
        );
    }

    #[test]
    fn definition_duplicate_rules_validate_every_consumed_field() {
        let path = "def/vehicle/truck/engine.sii";
        let source = |fields: &str| Source::Memory {
            files: BTreeMap::from([(
                path.into(),
                format!("SiiNunit {{\naccessory_engine_data : .test {{\n{fields}\n}}\n}}\n")
                    .into_bytes(),
            )]),
            label: "fixture".into(),
            archive: None,
        };
        let accepted = source("name: \"Test\"\nname: \"Test\"\ngps_path: \"first\"\ngps_path: \"second\"\ntorque: 1000\ntorque: 1000\ninfo[0]: \"One\"\ninfo[0]: \"One\"\ninfo[]: \"Append\"\ninfo[]: \"Append\"\nsuitable_for[0]: \"truck\"\nsuitable_for[0]: \"truck\"");
        let catalog = Catalog::scan_sources(&[accepted], &BTreeMap::new()).unwrap();
        assert!(catalog.scan_complete);
        let item = &catalog.definitions[&format!("/{path}")];
        assert_eq!(item.name, "Test");
        assert_eq!(item.metrics["torque"], "1000");
        assert_eq!(item.metrics["info"], "One · Append · Append");
        assert_eq!(item.suitable, ["truck"]);
        for key in ["name"]
            .into_iter()
            .chain(SCALAR_METRICS.iter().copied())
            .chain(APPEARANCE_FIELDS.iter().copied())
        {
            let conflicting = source(&format!("{key}: \"first\"\n{key}: \"second\""));
            let error = Catalog::scan_sources(&[conflicting], &BTreeMap::new())
                .err()
                .unwrap();
            assert!(error.contains(key), "{key}: {error}");
        }
        for key in ARRAY_METRICS.iter().chain(COMPATIBILITY_ARRAYS).copied() {
            let conflicting = source(&format!("{key}[0]: \"first\"\n{key}[0]: \"second\""));
            let error = Catalog::scan_sources(&[conflicting], &BTreeMap::new())
                .err()
                .unwrap();
            assert!(error.contains(key), "{key}: {error}");
        }
    }

    #[test]
    fn commented_includes_are_ignored_in_disk_and_memory() {
        let temp = tempfile::tempdir().unwrap();
        let text = "/*\n@include \"missing.inc\"\n*/\nSiiNunit {\naccessory_engine_data : .test {\nname: \"Test\"\n}\n}\n";
        let path = "def/vehicle/test.sii";
        std::fs::create_dir_all(temp.path().join("def/vehicle")).unwrap();
        std::fs::write(temp.path().join(path), text).unwrap();
        let disk = Source::Disk {
            root: temp.path().into(),
            label: "test".into(),
        };
        let memory = Source::Memory {
            files: BTreeMap::from([(path.into(), text.as_bytes().to_vec())]),
            label: "test".into(),
            archive: None,
        };
        let disk_text = expand_source(path, std::slice::from_ref(&disk), 0, &mut 0).unwrap();
        assert_eq!(
            disk_text,
            expand_source(path, std::slice::from_ref(&memory), 0, &mut 0).unwrap()
        );
        assert_eq!(
            Catalog::scan_sources(&[memory], &BTreeMap::new())
                .unwrap()
                .definitions
                .len(),
            1
        );
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
        std::fs::write(game.join("locale.scs"), tree_fixture(&[])).unwrap();
        std::fs::write(game.join("def.scs"), pack("first")).unwrap();
        let provider_calls = std::cell::Cell::new(0);
        let never_extract = || {
            provider_calls.set(provider_calls.get() + 1);
            Err("unexpected extractor request".into())
        };
        let cold = build_lazy(&game, &cache, &|_| {}, &never_extract).unwrap();
        assert_eq!(provider_calls.get(), 0);
        cold.fresh().unwrap();
        let mut legacy: serde_json::Value = serde_json::to_value(&cold).unwrap();
        for field in [
            "parser_version",
            "game_path",
            "source_fingerprint",
            "scan_complete",
            "archives",
        ] {
            let saved = legacy.as_object_mut().unwrap().remove(field).unwrap();
            let old: Catalog = serde_json::from_value(legacy.clone()).unwrap();
            assert!(old.fresh().is_err(), "missing {field} must reject editing");
            legacy[field] = saved;
        }
        let mut old_parser = cold.clone();
        old_parser.parser_version -= 1;
        assert!(old_parser.rebuild_reason().is_some());
        let key = "/def/vehicle/engine.sii";
        assert_eq!(cold.definitions[key].name, "first");
        let hot_messages = std::cell::RefCell::new(Vec::new());
        let hot = build_lazy(
            &game,
            &cache,
            &|message| hot_messages.borrow_mut().push(message.to_owned()),
            &never_extract,
        )
        .unwrap();
        assert_eq!(hot.signature, cold.signature);
        assert_eq!(provider_calls.get(), 0);
        assert_eq!(hot_messages.borrow().len(), 1);
        assert!(hot_messages.borrow()[0].contains("复用"));
        let cache_path = cache.join("parts-build-cache-v1.json");
        let mut without_names: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&cache_path).unwrap()).unwrap();
        without_names["catalog"]
            .as_object_mut()
            .unwrap()
            .remove("driver_names");
        without_names["catalog"]
            .as_object_mut()
            .unwrap()
            .remove("driver_names_schema");
        let legacy_names: Catalog =
            serde_json::from_value(without_names["catalog"].clone()).unwrap();
        legacy_names.fresh().unwrap(); // Names are optional, not a new editing restriction.
        std::fs::write(&cache_path, serde_json::to_vec(&without_names).unwrap()).unwrap();
        let refreshed_names = build(&game, &extractor, &cache).unwrap();
        assert_eq!(
            refreshed_names.driver_names_schema,
            crate::localization::DRIVER_NAMES_SCHEMA
        );
        let mut old: serde_json::Value =
            serde_json::from_slice(&std::fs::read(&cache_path).unwrap()).unwrap();
        old["version"] = serde_json::json!(1);
        old["catalog"]["definitions"][key]["name"] = serde_json::json!("stale parser result");
        std::fs::write(&cache_path, serde_json::to_vec(&old).unwrap()).unwrap();
        assert_eq!(
            build(&game, &extractor, &cache).unwrap().definitions[key].name,
            "first"
        );
        std::fs::write(game.join("def.scs"), pack("changed name")).unwrap();
        assert!(cold.fresh().is_err());
        assert_eq!(
            build(&game, &extractor, &cache).unwrap().definitions[key].name,
            "changed name"
        );
        let before_dlc = build(&game, &extractor, &cache).unwrap();
        std::fs::write(game.join("dlc_daf.scs"), pack("DLC override")).unwrap();
        assert!(before_dlc.fresh().is_err());
        assert_eq!(
            build(&game, &extractor, &cache).unwrap().definitions[key].name,
            "DLC override"
        );
        let with_dlc = build(&game, &extractor, &cache).unwrap();
        std::fs::remove_file(game.join("dlc_daf.scs")).unwrap();
        assert!(with_dlc.fresh().is_err());
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
        let same_second = build(&game, &extractor, &cache).unwrap();
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
        assert!(same_second.fresh().is_err());
        std::fs::write(game.join("def.scs"), b"unsupported synthetic archive").unwrap();
        assert!(build_lazy(&game, &cache, &|_| {}, &never_extract)
            .err()
            .unwrap()
            .contains("unexpected extractor request"));
        assert_eq!(provider_calls.get(), 1);
    }

    #[cfg(windows)]
    #[test]
    fn locale_lock_does_not_cache_incomplete_names() {
        use crate::game_archive::tests::{listing, tree_fixture};
        use std::os::windows::fs::OpenOptionsExt;
        let tmp = tempfile::tempdir().unwrap();
        let game = tmp.path().join("game");
        std::fs::create_dir_all(&game).unwrap();
        let root = listing(&["/def"]);
        let def = listing(&["/vehicle"]);
        let vehicle = listing(&["engine.sii"]);
        let text = b"SiiNunit {\naccessory_engine_data : .test {\nname: \"@@engine@@\"\n}\n}\n";
        std::fs::write(
            game.join("def.scs"),
            tree_fixture(&[
                ("", 0x81, &root, 0),
                ("def", 0x81, &def, 0),
                ("def/vehicle", 0x81, &vehicle, 0),
                ("def/vehicle/engine.sii", 0x80, text, 0),
            ]),
        )
        .unwrap();
        let locale =
            b"SiiNunit {\nlocalization_db : .l {\nkey[]: \"engine\"\nval[]: \"Translated\"\n}\n}\n";
        let locale_file = game.join("locale.scs");
        std::fs::write(
            &locale_file,
            tree_fixture(&[
                ("locale/en_gb/local.sii", 0x80, locale, 0),
                ("locale/zh_cn/local.sii", 0x80, locale, 0),
            ]),
        )
        .unwrap();
        let original_time = std::fs::metadata(&locale_file).unwrap().modified().unwrap();
        let extractor = tmp.path().join("unused-extractor.exe");
        std::fs::write(&extractor, b"unused").unwrap();
        let cache = tmp.path().join("cache");
        let lock = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&locale_file)
            .unwrap();
        let incomplete = build(&game, &extractor, &cache).unwrap();
        assert_eq!(incomplete.name_schema, 0);
        assert!(incomplete.warnings.iter().any(|w| w.contains("未读取")));
        assert!(!cache.join("parts-build-cache-v1.json").exists());
        drop(lock);
        assert_eq!(
            std::fs::metadata(&locale_file).unwrap().modified().unwrap(),
            original_time
        );
        let recovered = build(&game, &extractor, &cache).unwrap();
        assert_eq!(recovered.name_schema, 1);
        assert_eq!(
            recovered.definitions["/def/vehicle/engine.sii"].name,
            "Translated"
        );
        let messages = std::cell::RefCell::new(Vec::new());
        let hot = build_with_progress(&game, &extractor, &cache, &|m| {
            messages.borrow_mut().push(m.to_owned())
        })
        .unwrap();
        assert_eq!(
            serde_json::to_value(hot).unwrap(),
            serde_json::to_value(recovered).unwrap()
        );
        assert_eq!(messages.borrow().len(), 1);
        assert!(messages.borrow()[0].contains("复用"));
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
