//! First-run discovery and acquisition of the official SCS extractor.
use crate::{
    hash,
    storage::{app_dir, Settings},
    Result,
};
use serde::Serialize;
use std::{
    io::Read,
    path::{Path, PathBuf},
    time::Duration,
};
pub const EXTRACTOR_URL: &str = "https://download.eurotrucksimulator2.com/scs_extractor_1_55.zip";
pub const ARCHIVE_SHA: &str = "45385795fa830b975bd5c80a2076e17fa8398d8d23d679325e3b89b528b0229c";
pub const EXE_SHA: &str = "55bd670691bee62c218220026a0b055bb597eb2c360e33e635c1c4c5c370ea29";
pub const SETUP_VERSION: u32 = 1;
#[derive(Serialize)]
pub struct Detection {
    pub games: Vec<String>,
    pub documents: Vec<String>,
    pub steam_roots: Vec<String>,
    pub notes: Vec<String>,
}
fn add(paths: &mut Vec<String>, p: PathBuf) {
    if p.is_dir() {
        let canonical = p.canonicalize().unwrap_or(p);
        let raw = canonical.to_string_lossy();
        let s = if let Some(unc) = raw.strip_prefix("\\\\?\\UNC\\") {
            format!("//{}", unc.replace('\\', "/"))
        } else {
            raw.strip_prefix("\\\\?\\")
                .unwrap_or(&raw)
                .replace('\\', "/")
        };
        if !paths.iter().any(|x| x.eq_ignore_ascii_case(&s)) {
            paths.push(s);
        }
    }
}
pub fn steam_roots() -> Vec<String> {
    let mut roots = Vec::new();
    #[cfg(windows)]
    {
        use winreg::{enums::HKEY_CURRENT_USER, RegKey};
        if let Ok(k) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam") {
            if let Ok(s) = k.get_value::<String, _>("SteamPath") {
                add(&mut roots, PathBuf::from(s));
            }
        }
    }
    if let Some(p) = std::env::var_os("ProgramFiles(x86)") {
        add(&mut roots, PathBuf::from(p).join("Steam"));
    }
    roots
}
pub fn library_paths(text: &str) -> Vec<PathBuf> {
    let re = regex::Regex::new(r#""path"\s+"((?:\\.|[^"\\])*)""#).unwrap();
    re.captures_iter(text)
        .map(|c| PathBuf::from(c[1].replace("\\\\", "\\")))
        .collect()
}
pub fn detect() -> Detection {
    let steam = steam_roots();
    let mut games = Vec::new();
    let mut documents = Vec::new();
    if let Some(u) = directories::UserDirs::new() {
        if let Some(p) = u.document_dir() {
            add(&mut documents, p.join("Euro Truck Simulator 2"));
        }
    }
    if let Some(home) = std::env::var_os("USERPROFILE") {
        add(
            &mut documents,
            PathBuf::from(home).join("Documents/Euro Truck Simulator 2"),
        );
    }
    for key in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
        if let Some(p) = std::env::var_os(key) {
            add(
                &mut documents,
                PathBuf::from(p).join("Documents/Euro Truck Simulator 2"),
            );
        }
    }
    for root in &steam {
        let root = PathBuf::from(root);
        let mut libraries = vec![root.clone()];
        if let Ok(vdf) = std::fs::read_to_string(root.join("steamapps/libraryfolders.vdf")) {
            libraries.extend(library_paths(&vdf));
        }
        for lib in libraries {
            let manifest = lib.join("steamapps/appmanifest_227300.acf");
            if let Ok(text) = std::fs::read_to_string(manifest) {
                let re = regex::Regex::new(r#""installdir"\s+"([^"]+)""#).unwrap();
                if let Some(c) = re.captures(&text) {
                    let game = lib.join("steamapps/common").join(&c[1]);
                    if game.join("def.scs").is_file() {
                        add(&mut games, game);
                    }
                }
            }
            let game = lib.join("steamapps/common/Euro Truck Simulator 2");
            if game.join("def.scs").is_file() {
                add(&mut games, game);
            }
        }
    }
    for docs in documents.clone() {
        if let Ok(log) = std::fs::read_to_string(Path::new(&docs).join("game.log.txt")) {
            if let Some(line) = log.lines().find(|l| l.contains("[sys] Command line:")) {
                let cmd = line
                    .split("Command line:")
                    .nth(1)
                    .unwrap_or("")
                    .trim()
                    .replace('\\', "/");
                if let Some((prefix, _)) = cmd.split_once("/bin/") {
                    let p = PathBuf::from(prefix.trim().trim_start_matches('"'));
                    if p.join("def.scs").is_file() {
                        add(&mut games, p);
                    }
                }
                let re = regex::Regex::new(r#"(?i)-homedir\s+(?:"([^"]+)"|(\S+))"#).unwrap();
                if let Some(c) = re.captures(&cmd) {
                    add(
                        &mut documents,
                        PathBuf::from(c.get(1).or_else(|| c.get(2)).unwrap().as_str()),
                    );
                }
            }
        }
    }
    Detection {
        games,
        documents,
        steam_roots: steam,
        notes: vec![
            "路径只用于读取本机游戏和存档；确认后才保存设置。".into(),
            "首次建立配件目录会从 SCS 官方下载约 0.3 MB 解包工具，需要联网。".into(),
            "仅支持原版与官方 DLC。检测到 Mod 依赖的存档将拒绝打开。".into(),
        ],
    }
}
pub fn managed_extractor() -> PathBuf {
    app_dir().join("tools/scs-extractor-1.55/scs_extractor.exe")
}
fn verified_exe(path: &Path) -> bool {
    std::fs::read(path)
        .map(|b| hash(&b) == EXE_SHA)
        .unwrap_or(false)
}
pub fn unpack_verified(bytes: &[u8]) -> Result<Vec<u8>> {
    if hash(bytes) != ARCHIVE_SHA {
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
    if hash(&out) != EXE_SHA {
        return Err("解包工具校验失败".into());
    }
    Ok(out)
}
pub fn ensure_extractor(progress: &dyn Fn(&str)) -> Result<PathBuf> {
    let target = managed_extractor();
    if verified_exe(&target) {
        progress("解包工具已就绪");
        return Ok(target);
    }
    progress("正在从 SCS 官方下载解包工具…");
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
        .map_err(|e| e.to_string())?;
    let exe = unpack_verified(&bytes)?;
    std::fs::create_dir_all(target.parent().unwrap()).map_err(|e| e.to_string())?;
    let temp = target.with_extension(format!("{}.tmp", uuid::Uuid::new_v4().simple()));
    std::fs::write(&temp, exe).map_err(|e| e.to_string())?;
    // Remove only our corrupted managed binary, after the replacement has passed both hashes.
    if target.exists() {
        std::fs::remove_file(&target).map_err(|e| e.to_string())?;
    }
    std::fs::rename(&temp, &target).map_err(|e| e.to_string())?;
    progress("官方解包工具已下载并通过校验");
    Ok(target)
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
    Ok(())
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
