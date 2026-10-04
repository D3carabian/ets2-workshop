//! First-run discovery and acquisition of the official SCS extractor.
use crate::{
    hash,
    storage::{app_dir, Settings},
    Result,
};
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
pub const EXTRACTOR_URL: &str = "https://download.eurotrucksimulator2.com/scs_extractor_1_55.zip";
pub const ARCHIVE_SHA: &str = "45385795fa830b975bd5c80a2076e17fa8398d8d23d679325e3b89b528b0229c";
pub const EXE_SHA: &str = "55bd670691bee62c218220026a0b055bb597eb2c360e33e635c1c4c5c370ea29";
pub const SETUP_VERSION: u32 = 1;
pub use crate::discovery::{library_paths, Detection, DiscoveryInputs};
pub fn steam_roots() -> Vec<String> {
    crate::discovery::detect(&DiscoveryInputs::from_host()).steam_roots
}
pub fn detect() -> Detection {
    detect_with(&DiscoveryInputs::from_host())
}
pub fn detect_with(inputs: &DiscoveryInputs) -> Detection {
    crate::discovery::detect(inputs)
}
pub fn managed_extractor() -> PathBuf {
    app_dir().join("tools/scs-extractor-1.55/scs_extractor.exe")
}
// Atomic replacement keeps a valid cache/settings file intact on write failure.
pub(crate) fn write_atomic(path: &Path, bytes: &[u8]) -> Result<()> {
    use std::io::Write;
    let temp = path.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
    let result = (|| {
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
            use windows_sys::Win32::Storage::FileSystem::*;
            let from: Vec<u16> = temp.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe {
                MoveFileExW(
                    from.as_ptr(),
                    to.as_ptr(),
                    MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
                )
            } == 0
            {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        std::fs::rename(&temp, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temp);
    }
    result
}
pub fn unpack_verified(bytes: &[u8]) -> Result<Vec<u8>> {
    unpack_with_hashes(bytes, ARCHIVE_SHA, EXE_SHA)
}
fn unpack_with_hashes(bytes: &[u8], archive_sha: &str, exe_sha: &str) -> Result<Vec<u8>> {
    if bytes.len() > 8 * 1024 * 1024 || hash(bytes) != archive_sha {
        return Err("官方下载文件校验失败，未安装或执行。请更新应用后重试。".into());
    }
    let mut zip = zip::ZipArchive::new(std::io::Cursor::new(bytes)).map_err(|e| e.to_string())?;
    let file = zip
        .by_name("scs_extractor.exe")
        .map_err(|e| e.to_string())?;
    if file.size() > 4 * 1024 * 1024 {
        return Err("解包工具尺寸异常".into());
    }
    let mut out = Vec::new();
    file.take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut out)
        .map_err(|e| e.to_string())?;
    if out.len() > 4 * 1024 * 1024 || hash(&out) != exe_sha {
        return Err("解包工具校验失败，未安装或执行。请更新应用后重试。".into());
    }
    Ok(out)
}
fn download_extractor() -> Result<Vec<u8>> {
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(60))
        .redirect(reqwest::redirect::Policy::limited(3))
        .build()
        .map_err(|e| e.to_string())?;
    let response = client
        .get(EXTRACTOR_URL)
        .send()
        .and_then(|r| r.error_for_status())
        .map_err(|e| format!("无法下载官方解包工具：{e}。检查网络后可重试。"))?;
    let mut bytes = Vec::new();
    response
        .take(8 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("下载中断：{e}。检查网络后可重试。"))?;
    Ok(bytes)
}
fn acquire_extractor(
    target: &Path,
    exe_sha: &str,
    download: impl FnOnce() -> Result<Vec<u8>>,
    unpack: impl FnOnce(&[u8]) -> Result<Vec<u8>>,
    progress: &dyn Fn(&str),
) -> Result<PathBuf> {
    if validate_extractor_hash(target, exe_sha).is_ok() {
        progress("解包工具已就绪");
        return Ok(target.into());
    }
    progress("正在从 SCS 官方下载解包工具…");
    let exe = unpack(&download()?)?;
    std::fs::create_dir_all(target.parent().ok_or("解包工具路径无效")?)
        .map_err(|e| format!("无法建立工具缓存：{e}。请检查应用数据目录权限和剩余空间后重试。"))?;
    write_atomic(target, &exe).map_err(|e| {
        format!("无法保存解包工具：{e}。请关闭占用工具的程序，检查目录权限和剩余空间后重试。")
    })?;
    progress("官方解包工具已下载并通过校验");
    Ok(target.into())
}
fn validate_extractor_hash(path: &Path, expected: &str) -> Result<()> {
    let mut bytes = Vec::new();
    std::fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(4 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 4 * 1024 * 1024 || hash(&bytes) != expected {
        return Err("解包工具校验失败，未执行。请重新准备官方工具".into());
    }
    Ok(())
}
pub fn validate_extractor(path: &Path) -> Result<()> {
    validate_extractor_hash(path, EXE_SHA)
}
pub fn ensure_extractor(progress: &dyn Fn(&str)) -> Result<PathBuf> {
    acquire_extractor(
        &managed_extractor(),
        EXE_SHA,
        download_extractor,
        unpack_verified,
        progress,
    )
}

pub fn prepare(
    s: Settings,
    progress: &dyn Fn(&str),
) -> Result<(Settings, crate::catalog::Catalog)> {
    prepare_with(s, &app_dir(), progress, |s| {
        crate::catalog::build_lazy(Path::new(&s.game), &app_dir(), progress, &|| {
            ensure_extractor(progress)
        })
    })
}
fn prepare_with(
    mut s: Settings,
    data: &Path,
    progress: &dyn Fn(&str),
    build: impl FnOnce(&Settings) -> Result<crate::catalog::Catalog>,
) -> Result<(Settings, crate::catalog::Catalog)> {
    progress("正在检查游戏和存档目录…");
    validate(&s)?;
    progress("正在建立配件目录…");
    let c = build(&s).map_err(|e| {
        format!("配件目录准备失败：{e}。请检查游戏文件、缓存目录权限和剩余空间后重试。")
    })?;
    s.extractor = managed_extractor().to_string_lossy().into();
    let s = publish_catalog(s, &c, data)?;
    Ok((s, c))
}

/// Publish a complete catalog generation before changing the settings pointer.
/// All configuration and catalog update entry points share this transaction.
pub fn publish_catalog(s: Settings, c: &crate::catalog::Catalog, data: &Path) -> Result<Settings> {
    publish_with(s, c, data, &write_atomic)
}
fn publish_with(
    mut s: Settings,
    c: &crate::catalog::Catalog,
    data: &Path,
    write: &dyn Fn(&Path, &[u8]) -> Result<()>,
) -> Result<Settings> {
    if c.definitions.is_empty() {
        return Err("未发现配件定义，请检查游戏目录后重试。".into());
    }
    std::fs::create_dir_all(data)
        .map_err(|e| format!("无法写入应用数据：{e}。请检查目录权限和剩余空间后重试。"))?;
    let previous: Option<Settings> = std::fs::read(data.join("settings.json"))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok());
    // Commit the settings pointer last: failed reconfiguration preserves the old catalog too.
    s.catalog_file = format!("catalog-{}.json", uuid::Uuid::new_v4().simple());
    let catalog_path = data.join(&s.catalog_file);
    write(&catalog_path, &serde_json::to_vec(&c).unwrap())
        .map_err(|e| format!("无法保存配件目录：{e}。请检查目录权限和剩余空间后重试。"))?;
    s.onboarding_version = SETUP_VERSION;
    if let Err(e) = write(
        &data.join("settings.json"),
        &serde_json::to_vec_pretty(&s).unwrap(),
    ) {
        let _ = std::fs::remove_file(&catalog_path);
        return Err(format!(
            "配置未完成：无法保存设置：{e}。请检查应用数据目录权限后重试。"
        ));
    }
    if let Some(old) = previous {
        let old_path = crate::storage::catalog_path(&old, data);
        // Retire only the previous managed generation AFTER the new pointer is durable.
        if !old.catalog_file.is_empty()
            && old_path.file_name().is_some_and(|n| n != "catalog.json")
            && old_path != catalog_path
        {
            let _ = std::fs::remove_file(old_path);
        }
    }
    Ok(s)
}
pub fn validate(s: &Settings) -> Result<()> {
    let game = Path::new(&s.game);
    if !game.join("def.scs").is_file() || !game.join("bin/win_x64/eurotrucks2.exe").is_file() {
        return Err("游戏目录应包含 def.scs 和 bin/win_x64/eurotrucks2.exe".into());
    }
    let docs = Path::new(&s.documents);
    if !docs.is_dir()
        || !["profiles", "steam_profiles", "config.cfg", "game.log.txt"]
            .iter()
            .any(|n| docs.join(n).exists())
    {
        return Err(
            "请选择 ETS2 用户数据目录，其中应包含 profiles、steam_profiles 或 config.cfg".into(),
        );
    }
    crate::discovery::validate_documents(docs)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn steam_multiple_libraries() {
        let text =
            r#""libraryfolders" { "0" { "path" "C:\\Steam" } "1" { "path" "E:\\SteamLibrary" } }"#;
        assert_eq!(
            library_paths(text),
            vec![
                PathBuf::from("C:\\Steam"),
                PathBuf::from("E:\\SteamLibrary")
            ]
        );
    }
    #[test]
    fn reject_unverified_download() {
        assert!(unpack_verified(b"not a trusted zip").is_err());
    }
}

