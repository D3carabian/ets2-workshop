use crate::{
    catalog::Catalog,
    decoder,
    edit::{Change, Operation, Preview},
    garage, hash,
    sii::{quoted, unquote, Document},
    Result,
};
use serde::{Deserialize, Serialize};
use std::{
    io::Write,
    path::{Path, PathBuf},
};
#[derive(Clone, Serialize, Deserialize)]
pub struct Settings {
    pub documents: String,
    pub game: String,
    #[serde(default)]
    pub extractor: String,
    #[serde(default)]
    pub onboarding_version: u32,
}
impl Default for Settings {
    fn default() -> Self {
        let home = std::env::var("USERPROFILE").unwrap_or_default();
        let docs = directories::UserDirs::new()
            .and_then(|u| u.document_dir().map(Path::to_path_buf))
            .unwrap_or_else(|| PathBuf::from(&home).join("Documents"));
        let mut game = String::new();
        let log = docs.join("Euro Truck Simulator 2/game.log.txt");
        if let Ok(t) = std::fs::read_to_string(log) {
            if let Some(line) = t.lines().find(|l| l.contains("[sys] Command line:")) {
                if let Some(p) = line.split("Command line:").nth(1) {
                    let p = p.trim().trim_matches('"').replace('\\', "/");
                    if let Some((root, _)) = p.split_once("/bin/") {
                        game = root.into();
                    }
                }
            }
        }
        Self {
            documents: docs.join("Euro Truck Simulator 2").to_string_lossy().into(),
            game,
            extractor: crate::setup::managed_extractor().to_string_lossy().into(),
            onboarding_version: 0,
        }
    }
}
pub fn app_dir() -> PathBuf {
    if let Some(p) = std::env::var_os("ETS2_WORKSHOP_DATA_DIR") {
        return PathBuf::from(p);
    }
    directories::ProjectDirs::from("local", "ETS2", "Workshop")
        .unwrap()
        .data_local_dir()
        .into()
}
pub fn settings() -> Settings {
    std::fs::read(app_dir().join("settings.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default()
}
pub fn save_settings(s: &Settings) -> Result<()> {
    std::fs::create_dir_all(app_dir()).map_err(|e| e.to_string())?;
    std::fs::write(
        app_dir().join("settings.json"),
        serde_json::to_vec_pretty(s).unwrap(),
    )
    .map_err(|e| e.to_string())
}
#[derive(Clone, Serialize)]
pub struct SaveEntry {
    pub path: String,
    pub name: String,
    pub profile: String,
    pub modified: u64,
    pub error: Option<String>,
}
pub fn discover(s: &Settings) -> Result<Vec<SaveEntry>> {
    let mut roots = vec![
        PathBuf::from(&s.documents).join("profiles"),
        PathBuf::from(&s.documents).join("steam_profiles"),
    ];
    for steam in crate::setup::steam_roots() {
        if let Ok(users) = std::fs::read_dir(Path::new(&steam).join("userdata")) {
            for u in users.flatten() {
                roots.push(u.path().join("227300/remote/profiles"));
            }
        }
    }
    let mut out = Vec::new();
    let mut seen = std::collections::HashSet::new();
    for root in roots {
        for e in walkdir::WalkDir::new(root)
            .max_depth(4)
            .follow_links(false)
            .into_iter()
            .filter_map(|e| e.ok())
        {
            if e.file_name() != "info.sii" || !e.path().parent().unwrap().join("game.sii").exists()
            {
                continue;
            }
            let path = e.path().parent().unwrap().join("game.sii");
            if !seen.insert(path.clone()) {
                continue;
            }
            let profile = e
                .path()
                .ancestors()
                .nth(3)
                .and_then(|p| p.file_name())
                .unwrap_or_default()
                .to_string_lossy()
                .to_string();
            let modified = std::fs::metadata(&path)
                .ok()
                .and_then(|m| m.modified().ok())
                .and_then(|t| t.duration_since(std::time::UNIX_EPOCH).ok())
                .map(|d| d.as_secs())
                .unwrap_or(0);
            let (name, error) = match decoder::read(e.path()).and_then(Document::parse) {
                Ok(d) => (
                    d.units
                        .first()
                        .and_then(|u| u.get("name"))
                        .map(unquote)
                        .filter(|s| !s.is_empty())
                        .unwrap_or_else(|| {
                            path.parent()
                                .unwrap()
                                .file_name()
                                .unwrap()
                                .to_string_lossy()
                                .into()
                        }),
                    None,
                ),
                Err(e) => ("无法读取名称".into(), Some(e)),
            };
            out.push(SaveEntry {
                path: path.to_string_lossy().into(),
                name,
                profile,
                modified,
                error,
            });
        }
    }
    out.sort_by_key(|s| std::cmp::Reverse(s.modified));
    Ok(out)
}
pub fn has_mod_dependencies(info: &str) -> Result<bool> {
    if info.is_empty() {
        return Err("缺少 info.sii，无法确认存档依赖；请选择完整存档目录".into());
    }
    let doc = Document::parse(info.into())?;
    let container = doc
        .units
        .iter()
        .find(|u| u.kind == "save_container")
        .ok_or("存档信息缺少 save_container")?;
    let dependencies = container.array("dependencies")?;
    Ok(dependencies
        .iter()
        .map(|s| unquote(s))
        .any(|s| !(s.starts_with("dlc|") || s.starts_with("rdlc|"))))
}
#[derive(Clone)]
pub struct Session {
    pub path: PathBuf,
    pub original_hash: String,
    pub info_hash: Option<String>,
    pub doc: Document,
    pub mods: bool,
}
#[derive(Serialize)]
pub struct Opened {
    pub path: String,
    pub hash: String,
    pub trucks: Vec<garage::Truck>,
    pub warnings: Vec<String>,
}
impl Session {
    pub fn open(path: &Path, _s: &Settings) -> Result<Self> {
        let raw = std::fs::read(path).map_err(|e| e.to_string())?;
        let original_hash = hash(&raw);
        let doc = Document::parse(decoder::read(path)?)?;
        if hash(&std::fs::read(path).map_err(|e| e.to_string())?) != original_hash {
            return Err("读取期间存档发生变化，请重试".into());
        }
        doc.validate_vehicles()?;
        let info = path.parent().unwrap().join("info.sii");
        let info_hash = std::fs::read(&info).ok().map(|b| hash(&b));
        let info_text = if info.exists() {
            decoder::read(&info)?
        } else {
            String::new()
        };
        let mods = has_mod_dependencies(&info_text)?;
        if mods {
            return Err(
                "此存档包含 Mod 或未知扩展依赖，当前版本不支持。请使用原版及官方 DLC 存档。".into(),
            );
        }
        Ok(Self {
            path: path.into(),
            original_hash,
            info_hash,
            doc,
            mods,
        })
    }
    pub fn view(&self, c: &Catalog) -> Result<Opened> {
        let mut warnings = Vec::new();
        if self.mods {
            warnings.push("此存档声明了 Mod 依赖，当前版本不支持。".into());
        }
        if c.definitions.is_empty() {
            warnings.push("尚未建立配件目录。所有配件可查看；编辑前请在设置中建立目录。".into());
        }
        Ok(Opened {
            path: self.path.to_string_lossy().into(),
            hash: self.original_hash.clone(),
            trucks: garage::inventory(&self.doc, c)?,
            warnings,
        })
    }
    pub fn preview(&self, c: &Catalog, ops: &[Operation]) -> Result<(Document, Preview)> {
        if !ops.is_empty() {
            c.fresh()?;
        }
        if self.mods && !ops.is_empty() {
            return Err("当前版本不支持 Mod 存档".into());
        }
        crate::edit::apply(&self.doc, c, ops)
    }
    pub fn fresh(&self) -> Result<()> {
        if hash(&std::fs::read(&self.path).map_err(|e| e.to_string())?) != self.original_hash {
            return Err("源存档已被游戏或其他程序更新，请重新打开后再修改".into());
        }
        if let Some(h) = &self.info_hash {
            let b = std::fs::read(self.path.parent().unwrap().join("info.sii"))
                .map_err(|e| e.to_string())?;
            if &hash(&b) != h {
                return Err("存档信息已更新，请重新打开".into());
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Receipt {
    pub id: String,
    pub source: String,
    pub output: String,
    pub backup: String,
    pub before_hash: String,
    pub after_hash: String,
    pub changes: Vec<Change>,
}
pub fn receipts() -> Vec<Receipt> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(app_dir().join("history")) {
        for e in entries.flatten() {
            if let Ok(b) = std::fs::read(e.path().join("receipt.json")) {
                if let Ok(r) = serde_json::from_slice(&b) {
                    out.push(r)
                }
            }
        }
    }
    out.sort_by(|a: &Receipt, b| b.id.cmp(&a.id));
    out
}
fn copy_files(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to).map_err(|e| e.to_string())?;
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        if e.file_type().map_err(|e| e.to_string())?.is_file() {
            std::fs::copy(e.path(), to.join(e.file_name())).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())?;
    drop(f);
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let a: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                a.as_ptr(),
                b.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            let _ = std::fs::remove_file(&temp);
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
    Ok(())
}
pub fn commit(
    session: &Session,
    catalog: &Catalog,
    ops: &[Operation],
    mode: &str,
    name: &str,
    _settings: &Settings,
) -> Result<Receipt> {
    if ops.is_empty() {
        return Err("没有待保存的修改".into());
    }
    if mode != "new" && mode != "overwrite" {
        return Err("未知保存模式".into());
    }
    session.fresh()?;
    let (doc, preview) = session.preview(catalog, ops)?;
    let id = format!(
        "{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        &uuid::Uuid::new_v4().simple().to_string()[..8]
    );
    let history = app_dir().join("history").join(&id);
    let backup = history.join("original");
    copy_files(session.path.parent().unwrap(), &backup)?;
    session.fresh()?;
    if hash(
        &std::fs::read(backup.join(session.path.file_name().unwrap()))
            .map_err(|e| e.to_string())?,
    ) != session.original_hash
    {
        return Err("备份校验失败".into());
    }
    let output = if mode == "new" {
        if name.trim().is_empty() || name.chars().count() > 80 {
            return Err("新存档名称应为 1–80 个字符".into());
        }
        let name_value = quoted(name)?;
        if session.path.file_name().and_then(|s| s.to_str()) != Some("game.sii") {
            return Err("另存需要完整的游戏存档目录".into());
        }
        let root = session
            .path
            .parent()
            .and_then(Path::parent)
            .ok_or("找不到存档目录")?;
        let slot = std::fs::read_dir(root)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse::<u64>().ok()))
            .max()
            .unwrap_or(0)
            + 1;
        let target = root.join(slot.to_string());
        let staged = root.join(format!(".workshop-{id}"));
        copy_files(&backup, &staged)?;
        let result: Result<PathBuf> = (|| {
            let infopath = staged.join("info.sii");
            let mut info = Document::parse(decoder::read(&infopath)?)?;
            let uid = info.units.first().ok_or("info.sii 无保存信息")?.id.clone();
            info = info.replace(&uid, "name", name_value)?;
            if info.unit(&uid)?.get("file_time").is_some() {
                info = info.replace(
                    &uid,
                    "file_time",
                    std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap()
                        .as_secs()
                        .to_string(),
                )?;
            }
            std::fs::write(staged.join("game.sii"), doc.text.as_bytes())
                .map_err(|e| e.to_string())?;
            std::fs::write(infopath, info.text.as_bytes()).map_err(|e| e.to_string())?;
            session.fresh()?;
            // rename refuses an existing destination on Windows; never merge an occupied slot.
            if target.exists() {
                return Err("目标槽位已被占用，请重试".into());
            }
            std::fs::rename(&staged, &target).map_err(|e| e.to_string())?;
            Ok(target.join("game.sii"))
        })();
        if staged.exists() {
            let _ = std::fs::remove_dir_all(staged);
        }
        result?
    } else {
        session.fresh()?;
        atomic(&session.path, doc.text.as_bytes())?;
        session.path.clone()
    };
    let receipt = Receipt {
        id,
        source: session.path.to_string_lossy().into(),
        output: output.to_string_lossy().into(),
        backup: backup.to_string_lossy().into(),
        before_hash: session.original_hash.clone(),
        after_hash: hash(doc.text.as_bytes()),
        changes: preview.changes,
    };
    if hash(&std::fs::read(&output).map_err(|e| e.to_string())?) != receipt.after_hash {
        return Err("写入后校验失败，备份已保留".into());
    }
    std::fs::write(
        history.join("receipt.json"),
        serde_json::to_vec_pretty(&receipt).unwrap(),
    )
    .map_err(|e| e.to_string())?;
    Ok(receipt)
}
pub fn restore(id: &str) -> Result<String> {
    let r = receipts()
        .into_iter()
        .find(|r| r.id == id)
        .ok_or("找不到本程序的备份记录")?;
    let output = Path::new(&r.output);
    if hash(&std::fs::read(output).map_err(|e| e.to_string())?) != r.after_hash {
        return Err("目标存档已被游戏更新，为避免覆盖进度，不能直接回滚此历史记录".into());
    }
    let data = std::fs::read(Path::new(&r.backup).join("game.sii")).map_err(|e| e.to_string())?;
    if hash(&data) != r.before_hash {
        return Err("备份内容校验失败".into());
    }
    atomic(output, &data)?;
    Ok(r.output)
}
