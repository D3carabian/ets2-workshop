//! Read-only discovery. Tests inject every host-dependent location explicitly.
use crate::{
    decoder,
    sii::{unquote, Document},
    storage::SaveEntry,
    Result,
};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    io::{self, Read},
    path::{Path, PathBuf},
};

#[derive(Default, Debug, Clone)]
pub struct DiscoveryInputs {
    pub steam_roots: Vec<PathBuf>,
    /// Known Documents folders, including redirected folders.
    pub document_dirs: Vec<PathBuf>,
    pub user_home: Option<PathBuf>,
    pub onedrive_roots: Vec<PathBuf>,
    /// Exact ETS2 user-data directories, not their parent Documents folders.
    pub custom_user_dirs: Vec<PathBuf>,
}
impl DiscoveryInputs {
    pub fn from_host() -> Self {
        let mut inputs = Self::default();
        #[cfg(windows)]
        {
            use winreg::{enums::HKEY_CURRENT_USER, RegKey};
            if let Ok(key) = RegKey::predef(HKEY_CURRENT_USER).open_subkey("Software\\Valve\\Steam")
            {
                if let Ok(path) = key.get_value::<String, _>("SteamPath") {
                    inputs.steam_roots.push(path.into());
                }
            }
        }
        if let Some(path) = std::env::var_os("ProgramFiles(x86)") {
            inputs.steam_roots.push(PathBuf::from(path).join("Steam"));
        }
        if let Some(user) = directories::UserDirs::new() {
            if let Some(path) = user.document_dir() {
                inputs.document_dirs.push(path.into());
            }
        }
        inputs.user_home = std::env::var_os("USERPROFILE").map(PathBuf::from);
        for key in ["OneDrive", "OneDriveConsumer", "OneDriveCommercial"] {
            if let Some(path) = std::env::var_os(key) {
                inputs.onedrive_roots.push(path.into());
            }
        }
        inputs
    }
}
#[derive(Serialize)]
pub struct Detection {
    pub games: Vec<String>,
    pub documents: Vec<String>,
    pub steam_roots: Vec<String>,
    pub notes: Vec<String>,
}
fn display(path: &Path) -> String {
    let raw = path.to_string_lossy();
    if let Some(unc) = raw.strip_prefix("\\\\?\\UNC\\") {
        format!("//{}", unc.replace('\\', "/"))
    } else {
        raw.strip_prefix("\\\\?\\")
            .unwrap_or(&raw)
            .replace('\\', "/")
    }
}
fn identity(path: &Path) -> String {
    let path = display(path);
    if cfg!(windows) {
        path.to_lowercase()
    } else {
        path
    }
}
fn failure(path: &Path, error: impl std::fmt::Display) -> String {
    format!(
        "无法读取 {}：{error}。请检查目录和读取权限后重试，或选择其他目录。",
        display(path)
    )
}
fn optional_metadata(path: &Path) -> Result<Option<fs::Metadata>> {
    match fs::metadata(path) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(failure(path, e)),
    }
}
fn optional_text(path: &Path) -> Result<Option<String>> {
    match fs::read_to_string(path) {
        Ok(value) => Ok(Some(value)),
        Err(e) if e.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(failure(path, e)),
    }
}
fn directories(path: &Path) -> Result<Vec<PathBuf>> {
    let Some(metadata) = optional_metadata(path)? else {
        return Ok(vec![]);
    };
    if !metadata.is_dir() {
        return Err(failure(path, "该路径不是文件夹"));
    }
    let mut paths = Vec::new();
    for entry in fs::read_dir(path).map_err(|e| failure(path, e))? {
        let entry = entry.map_err(|e| failure(path, e))?;
        if entry
            .metadata()
            .map_err(|e| failure(&entry.path(), e))?
            .is_dir()
        {
            paths.push(entry.path());
        }
    }
    paths.sort();
    Ok(paths)
}
fn add(paths: &mut Vec<String>, path: &Path) -> Result<()> {
    let Some(metadata) = optional_metadata(path)? else {
        return Ok(());
    };
    if !metadata.is_dir() {
        return Err(failure(path, "该路径不是文件夹"));
    }
    let canonical = fs::canonicalize(path).map_err(|e| failure(path, e))?;
    let value = display(&canonical);
    if !paths
        .iter()
        .any(|p| identity(Path::new(p)) == identity(&canonical))
    {
        paths.push(value);
    }
    Ok(())
}
fn note(notes: &mut Vec<String>, result: Result<()>) {
    if let Err(error) = result {
        if !notes.contains(&error) {
            notes.push(error);
        }
    }
}