#[cfg(test)]
mod failure_tests {
    use super::*;
    use crate::{
        catalog::{Catalog, Definition},
        storage,
    };
    use std::collections::{BTreeMap, HashMap};
    fn sandbox() -> tempfile::TempDir {
        let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/setup-tests");
        std::fs::create_dir_all(&root).unwrap();
        tempfile::tempdir_in(root).unwrap()
    }
    fn archive() -> (Vec<u8>, Vec<u8>) {
        use std::io::Write;
        let exe = b"synthetic extractor bytes, never executed".to_vec();
        let mut zip = zip::ZipWriter::new(std::io::Cursor::new(Vec::new()));
        zip.start_file(
            "scs_extractor.exe",
            zip::write::SimpleFileOptions::default(),
        )
        .unwrap();
        zip.write_all(&exe).unwrap();
        (zip.finish().unwrap().into_inner(), exe)
    }
    #[test]
    fn download_failure_hash_failure_corrupt_cache_and_retry() {
        let dir = sandbox();
        let target = dir.path().join("tools/scs_extractor.exe");
        let (zip, exe) = archive();
        let attempt = |download: Result<Vec<u8>>| {
            acquire_extractor(
                &target,
                &hash(&exe),
                || download,
                |b| unpack_with_hashes(b, &hash(&zip), &hash(&exe)),
                &|_| {},
            )
        };
        assert!(attempt(Err("network unavailable; retry".into())).is_err());
        assert!(!target.exists());
        assert!(attempt(Ok(b"invalid download".to_vec())).is_err());
        assert!(!target.exists());
        assert!(unpack_with_hashes(&zip, &hash(&zip), "wrong executable hash").is_err());
        attempt(Ok(zip.clone())).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), exe);
        // A verified cache must work offline without invoking download.
        acquire_extractor(
            &target,
            &hash(&exe),
            || panic!("cache should skip network"),
            |_| panic!("skip unpack"),
            &|_| {},
        )
        .unwrap();
        std::fs::write(&target, b"corrupt cache").unwrap();
        assert!(attempt(Err("offline".into())).is_err());
        assert_eq!(std::fs::read(&target).unwrap(), b"corrupt cache");
        attempt(Ok(zip.clone())).unwrap();
        assert_eq!(std::fs::read(&target).unwrap(), exe);
    }
    #[test]
    fn unwritable_cache_does_not_leave_temporary_files() {
        let dir = sandbox();
        let target = dir.path().join("scs_extractor.exe");
        std::fs::create_dir(&target).unwrap(); // deterministic target conflict on every platform
        let (zip, exe) = archive();
        let e = acquire_extractor(
            &target,
            &hash(&exe),
            || Ok(zip.clone()),
            |b| unpack_with_hashes(b, &hash(&zip), &hash(&exe)),
            &|_| {},
        )
        .unwrap_err();
        assert!(e.contains("重试"));
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 1);
    }
    fn fixture(dir: &Path) -> (Settings, Catalog) {
        let game = dir.join("中文 game");
        std::fs::create_dir_all(game.join("bin/win_x64")).unwrap();
        std::fs::write(game.join("bin/win_x64/eurotrucks2.exe"), b"fixture").unwrap();
        std::fs::write(game.join("def.scs"), b"fixture").unwrap();
        let documents = dir.join("用户 docs");
        std::fs::create_dir_all(documents.join("profiles")).unwrap();
        let s: Settings =
            serde_json::from_value(serde_json::json!({"game":game,"documents":documents})).unwrap();
        let d = Definition {
            raw_name: String::new(),
            names: std::collections::BTreeMap::new(),
            category_names: std::collections::BTreeMap::new(),
            name_alias: None,
            path: "/def/vehicle/truck/test/engine/test.sii".into(),
            kind: "accessory_engine_data".into(),
            unit: "test".into(),
            name: "test".into(),
            category: "engine".into(),
            model: "test".into(),
            source: "synthetic".into(),
            metrics: BTreeMap::new(),
            suitable: vec![],
            conflicts: vec![],
            requires: vec![],
        };
        (
            s,
            Catalog {
                definitions: HashMap::from([(d.path.clone(), d)]),
                ..Catalog::default()
            },
        )
    }
    #[test]
    fn setup_failure_retry_and_reload_are_transactional() {
        let dir = sandbox();
        let data = dir.path().join("app");
        let (s, catalog) = fixture(dir.path());
        let prepare = |s: Settings, c: Result<Catalog>| prepare_with(s, &data, &|_| {}, |_| c);
        assert!(!data.exists()); // discovery/validation require no persistence
        assert!(prepare(s.clone(), Err("extraction failed".into())).is_err());
        assert!(!data.join("settings.json").exists());
        let (saved, _) = prepare(s.clone(), Ok(catalog.clone())).unwrap();
        let bytes = std::fs::read(data.join("settings.json")).unwrap();
        let reloaded: Settings = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(reloaded.onboarding_version, SETUP_VERSION);
        assert_eq!(reloaded.game, s.game);
        let old_catalog = storage::catalog_path(&saved, &data);
        assert!(old_catalog.is_file());
        assert!(prepare(s.clone(), Err("retry failed".into())).is_err());
        assert_eq!(std::fs::read(data.join("settings.json")).unwrap(), bytes);
        assert!(old_catalog.is_file());
        // Simulate failure at the LAST write, after catalog creation.
        std::fs::remove_file(data.join("settings.json")).unwrap();
        std::fs::create_dir(data.join("settings.json")).unwrap();
        let before = std::fs::read_dir(&data).unwrap().count();
        assert!(prepare(s.clone(), Ok(catalog.clone()))
            .err()
            .unwrap()
            .contains("配置未完成"));
        assert_eq!(std::fs::read_dir(&data).unwrap().count(), before);
        std::fs::remove_dir(data.join("settings.json")).unwrap();
        prepare(s, Ok(catalog)).unwrap();
    }
    #[test]
    fn failed_catalog_generation_keeps_previous_active_files() {
        let dir = sandbox();
        let data = dir.path().join("app");
        let (s, c) = fixture(dir.path());
        let published = publish_catalog(s.clone(), &c, &data).unwrap();
        let settings_bytes = std::fs::read(data.join("settings.json")).unwrap();
        let catalog_path = storage::catalog_path(&published, &data);
        let catalog_bytes = std::fs::read(&catalog_path).unwrap();
        assert!(publish_with(s, &c, &data, &|path, _| {
            assert!(path
                .file_name()
                .unwrap()
                .to_string_lossy()
                .starts_with("catalog-"));
            Err("simulated catalog write failure".into())
        })
        .is_err());
        assert_eq!(
            std::fs::read(data.join("settings.json")).unwrap(),
            settings_bytes
        );
        assert_eq!(std::fs::read(catalog_path).unwrap(), catalog_bytes);
        assert_eq!(std::fs::read_dir(data).unwrap().count(), 2);
    }
    #[test]
    #[cfg(windows)]
    fn locked_settings_preserve_previous_configuration_and_allow_retry() {
        use std::os::windows::fs::OpenOptionsExt;
        let dir = sandbox();
        let data = dir.path().join("app");
        let (s, c) = fixture(dir.path());
        let prepare = || prepare_with(s.clone(), &data, &|_| {}, |_| Ok(c.clone()));
        prepare().unwrap();
        let path = data.join("settings.json");
        let previous = std::fs::read(&path).unwrap();
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(&path)
            .unwrap();
        assert!(prepare().is_err());
        drop(locked);
        assert_eq!(std::fs::read(path).unwrap(), previous);
        assert_eq!(std::fs::read_dir(&data).unwrap().count(), 2);
        prepare().unwrap();
        assert_eq!(std::fs::read_dir(&data).unwrap().count(), 2);
    }
}
