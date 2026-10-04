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
    #[serde(default)]
    pub catalog_file: String,
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
            catalog_file: String::new(),
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
    crate::setup::write_atomic(
        &app_dir().join("settings.json"),
        &serde_json::to_vec_pretty(s).unwrap(),
    )
}
pub fn catalog_path(s: &Settings, data: &Path) -> PathBuf {
    // Only application-generated basenames are accepted from persisted settings.
    if s.catalog_file.starts_with("catalog-")
        && s.catalog_file.ends_with(".json")
        && !s.catalog_file.contains(['/', '\\', ':'])
    {
        data.join(&s.catalog_file)
    } else {
        data.join("catalog.json")
    }
}
#[derive(Clone, Serialize)]
pub struct SaveEntry {
    pub is_autosave: bool,
    pub path: String,
    pub name: String,
    pub profile: String,
    pub modified: u64,
    pub error: Option<String>,
}
pub fn discover(s: &Settings) -> Result<Vec<SaveEntry>> {
    discover_with(s, &crate::discovery::DiscoveryInputs::from_host())
}
pub fn discover_with(
    s: &Settings,
    inputs: &crate::discovery::DiscoveryInputs,
) -> Result<Vec<SaveEntry>> {
    crate::discovery::discover(Path::new(&s.documents), inputs)
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
    pub info_hash: String,
    pub doc: Document,
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
        Self::open_with(path, decoder::read)
    }
    fn open_with(path: &Path, decode: impl Fn(&Path) -> Result<String>) -> Result<Self> {
        reject_staging(path)?;
        let original_hash = hash(&decoder::read_bounded(path)?);
        let doc = Document::parse(decode(path)?)?;
        doc.validate_vehicles()?;
        let info = path.parent().unwrap().join("info.sii");
        let info_hash = hash(&decoder::read_bounded(&info)?);
        let info_text = decode(&info)?;
        if has_mod_dependencies(&info_text)? {
            return Err(
                "此存档包含 Mod 或未知扩展依赖，当前版本不支持。请使用原版及官方 DLC 存档。".into(),
            );
        }
        if hash(&decoder::read_bounded(path)?) != original_hash {
            return Err("读取期间存档发生变化，请重试".into());
        }
        if hash(&decoder::read_bounded(&info)?) != info_hash {
            return Err("读取期间存档信息发生变化，请重试".into());
        }
        Ok(Self {
            path: path.into(),
            original_hash,
            info_hash,
            doc,
        })
    }
    pub fn view(&self, c: &Catalog) -> Result<Opened> {
        let mut warnings = Vec::new();
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
        crate::edit::apply(&self.doc, c, ops)
    }
    pub fn fresh(&self) -> Result<()> {
        if digest(&self.path)? != self.original_hash {
            return Err("源存档已被游戏或其他程序更新，请重新打开后再修改".into());
        }
        if digest(&self.path.parent().unwrap().join("info.sii"))? != self.info_hash {
            return Err("存档信息已更新，请重新打开".into());
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
    // Older records describe completed writes and did not guard info.sii.
    #[serde(default = "completed")]
    pub state: String,
    #[serde(default)]
    pub info_hash: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
}
fn completed() -> String {
    "completed".into()
}
fn history_dir(id: &str) -> Result<PathBuf> {
    if id.is_empty() || !id.bytes().all(|b| b.is_ascii_hexdigit() || b == b'-') {
        return Err("无效记录编号".into());
    }
    Ok(app_dir().join("history").join(id))
}
pub fn receipt(id: &str) -> Result<Receipt> {
    let r: Receipt = serde_json::from_slice(
        &std::fs::read(history_dir(id)?.join("receipt.json")).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    if r.id != id {
        return Err("记录编号不一致".into());
    }
    Ok(r)
}
pub fn receipts() -> Vec<Receipt> {
    let mut out = Vec::new();
    if let Ok(entries) = std::fs::read_dir(app_dir().join("history")) {
        for e in entries.flatten() {
            if let Some(id) = e.file_name().to_str() {
                if let Ok(r) = receipt(id) {
                    out.push(r);
                }
            }
        }
    }
    // Listing history never reads or hashes vehicle saves.
    out.sort_by(|a, b| b.id.cmp(&a.id));
    out
}
// OS locks are released on process exit; an interrupted operation cannot leave a stale lock.
fn lock_file(path: &Path) -> Result<std::fs::File> {
    let f = std::fs::OpenOptions::new()
        .read(true)
        .write(true)
        .create(true)
        .truncate(false)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.try_lock()
        .map_err(|_| "该存档或记录正在被另一实例操作，请稍后重试")?;
    Ok(f)
}
fn lock_record(id: &str) -> Result<std::fs::File> {
    lock_file(&history_dir(id)?.join("operation.lock"))
}
fn lock_output(path: &Path) -> Result<std::fs::File> {
    let dir = app_dir().join("locks");
    std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    // All callers pass canonical paths. Windows path casing must not create a second lock.
    let key = path.to_string_lossy().to_lowercase();
    lock_file(&dir.join(format!("{}.lock", hash(key.as_bytes()))))
}
fn write_synced(path: &Path, bytes: &[u8]) -> Result<()> {
    let mut f = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(path)
        .map_err(|e| e.to_string())?;
    f.write_all(bytes)
        .and_then(|_| f.sync_all())
        .map_err(|e| e.to_string())
}
fn copy_files(from: &Path, to: &Path, skip: &[&str]) -> Result<()> {
    for e in std::fs::read_dir(from).map_err(|e| e.to_string())? {
        let e = e.map_err(|e| e.to_string())?;
        if e.file_type().map_err(|e| e.to_string())?.is_file() {
            let target = to.join(e.file_name());
            // Avoid copying an earlier interrupted transaction's temporary file.
            if e.file_name().to_string_lossy().starts_with(".workshop-")
                || skip.iter().any(|name| e.file_name() == *name)
            {
                continue;
            }
            let mut input = std::fs::File::open(e.path()).map_err(|e| e.to_string())?;
            let mut output = std::fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .open(target)
                .map_err(|e| e.to_string())?;
            std::io::copy(&mut input, &mut output)
                .and_then(|_| output.sync_all())
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}
fn move_replace(from: &Path, to: &Path) -> Result<()> {
    #[cfg(windows)]
    {
        use std::os::windows::ffi::OsStrExt;
        let a: Vec<u16> = from.as_os_str().encode_wide().chain(Some(0)).collect();
        let b: Vec<u16> = to.as_os_str().encode_wide().chain(Some(0)).collect();
        if unsafe {
            windows_sys::Win32::Storage::FileSystem::MoveFileExW(
                a.as_ptr(),
                b.as_ptr(),
                windows_sys::Win32::Storage::FileSystem::MOVEFILE_REPLACE_EXISTING
                    | windows_sys::Win32::Storage::FileSystem::MOVEFILE_WRITE_THROUGH,
            )
        } == 0
        {
            return Err(std::io::Error::last_os_error().to_string());
        }
    }
    #[cfg(not(windows))]
    std::fs::rename(from, to).map_err(|e| e.to_string())?;
    Ok(())
}
fn atomic(
    path: &Path,
    temp: &Path,
    bytes: &[u8],
    guard: impl FnOnce() -> Result<()>,
) -> Result<()> {
    // create_new prevents a retry from overwriting an interrupted transaction.
    let result = (|| {
        write_synced(temp, bytes)?;
        guard()?; // Recheck after staging potentially large saves, immediately before replace.
        move_replace(temp, path)
    })();
    // Leave interrupted data at the transaction-specific path for explicit cleanup.
    result
}
fn persist(r: &Receipt) -> Result<()> {
    let dir = history_dir(&r.id)?;
    let temp = dir.join("receipt.tmp");
    let result = atomic(
        &dir.join("receipt.json"),
        &temp,
        &serde_json::to_vec_pretty(r).map_err(|e| e.to_string())?,
        || Ok(()),
    );
    if result.is_err() {
        let _ = std::fs::remove_file(temp);
    }
    result
}
fn digest(path: &Path) -> Result<String> {
    use sha2::{Digest, Sha256};
    use std::io::Read;
    let mut f = std::fs::File::open(path).map_err(|e| e.to_string())?;
    let mut h = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let n = f.read(&mut buffer).map_err(|e| e.to_string())?;
        if n == 0 {
            break;
        }
        h.update(&buffer[..n]);
    }
    Ok(format!("{:x}", h.finalize()))
}
fn check_info(r: &Receipt) -> Result<()> {
    if let Some(expected) = &r.info_hash {
        if &digest(&Path::new(&r.output).with_file_name("info.sii"))? != expected {
            return Err("存档信息已被更新，为避免覆盖进度，不能直接恢复".into());
        }
    }
    Ok(())
}
fn reject_staging(path: &Path) -> Result<()> {
    let canonical = std::fs::canonicalize(path).map_err(|e| e.to_string())?;
    if canonical
        .components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with(".workshop-"))
    {
        return Err("这是未完成保存的暂存文件；请从改装记录清理后重新保存".into());
    }
    Ok(())
}
fn display_path(path: &Path) -> String {
    let path = path.to_string_lossy();
    if let Some(unc) = path.strip_prefix(r"\\?\UNC\") {
        return format!(r"\\{unc}");
    }
    path.strip_prefix(r"\\?\").unwrap_or(&path).into()
}
fn stage_path(r: &Receipt) -> PathBuf {
    Path::new(&r.output)
        .parent()
        .unwrap()
        .with_file_name(format!(".workshop-{}", r.id))
}
fn temp_path(r: &Receipt) -> PathBuf {
    Path::new(&r.output).with_file_name(format!(".workshop-{}.tmp", r.id))
}
// Remove only artifacts named by a durable record, never a numeric save slot or backups.
// Refuse links and unexpected files instead of recursively traversing them.
fn cleanup_record(r: &Receipt) -> Result<()> {
    let temp = temp_path(r);
    if temp.try_exists().map_err(|e| e.to_string())? {
        if !std::fs::symlink_metadata(&temp)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_file()
        {
            return Err("临时路径不是普通文件，未清理".into());
        }
        std::fs::remove_file(&temp).map_err(|e| e.to_string())?;
    }
    if r.source != r.output {
        let staged = stage_path(r);
        if staged.try_exists().map_err(|e| e.to_string())? {
            if !std::fs::symlink_metadata(&staged)
                .map_err(|e| e.to_string())?
                .file_type()
                .is_dir()
            {
                return Err("暂存路径不是普通目录，未清理".into());
            }
            let marker = staged.join(".workshop-owner");
            let entries: Vec<_> = std::fs::read_dir(&staged)
                .map_err(|e| e.to_string())?
                .collect::<std::io::Result<_>>()
                .map_err(|e| e.to_string())?;
            // Directory creation, marker creation and cleanup can each be interrupted.
            // With no payload present, an empty directory or partial marker is safe to remove.
            let marker_only = entries.iter().all(|e| e.path() == marker);
            if !marker_only && std::fs::read_to_string(&marker).map_err(|e| e.to_string())? != r.id
            {
                return Err("暂存目录归属不明确，未清理".into());
            }
            for e in &entries {
                if !e.file_type().map_err(|e| e.to_string())?.is_file()
                    || (e.file_name() != ".workshop-owner"
                        && !Path::new(&r.backup).join(e.file_name()).is_file())
                {
                    return Err("暂存目录含未知内容，未清理".into());
                }
            }
            for e in &entries {
                if e.path() != marker {
                    std::fs::remove_file(e.path()).map_err(|e| e.to_string())?;
                }
            }
            if !entries.is_empty() {
                std::fs::remove_file(marker).map_err(|e| e.to_string())?;
            }
            std::fs::remove_dir(staged).map_err(|e| e.to_string())?;
        }
    }
    let temp = history_dir(&r.id)?.join("receipt.tmp");
    if temp.is_file() {
        std::fs::remove_file(temp).map_err(|e| e.to_string())?;
    }
    Ok(())
}
pub fn cleanup(id: &str) -> Result<()> {
    let _lock = lock_record(id)?;
    cleanup_record(&receipt(id)?)
}

pub fn commit(
    session: &Session,
    catalog: &Catalog,
    ops: &[Operation],
    mode: &str,
    name: &str,
    _settings: &Settings,
) -> Result<Receipt> {
    commit_with(session, catalog, ops, mode, name, &|_, _| Ok(()))
}
// Local checkpoints make destructive failure paths reproducible without global fault flags.
fn commit_with(
    session: &Session,
    catalog: &Catalog,
    ops: &[Operation],
    mode: &str,
    name: &str,
    checkpoint: &dyn Fn(&str, &Receipt) -> Result<()>,
) -> Result<Receipt> {
    reject_staging(&session.path)?;
    if ops.is_empty() {
        return Err("没有待保存的修改".into());
    }
    if mode != "new" && mode != "overwrite" {
        return Err("未知保存模式".into());
    }
    if session.path.file_name().and_then(|s| s.to_str()) != Some("game.sii") {
        return Err("保存需要完整的游戏存档目录及 game.sii".into());
    }
    let name_value = if mode == "new" {
        if name.trim().is_empty() || name.chars().count() > 80 {
            return Err("新存档名称应为 1–80 个字符".into());
        }
        Some(quoted(name)?)
    } else {
        None
    };
    let source = std::fs::canonicalize(&session.path).map_err(|e| e.to_string())?;
    let _source_lock = lock_output(&source)?;
    session.fresh()?;
    let (doc, Preview { changes, .. }) = session.preview(catalog, ops)?;
    let id = format!(
        "{}-{}",
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
        uuid::Uuid::new_v4().simple()
    );
    let output = if mode == "new" {
        let root = source
            .parent()
            .and_then(Path::parent)
            .ok_or("找不到存档目录")?;
        let slot = std::fs::read_dir(root)
            .map_err(|e| e.to_string())?
            .filter_map(|e| e.ok())
            .filter_map(|e| e.file_name().to_str().and_then(|s| s.parse::<u64>().ok()))
            .max()
            .unwrap_or(0)
            .checked_add(1)
            .ok_or("存档槽位编号已满")?;
        root.join(slot.to_string()).join("game.sii")
    } else {
        source.clone()
    };
    let history = history_dir(&id)?;
    std::fs::create_dir_all(history.parent().unwrap()).map_err(|e| e.to_string())?;
    std::fs::create_dir(&history).map_err(|e| e.to_string())?;
    let _lock = lock_record(&id)?;
    let backup = history.join("original");
    let mut r = Receipt {
        id,
        source: display_path(&source),
        output: display_path(&output),
        backup: backup.to_string_lossy().into(),
        before_hash: session.original_hash.clone(),
        after_hash: hash(doc.text.as_bytes()),
        changes,
        state: "preparing".into(),
        info_hash: Some(session.info_hash.clone()),
        warning: None,
    };
    if let Err(e) = persist(&r) {
        drop(_lock);
        let _ = std::fs::remove_file(history.join("operation.lock"));
        let _ = std::fs::remove_dir(&history);
        return Err(format!("本工具未改写原档，无法建立恢复记录：{e}"));
    }
    let prepare: Result<()> = (|| {
        checkpoint("backup", &r)?;
        std::fs::create_dir(&backup).map_err(|e| e.to_string())?;
        copy_files(source.parent().unwrap(), &backup, &[])?;
        session.fresh()?;
        if digest(&backup.join("game.sii"))? != r.before_hash
            || session.info_hash != digest(&backup.join("info.sii"))?
        {
            return Err("备份校验失败".into());
        }
        if let Some(name_value) = name_value {
            let staged = stage_path(&r);
            std::fs::create_dir(&staged).map_err(|e| e.to_string())?;
            write_synced(&staged.join(".workshop-owner"), r.id.as_bytes())?;
            copy_files(&backup, &staged, &["game.sii", "info.sii"])?;
            let infopath = staged.join("info.sii");
            let mut info = Document::parse(decoder::read(&backup.join("info.sii"))?)?;
            let uid = info
                .units
                .iter()
                .find(|u| u.kind == "save_container")
                .ok_or("info.sii 无保存信息")?
                .id
                .clone();
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
            // These files are private staged copies, never visible game saves.
            write_synced(&staged.join("game.sii"), doc.text.as_bytes())?;
            write_synced(&infopath, info.text.as_bytes())?;
            r.info_hash = Some(hash(info.text.as_bytes()));
        }
        r.state = "prepared".into();
        checkpoint("record", &r)?;
        persist(&r)?; // Recovery metadata and backup must precede publication.
        checkpoint("publish", &r)?;
        session.fresh()?;
        Ok(())
    })();
    if let Err(e) = prepare {
        let cleanup = cleanup_record(&r)
            .err()
            .map(|e| format!("；临时文件清理失败：{e}，可在记录中重试"))
            .unwrap_or_default();
        return Err(format!(
            "本工具未改写原档，未发布新存档：{e}。记录 {}；备份位置 {}{cleanup}",
            r.id, r.backup
        ));
    }
    let publish = if mode == "new" {
        let staged = stage_path(&r);
        let target = output.parent().unwrap();
        if target.exists() {
            Err("目标槽位已被占用，请重试".into())
        } else {
            std::fs::rename(&staged, target).map_err(|e| e.to_string())
        }
    } else {
        atomic(&output, &temp_path(&r), doc.text.as_bytes(), || {
            session.fresh()
        })
    };
    if let Err(e) = publish {
        let cleanup = cleanup_record(&r)
            .err()
            .map(|e| format!("；清理失败：{e}"))
            .unwrap_or_default();
        return Err(format!(
            "本工具未改写原档，未发布新存档：{e}。记录 {}；备份位置 {}{cleanup}",
            r.id, r.backup
        ));
    }
    // From this point on, never report a plain failure suggesting it is safe to reapply operations.
    r.state = "completed".into();
    if let Err(e) = checkpoint("verify", &r).and_then(|_| {
        if digest(&output)? != r.after_hash {
            return Err("写入后内容不一致".into());
        }
        check_info(&r)
    }) {
        r.state = "prepared".into();
        r.warning = Some(format!(
            "存档已写入，但后续校验未通过：{e}。请勿重复保存；到改装记录核对或恢复。备份：{}",
            r.backup
        ));
        return Ok(r);
    }
    if mode == "new" {
        let _ = std::fs::remove_file(output.with_file_name(".workshop-owner"));
    }
    if let Err(e) = checkpoint("finalize", &r).and_then(|_| persist(&r)) {
        r.warning = Some(format!(
            "存档已保存并校验，但记录确认失败：{e}。恢复记录和备份已保留：{}",
            r.backup
        ));
    }
    Ok(r)
}
pub fn restore(id: &str) -> Result<String> {
    let _lock = lock_record(id)?;
    let r = receipt(id)?;
    let output = Path::new(&r.output);
    let canonical = std::fs::canonicalize(output).map_err(|e| e.to_string())?;
    let _output_lock = lock_output(&canonical)?;
    check_info(&r)?;
    let current = digest(output)?;
    if current == r.before_hash {
        return Ok(r.output);
    } // Retrying a completed restore is harmless.
    if current != r.after_hash {
        return Err("目标存档已被游戏更新，为避免覆盖进度，不能直接回滚此历史记录".into());
    }
    let data = decoder::read_bounded(&Path::new(&r.backup).join("game.sii"))?;
    if hash(&data) != r.before_hash {
        return Err("备份内容校验失败".into());
    }
    cleanup_record(&r)?;
    atomic(output, &temp_path(&r), &data, || {
        check_info(&r)?;
        if digest(output)? != r.after_hash {
            return Err("目标存档已被更新，取消恢复".into());
        }
        Ok(())
    })
    .map_err(|e| format!("未恢复，备份已保留：{e}；可清理临时文件后重试"))?;
    if digest(output).as_ref() != Ok(&r.before_hash) {
        return Err(format!(
            "恢复已写入但校验失败，请检查目标存档；备份：{}",
            r.backup
        ));
    }
    Ok(r.output)
}

#[cfg(test)]
#[path = "storage_tests.rs"]
mod tests;