#[derive(Debug)]
enum Value {
    Text(String),
    Object(Vec<(String, Value)>),
}
// Valve KeyValues supports quoted/unquoted tokens, escaped quotes and // comments.
fn keyvalues(text: &str) -> Vec<(String, Value)> {
    let mut chars = text.chars().peekable();
    let mut tokens = Vec::new();
    while let Some(c) = chars.next() {
        if c.is_whitespace() || c == '\u{feff}' {
            continue;
        }
        if c == '/' && chars.peek() == Some(&'/') {
            for c in chars.by_ref() {
                if c == '\n' {
                    break;
                }
            }
            continue;
        }
        if c == '{' || c == '}' {
            tokens.push(c.to_string());
            continue;
        }
        let mut token = String::new();
        if c == '"' {
            while let Some(c) = chars.next() {
                if c == '"' {
                    break;
                }
                if c == '\\' && matches!(chars.peek(), Some('"' | '\\')) {
                    token.push(chars.next().unwrap());
                } else {
                    token.push(c);
                }
            }
        } else {
            token.push(c);
            while chars
                .peek()
                .is_some_and(|c| !c.is_whitespace() && *c != '{' && *c != '}')
            {
                token.push(chars.next().unwrap());
            }
        }
        tokens.push(token);
    }
    fn object(tokens: &[String], at: &mut usize, depth: usize) -> Vec<(String, Value)> {
        let mut out = vec![];
        while *at + 1 < tokens.len() && tokens[*at] != "}" {
            let key = tokens[*at].clone();
            *at += 1;
            let value = if tokens[*at] == "{" {
                *at += 1;
                // Bound recursion for malformed/untrusted Steam configuration.
                if depth >= 32 {
                    break;
                }
                Value::Object(object(tokens, at, depth + 1))
            } else {
                let value = Value::Text(tokens[*at].clone());
                *at += 1;
                value
            };
            out.push((key, value));
        }
        if tokens.get(*at).is_some_and(|t| t == "}") {
            *at += 1;
        }
        out
    }
    object(&tokens, &mut 0, 0)
}
fn text_value<'a>(values: &'a [(String, Value)], key: &str) -> Option<&'a str> {
    values.iter().find_map(|(k, v)| match v {
        Value::Text(t) if k.eq_ignore_ascii_case(key) => Some(t.as_str()),
        _ => None,
    })
}
pub fn library_paths(text: &str) -> Vec<PathBuf> {
    let parsed = keyvalues(text);
    let values = parsed
        .iter()
        .find_map(|(k, v)| match v {
            Value::Object(v) if k.eq_ignore_ascii_case("libraryfolders") => Some(v),
            _ => None,
        })
        .unwrap_or(&parsed);
    values
        .iter()
        .filter(|(k, _)| k.parse::<u32>().is_ok())
        .filter_map(|(_, v)| match v {
            Value::Text(path) => Some(PathBuf::from(path)),
            Value::Object(v) => text_value(v, "path").map(PathBuf::from),
        })
        .collect()
}
fn visit_launch_options(values: &[(String, Value)], out: &mut Vec<String>) {
    for (key, value) in values {
        if let Value::Object(children) = value {
            if key == "227300" {
                if let Some(options) = text_value(children, "LaunchOptions") {
                    out.push(options.into());
                }
            }
            visit_launch_options(children, out);
        }
    }
}
fn command_home(command: &str) -> Option<PathBuf> {
    let re = regex::Regex::new(r#"(?i)(?:^|\s)-homedir(?:\s+|=)(?:"([^"]+)"|(\S+))"#).unwrap();
    re.captures(command)
        .map(|c| PathBuf::from(c.get(1).or_else(|| c.get(2)).unwrap().as_str()))
}
pub fn detect(inputs: &DiscoveryInputs) -> Detection {
    let mut result = Detection {
        games: vec![],
        documents: vec![],
        steam_roots: vec![],
        notes: vec![],
    };
    for path in &inputs.steam_roots {
        note(&mut result.notes, add(&mut result.steam_roots, path));
    }
    let mut documents = inputs.document_dirs.clone();
    if let Some(home) = &inputs.user_home {
        documents.push(home.join("Documents"));
    }
    documents.extend(inputs.onedrive_roots.iter().map(|p| p.join("Documents")));
    for path in documents {
        note(
            &mut result.notes,
            add(&mut result.documents, &path.join("Euro Truck Simulator 2")),
        );
    }
    for path in &inputs.custom_user_dirs {
        note(&mut result.notes, add(&mut result.documents, path));
    }
    for steam in result.steam_roots.clone() {
        let root = Path::new(&steam);
        let mut libraries = vec![root.to_path_buf()];
        match optional_text(&root.join("steamapps/libraryfolders.vdf")) {
            Ok(Some(text)) => libraries.extend(library_paths(&text)),
            Err(e) => result.notes.push(e),
            _ => (),
        }
        for library in libraries {
            let mut candidates = vec![library.join("steamapps/common/Euro Truck Simulator 2")];
            match optional_text(&library.join("steamapps/appmanifest_227300.acf")) {
                Ok(Some(text)) => {
                    let parsed = keyvalues(&text);
                    let values = parsed.iter().find_map(|(_, v)| {
                        if let Value::Object(v) = v {
                            Some(v)
                        } else {
                            None
                        }
                    });
                    if let Some(name) = values.and_then(|v| text_value(v, "installdir")) {
                        candidates.push(library.join("steamapps/common").join(name));
                    }
                }
                Err(e) => result.notes.push(e),
                _ => (),
            }
            for game in candidates {
                note(&mut result.notes, add_game(&mut result.games, &game));
            }
        }
        match directories(&root.join("userdata")) {
            Ok(users) => {
                for user in users {
                    let remote = user.join("227300/remote");
                    match optional_metadata(&remote.join("profiles")) {
                        Ok(Some(m)) if m.is_dir() => {
                            note(&mut result.notes, add(&mut result.documents, &remote))
                        }
                        Err(e) => result.notes.push(e),
                        _ => (),
                    }
                    match optional_text(&user.join("config/localconfig.vdf")) {
                        Ok(Some(text)) => {
                            let mut options = vec![];
                            visit_launch_options(&keyvalues(&text), &mut options);
                            for command in options {
                                if let Some(home) = command_home(&command) {
                                    note(&mut result.notes, add(&mut result.documents, &home));
                                }
                            }
                        }
                        Err(e) => result.notes.push(e),
                        _ => (),
                    }
                }
            }
            Err(e) => result.notes.push(e),
        }
    }
    // Process newly discovered homes too, with canonical deduplication bounding cycles.
    let mut index = 0;
    while index < result.documents.len() {
        let path = PathBuf::from(&result.documents[index]).join("game.log.txt");
        index += 1;
        match optional_text(&path) {
            Ok(Some(log)) => {
                if let Some(line) = log.lines().find(|l| l.contains("[sys] Command line:")) {
                    let command = line.split_once("Command line:").unwrap().1.trim();
                    if let Some(home) = command_home(command) {
                        note(&mut result.notes, add(&mut result.documents, &home));
                    }
                    let normalized = command.replace('\\', "/");
                    if let Some((game, _)) = normalized.split_once("/bin/") {
                        note(
                            &mut result.notes,
                            add_game(&mut result.games, Path::new(game.trim_start_matches('"'))),
                        );
                    }
                }
            }
            Err(e) => result.notes.push(e),
            _ => (),
        }
    }
    result
}
fn add_game(games: &mut Vec<String>, path: &Path) -> Result<()> {
    if optional_metadata(&path.join("def.scs"))?.is_some_and(|m| m.is_file()) {
        add(games, path)?;
    }
    Ok(())
}

/// Preflight only the selected user-data tree before committing first-run settings.
/// This enumerates directories without decoding or reading any save contents.
pub fn validate_documents(documents: &Path) -> Result<()> {
    if !optional_metadata(documents)?.is_some_and(|m| m.is_dir()) {
        return Err(failure(documents, "存档目录不存在或不是文件夹"));
    }
    directories(documents)?;
    for name in ["profiles", "steam_profiles"] {
        for profile in directories(&documents.join(name))? {
            directories(&profile)?;
            for slot in directories(&profile.join("save"))? {
                // Interrupted Workshop staging directories are private, never selectable saves.
                if slot
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .starts_with(".workshop-")
                {
                    continue;
                }
                directories(&slot)?;
            }
        }
    }
    Ok(())
}

/// Successful names are cached; actual info bytes are checked on every scan.
#[derive(Default)]
pub struct SaveListCache {
    names: HashMap<String, CachedSaveName>,
}
struct CachedSaveName {
    digest: String,
    name: String,
    is_autosave: bool,
}
fn info_bytes(path: &Path) -> Result<Vec<u8>> {
    const LIMIT: u64 = 256 * 1024 * 1024;
    let mut bytes = Vec::new();
    fs::File::open(path)
        .map_err(|e| e.to_string())?
        .take(LIMIT + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() as u64 > LIMIT {
        return Err("存档信息超过读取上限".into());
    }
    Ok(bytes)
}
pub fn is_autosave(slot: &str) -> bool {
    let slot = slot.to_ascii_lowercase();
    slot == "autosave" || slot.starts_with("autosave_")
}
pub fn discover(documents: &Path, inputs: &DiscoveryInputs) -> Result<Vec<SaveEntry>> {
    SaveListCache::default().discover(documents, inputs, true)
}
impl SaveListCache {
    pub fn discover(
        &mut self,
        documents: &Path,
        inputs: &DiscoveryInputs,
        include_autosaves: bool,
    ) -> Result<Vec<SaveEntry>> {
        self.discover_with_decoder(documents, inputs, include_autosaves, &|path, bytes| {
            if bytes.starts_with(b"SiiNunit") {
                decoder::decode_direct(bytes)
            } else {
                decoder::read(path)
            }
        })
    }
    fn discover_with_decoder(
        &mut self,
        documents: &Path,
        inputs: &DiscoveryInputs,
        include_autosaves: bool,
        decode: &dyn Fn(&Path, &[u8]) -> Result<String>,
    ) -> Result<Vec<SaveEntry>> {
        if !optional_metadata(documents)?.is_some_and(|m| m.is_dir()) {
            return Err(failure(documents, "存档目录不存在或不是文件夹"));
        }
        let mut roots = vec![documents.join("profiles"), documents.join("steam_profiles")];
        for steam in &inputs.steam_roots {
            for user in directories(&steam.join("userdata"))? {
                roots.push(user.join("227300/remote/profiles"));
            }
        }
        let mut seen_roots = HashSet::new();
        let mut seen_saves = HashSet::new();
        let mut out = Vec::new();
        for root in roots {
            if optional_metadata(&root)?.is_none() {
                continue;
            }
            let canonical = fs::canonicalize(&root).map_err(|e| failure(&root, e))?;
            if !seen_roots.insert(identity(&canonical)) {
                continue;
            }
            for profile in directories(&root)? {
                for slot in directories(&profile.join("save"))? {
                    // Interrupted Workshop staging directories are private, never selectable saves.
                    if slot
                        .file_name()
                        .unwrap()
                        .to_string_lossy()
                        .starts_with(".workshop-")
                    {
                        continue;
                    }
                    let slot_name = slot.file_name().unwrap().to_string_lossy();
                    let is_autosave = is_autosave(&slot_name);
                    if is_autosave && !include_autosaves {
                        continue;
                    }
                    let path = slot.join("game.sii");
                    if !optional_metadata(&path)?.is_some_and(|m| m.is_file()) {
                        continue;
                    }
                    let path = fs::canonicalize(&path).map_err(|e| failure(&path, e))?;
                    if !seen_saves.insert(identity(&path)) {
                        continue;
                    }
                    let info = slot.join("info.sii");
                    let metadata = fs::metadata(&path).map_err(|e| failure(&path, e))?;
                    let modified = metadata
                        .modified()
                        .map_err(|e| failure(&path, e))?
                        .duration_since(std::time::UNIX_EPOCH)
                        .map(|d| d.as_secs())
                        .unwrap_or(0);
                    let key = identity(&path);
                    let result = (|| -> Result<String> {
                        let bytes = info_bytes(&info)?;
                        let digest = crate::hash(&bytes);
                        if let Some(previous) = self.names.get(&key) {
                            if previous.digest == digest {
                                return Ok(previous.name.clone());
                            }
                        }
                        let doc = Document::parse(decode(&info, &bytes)?)?;
                        let name = doc
                            .units
                            .first()
                            .and_then(|u| u.get("name"))
                            .map(unquote)
                            .filter(|s| !s.is_empty())
                            .unwrap_or_else(|| slot_name.to_string());
                        if crate::hash(&info_bytes(&info)?) != digest {
                            return Err("读取时存档信息发生变化，请刷新重试".into());
                        }
                        self.names.insert(
                            key.clone(),
                            CachedSaveName {
                                digest,
                                name: name.clone(),
                                is_autosave,
                            },
                        );
                        Ok(name)
                    })();
                    let (name, error) = match result {
                        Ok(name) => (name, None),
                        Err(error) => {
                            self.names.remove(&key);
                            ("无法读取名称".into(), Some(failure(&info, error)))
                        }
                    };
                    out.push(SaveEntry {
                        path: display(&path),
                        name,
                        profile: profile.file_name().unwrap().to_string_lossy().into(),
                        modified,
                        is_autosave,
                        error,
                    });
                }
            }
        }
        // A display filter must not discard names that were already decoded.
        // Hidden autosaves are checked for changes/deletion when shown again.
        self.names.retain(|key, cached| {
            seen_saves.contains(key) || (!include_autosaves && cached.is_autosave)
        });
        out.sort_by(|a, b| {
            b.modified
                .cmp(&a.modified)
                .then_with(|| a.path.cmp(&b.path))
        });
        Ok(out)
    }
}

#[cfg(test)]
mod cache_tests {
    use super::*;
    use std::cell::Cell;
    fn save(root: &Path, slot: &str, name: &str) -> PathBuf {
        let dir = root.join("profiles/test/save").join(slot);
        fs::create_dir_all(&dir).unwrap();
        fs::write(dir.join("game.sii"), "SiiNunit {\n}\n").unwrap();
        fs::write(
            dir.join("info.sii"),
            format!("SiiNunit {{\nsave_container : test {{\n name: \"{name}\"\n}}\n}}\n"),
        )
        .unwrap();
        dir
    }
    #[test]
    fn filtering_happens_before_decode_and_keeps_quicksaves() {
        let root = tempfile::tempdir().unwrap();
        save(root.path(), "autosave", "auto");
        save(root.path(), "AUTOSAVE_JOB_1", "auto");
        save(root.path(), "quicksave", "quick");
        save(root.path(), "1", "autosave road trip");
        let calls = Cell::new(0);
        let decode = |_: &Path, bytes: &[u8]| {
            calls.set(calls.get() + 1);
            decoder::decode_direct(bytes)
        };
        let mut cache = SaveListCache::default();
        let inputs = DiscoveryInputs::default();
        let list = cache
            .discover_with_decoder(root.path(), &inputs, false, &decode)
            .unwrap();
        assert_eq!(list.len(), 2);
        assert_eq!(calls.get(), 2);
        assert!(list.iter().all(|s| !s.is_autosave));
        let list = cache
            .discover_with_decoder(root.path(), &inputs, true, &decode)
            .unwrap();
        assert_eq!(list.len(), 4);
        assert_eq!(calls.get(), 4);
        cache
            .discover_with_decoder(root.path(), &inputs, true, &decode)
            .unwrap();
        assert_eq!(calls.get(), 4);
    }
    #[test]
    fn hidden_autosaves_stay_warm_but_are_revalidated_when_shown() {
        let root = tempfile::tempdir().unwrap();
        let slot = save(root.path(), "autosave", "first");
        let calls = Cell::new(0);
        let decode = |_: &Path, bytes: &[u8]| {
            calls.set(calls.get() + 1);
            decoder::decode_direct(bytes)
        };
        let inputs = DiscoveryInputs::default();
        let mut cache = SaveListCache::default();
        for include in [true, true, false, true] {
            let list = cache
                .discover_with_decoder(root.path(), &inputs, include, &decode)
                .unwrap();
            assert_eq!(list.len(), usize::from(include));
        }
        assert_eq!(calls.get(), 1);
        cache
            .discover_with_decoder(root.path(), &inputs, false, &decode)
            .unwrap();
        save(root.path(), "autosave", "other");
        let list = cache
            .discover_with_decoder(root.path(), &inputs, true, &decode)
            .unwrap();
        assert_eq!(list[0].name, "other");
        assert_eq!(calls.get(), 2);
        cache
            .discover_with_decoder(root.path(), &inputs, false, &decode)
            .unwrap();
        fs::remove_file(slot.join("game.sii")).unwrap();
        assert!(cache
            .discover_with_decoder(root.path(), &inputs, true, &decode)
            .unwrap()
            .is_empty());
        assert!(cache.names.is_empty());
    }

    #[test]
    fn cache_detects_same_timestamp_rewrites_missing_files_and_new_saves() {
        let root = tempfile::tempdir().unwrap();
        let dir = save(root.path(), "1", "first");
        let calls = Cell::new(0);
        let decode = |_: &Path, bytes: &[u8]| {
            calls.set(calls.get() + 1);
            decoder::decode_direct(bytes)
        };
        let inputs = DiscoveryInputs::default();
        let mut cache = SaveListCache::default();
        cache
            .discover_with_decoder(root.path(), &inputs, true, &decode)
            .unwrap();
        let modified = fs::metadata(dir.join("info.sii"))
            .unwrap()
            .modified()
            .unwrap();
        save(root.path(), "1", "other");
        fs::File::options()
            .write(true)
            .open(dir.join("info.sii"))
            .unwrap()
            .set_times(fs::FileTimes::new().set_modified(modified))
            .unwrap();
        let list = cache
            .discover_with_decoder(root.path(), &inputs, true, &decode)
            .unwrap();
        assert_eq!(list[0].name, "other");
        assert_eq!(calls.get(), 2);
        fs::remove_file(dir.join("info.sii")).unwrap();
        assert!(cache.discover(root.path(), &inputs, true).unwrap()[0]
            .error
            .is_some());
        save(root.path(), "1", "back");
        save(root.path(), "2", "new");
        assert_eq!(cache.discover(root.path(), &inputs, true).unwrap().len(), 2);
        fs::remove_file(dir.join("game.sii")).unwrap();
        assert_eq!(cache.discover(root.path(), &inputs, true).unwrap().len(), 1);
        assert_eq!(cache.names.len(), 1);
    }
    #[test]
    fn racing_decode_is_not_cached() {
        let root = tempfile::tempdir().unwrap();
        save(root.path(), "1", "first");
        let decode = |path: &Path, bytes: &[u8]| {
            fs::write(path, "changed").unwrap();
            decoder::decode_direct(bytes)
        };
        let mut cache = SaveListCache::default();
        let list = cache
            .discover_with_decoder(root.path(), &DiscoveryInputs::default(), true, &decode)
            .unwrap();
        assert!(list[0].error.is_some());
        assert!(cache.names.is_empty());
    }
    #[cfg(windows)]
    #[test]
    fn locked_info_is_not_hidden_by_warm_cache() {
        use std::os::windows::fs::OpenOptionsExt;
        let root = tempfile::tempdir().unwrap();
        let dir = save(root.path(), "1", "first");
        let mut cache = SaveListCache::default();
        let inputs = DiscoveryInputs::default();
        cache.discover(root.path(), &inputs, true).unwrap();
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .open(dir.join("info.sii"))
            .unwrap();
        assert!(cache.discover(root.path(), &inputs, true).unwrap()[0]
            .error
            .is_some());
        drop(lock);
        assert_eq!(
            cache.discover(root.path(), &inputs, true).unwrap()[0].name,
            "first"
        );
    }
}
