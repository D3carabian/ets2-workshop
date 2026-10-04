use std::{
    fs,
    path::{Path, PathBuf},
};
use workshop_core::{
    discovery::{self, DiscoveryInputs, SaveListCache},
    setup,
    storage::{self, Settings},
};

fn sandbox() -> tempfile::TempDir {
    let root = Path::new(env!("CARGO_MANIFEST_DIR")).join("target/discovery-tests");
    fs::create_dir_all(&root).unwrap();
    tempfile::tempdir_in(root).unwrap()
}
fn file(root: &Path, relative: &str, text: &str) -> PathBuf {
    let path = root.join(relative);
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(&path, text).unwrap();
    path
}
fn vdf_path(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "\\\\")
}
fn canonical(path: &Path) -> String {
    path.canonicalize()
        .unwrap()
        .to_string_lossy()
        .trim_start_matches("\\\\?\\")
        .replace('\\', "/")
}
fn save(root: &Path, profile: &str, slot: &str, name: &str) {
    file(
        root,
        &format!("{profile}/save/{slot}/game.sii"),
        "SiiNunit {\n}\n",
    );
    file(
        root,
        &format!("{profile}/save/{slot}/info.sii"),
        &format!("SiiNunit {{\nsave_container : test {{\n name: \"{name}\"\n}}\n}}\n"),
    );
}
fn settings(documents: &Path) -> Settings {
    Settings {
        documents: documents.to_string_lossy().into(),
        game: String::new(),
        extractor: String::new(),
        onboarding_version: 0,
        catalog_file: String::new(),
    }
}

#[test]
fn modern_and_legacy_library_formats_ignore_comments_and_metadata() {
    let parsed = setup::library_paths(
        r#"
        // "path" "do not detect this"
        "libraryfolders" {
            "TimeNextStatsReport" "123456"
            "0" { "path" "C:\\Steam" "apps" { "227300" "12345" } }
            "1" "D:\\旧版 库"
            "2" { "PATH" "E:\\新版 库" }
        }
    "#,
    );
    assert_eq!(
        parsed,
        vec![
            PathBuf::from(r"C:\Steam"),
            PathBuf::from(r"D:\旧版 库"),
            PathBuf::from(r"E:\新版 库")
        ]
    );
}

#[test]
fn detects_multiple_libraries_profiles_cloud_and_document_sources() {
    let temp = sandbox();
    let root = temp.path();
    let steam = root.join("Steam");
    let legacy = root.join("旧版 Steam 库");
    let modern = root.join("现代 Steam 库");
    file(
        &steam,
        "steamapps/libraryfolders.vdf",
        &format!(
            r#""libraryfolders" {{ "0" {{ "path" "{}" }} "1" "{}" "2" {{ "path" "{}" }} }}"#,
            vdf_path(&steam),
            vdf_path(&legacy),
            vdf_path(&modern)
        ),
    );
    file(
        &steam,
        "steamapps/common/Euro Truck Simulator 2/def.scs",
        "synthetic",
    );
    file(
        &legacy,
        "steamapps/common/Euro Truck Simulator 2/def.scs",
        "synthetic",
    );
    file(
        &modern,
        "steamapps/common/自定义安装名/def.scs",
        "synthetic",
    );
    file(
        &modern,
        "steamapps/appmanifest_227300.acf",
        r#""AppState" { "installdir" "自定义安装名" }"#,
    );
    let known_docs = root.join("重定向 文档");
    let home = root.join("本地 用户");
    let onedrive = root.join("OneDrive 企业");
    let local = known_docs.join("Euro Truck Simulator 2");
    save(&local.join("profiles"), "profile-a", "1", "中文名称");
    save(&local.join("profiles"), "profile-b", "autosave", "自动存档");
    save(
        &local.join("steam_profiles"),
        "profile-c",
        "1",
        "Steam 本地",
    );
    fs::create_dir_all(home.join("Documents/Euro Truck Simulator 2")).unwrap();
    fs::create_dir_all(onedrive.join("Documents/Euro Truck Simulator 2")).unwrap();
    let cloud = steam.join("userdata/1000/227300/remote");
    save(&cloud.join("profiles"), "cloud-a", "1", "云端存档");
    save(
        &steam.join("userdata/2000/227300/remote/profiles"),
        "cloud-b",
        "1",
        "另一个帐号",
    );
    let inputs = DiscoveryInputs {
        steam_roots: vec![steam.clone(), steam.join(".")],
        document_dirs: vec![known_docs],
        user_home: Some(home),
        onedrive_roots: vec![onedrive],
        custom_user_dirs: vec![],
    };
    let found = setup::detect_with(&inputs);
    assert_eq!(found.games.len(), 3);
    assert_eq!(found.documents.len(), 5);
    assert_eq!(found.steam_roots.len(), 1);
    assert!(found
        .games
        .contains(&canonical(&modern.join("steamapps/common/自定义安装名"))));
    assert!(found.documents.contains(&canonical(&cloud)));
    let saves = storage::discover_with(&settings(&local), &inputs).unwrap();
    assert_eq!(saves.len(), 5);
    assert!(saves
        .iter()
        .any(|s| s.name == "中文名称" && s.profile == "profile-a"));
    assert!(saves.iter().all(|s| s.error.is_none()));
    // Selecting the cloud directory must not list that same save twice.
    let saves = storage::discover_with(&settings(&cloud), &inputs).unwrap();
    assert_eq!(saves.len(), 2);
}

#[test]
fn finds_custom_homes_from_game_log_and_ets2_launch_options_only() {
    let temp = sandbox();
    let root = temp.path();
    let documents = root.join("文档/Euro Truck Simulator 2");
    let custom = root.join("自定义 用户目录");
    let steam_home = root.join("Steam 用户目录");
    let unrelated = root.join("其他游戏目录");
    for dir in [&custom, &steam_home, &unrelated] {
        fs::create_dir_all(dir).unwrap();
    }
    let game = root.join("游戏 安装");
    file(&game, "def.scs", "synthetic");
    file(
        &documents,
        "game.log.txt",
        &format!(
            "00:00:00.000 : [sys] Command line: \"{}/bin/win_x64/eurotrucks2.exe\" -homedir \"{}\"",
            game.display(),
            custom.display()
        ),
    );
    // A cycle in logs terminates because candidates are canonicalized before adding.
    file(
        &custom,
        "game.log.txt",
        &format!(
            "[sys] Command line: ets2.exe -homedir=\"{}\"",
            documents.display()
        ),
    );
    let steam = root.join("Steam");
    file(
        &steam,
        "userdata/1/config/localconfig.vdf",
        &format!(
            r#""UserLocalConfigStore" {{ "Software" {{ "Valve" {{ "Steam" {{ "apps" {{ "227300" {{ "LaunchOptions" "-homedir \"{}\"" }} "123" {{ "LaunchOptions" "-homedir \"{}\"" }} }} }} }} }} }}"#,
            vdf_path(&steam_home),
            vdf_path(&unrelated)
        ),
    );
    let found = discovery::detect(&DiscoveryInputs {
        steam_roots: vec![steam],
        document_dirs: vec![root.join("文档")],
        ..Default::default()
    });
    assert_eq!(found.documents.len(), 3);
    assert!(found.documents.contains(&canonical(&custom)));
    assert!(found.documents.contains(&canonical(&steam_home)));
    assert!(!found.documents.contains(&canonical(&unrelated)));
    assert_eq!(found.games, vec![canonical(&game)]);
}

#[test]
fn empty_inputs_are_isolated_and_missing_optional_roots_are_normal() {
    let found = discovery::detect(&DiscoveryInputs::default());
    assert!(found.games.is_empty() && found.documents.is_empty() && found.steam_roots.is_empty());
    let temp = sandbox();
    assert!(
        discovery::discover(temp.path(), &DiscoveryInputs::default())
            .unwrap()
            .is_empty()
    );
    let error = discovery::discover(&temp.path().join("missing"), &DiscoveryInputs::default())
        .err()
        .unwrap();
    assert!(error.contains("重试"));
}

#[test]
fn invalid_roots_and_unreadable_configuration_report_actionable_errors() {
    let temp = sandbox();
    file(temp.path(), "profiles", "not a directory");
    save(
        &temp.path().join("steam_profiles"),
        "usable",
        "1",
        "available",
    );
    let mut cache = SaveListCache::default();
    let saves = cache
        .discover(temp.path(), &DiscoveryInputs::default(), true)
        .unwrap();
    assert_eq!(saves.len(), 1);
    assert!(cache
        .warnings()
        .iter()
        .any(|error| error.contains("profiles") && error.contains("读取权限后重试")));
    fs::remove_file(temp.path().join("profiles")).unwrap();
    save(&temp.path().join("profiles"), "repaired", "1", "recovered");
    assert_eq!(
        cache
            .discover(temp.path(), &DiscoveryInputs::default(), true)
            .unwrap()
            .len(),
        2
    );
    assert!(cache.warnings().is_empty());
    let steam = temp.path().join("Steam");
    file(
        &steam,
        "steamapps/libraryfolders.vdf/child",
        "directory instead of file",
    );
    let found = discovery::detect(&DiscoveryInputs {
        steam_roots: vec![steam],
        ..Default::default()
    });
    assert!(found
        .notes
        .iter()
        .any(|n| n.contains("libraryfolders.vdf") && n.contains("重试")));
}

#[test]
fn broken_or_missing_info_does_not_hide_save_and_duplicate_aliases_are_removed() {
    let temp = sandbox();
    let remote = temp.path().join("Steam/userdata/1/227300/remote");
    save(&remote.join("profiles"), "profile", "1", "valid");
    file(&remote, "profiles/profile/save/2/game.sii", "synthetic");
    file(&remote, "profiles/profile/save/3/game.sii", "synthetic");
    file(&remote, "profiles/profile/save/3/info.sii", "broken");
    let inputs = DiscoveryInputs {
        steam_roots: vec![temp.path().join("Steam"), temp.path().join("Steam/.")],
        ..Default::default()
    };
    let saves = discovery::discover(&remote, &inputs).unwrap();
    assert_eq!(saves.len(), 3);
    assert_eq!(saves.iter().filter(|s| s.error.is_some()).count(), 2);
}

#[cfg(windows)]
#[test]
fn locked_info_reports_error_instead_of_hiding_the_slot() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = sandbox();
    save(&temp.path().join("profiles"), "profile", "1", "test");
    let _lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .open(temp.path().join("profiles/profile/save/1/info.sii"))
        .unwrap();
    let saves = discovery::discover(temp.path(), &DiscoveryInputs::default()).unwrap();
    assert_eq!(saves.len(), 1);
    assert!(saves[0].error.as_ref().unwrap().contains("读取权限后重试"));
}

#[cfg(windows)]
#[test]
fn unreadable_profile_directory_does_not_hide_other_saves_and_recovers() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = sandbox();
    let profiles = temp.path().join("profiles");
    save(&profiles, "profile", "1", "test");
    save(
        &temp.path().join("steam_profiles"),
        "other",
        "1",
        "available",
    );
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(0x02000000) // FILE_FLAG_BACKUP_SEMANTICS allows a directory handle.
        .open(&profiles)
        .unwrap();
    let mut cache = SaveListCache::default();
    let inputs = DiscoveryInputs::default();
    let saves = cache.discover(temp.path(), &inputs, true).unwrap();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].name, "available");
    assert!(cache
        .warnings()
        .iter()
        .any(|error| error.contains("profiles") && error.contains("读取权限后重试")));
    drop(lock);
    assert_eq!(cache.discover(temp.path(), &inputs, true).unwrap().len(), 2);
    assert!(cache.warnings().is_empty());
}

#[test]
fn setup_checks_selected_root_without_rejecting_unusable_profile_branches() {
    let temp = sandbox();
    let docs = temp.path().join("documents");
    let game = temp.path().join("game");
    file(&game, "def.scs", "synthetic");
    file(&game, "bin/win_x64/eurotrucks2.exe", "synthetic");
    let mut selected = settings(&docs);
    selected.game = game.to_string_lossy().into();
    for name in ["profiles", "steam_profiles"] {
        let invalid = file(&docs, name, "not a directory");
        setup::validate(&selected).unwrap();
        let mut cache = SaveListCache::default();
        cache
            .discover(&docs, &DiscoveryInputs::default(), true)
            .unwrap();
        assert!(cache.warnings().iter().any(|error| error.contains(name)));
        fs::remove_file(&invalid).unwrap();
        fs::create_dir(&invalid).unwrap();
        setup::validate(&selected).unwrap();
    }
    // Validation checks enumeration, leaving malformed save contents to save discovery.
    file(&docs, "profiles/profile/save/1/info.sii", "not a save");
    setup::validate(&selected).unwrap();
}

#[cfg(windows)]
#[test]
fn setup_accepts_local_failures_but_rejects_a_locked_selected_root() {
    use std::os::windows::fs::OpenOptionsExt;
    let temp = sandbox();
    let docs = temp.path().join("documents");
    let game = temp.path().join("game");
    file(&game, "def.scs", "synthetic");
    file(&game, "bin/win_x64/eurotrucks2.exe", "synthetic");
    save(&docs.join("profiles"), "profile", "1", "synthetic");
    save(&docs.join("steam_profiles"), "other", "1", "available");
    let mut selected = settings(&docs);
    selected.game = game.to_string_lossy().into();
    for relative in [
        "profiles",
        "profiles/profile",
        "profiles/profile/save",
        "profiles/profile/save/1",
    ] {
        let lock = fs::OpenOptions::new()
            .read(true)
            .share_mode(0)
            .custom_flags(0x02000000)
            .open(docs.join(relative))
            .unwrap();
        setup::validate(&selected).unwrap();
        let mut cache = SaveListCache::default();
        let saves = cache
            .discover(&docs, &DiscoveryInputs::default(), true)
            .unwrap();
        assert_eq!(saves.len(), 1, "locked {relative} hid a usable branch");
        assert_eq!(saves[0].name, "available");
        assert!(
            !cache.warnings().is_empty(),
            "locked {relative} was silently ignored"
        );
        drop(lock);
        setup::validate(&selected).unwrap();
        assert_eq!(
            cache
                .discover(&docs, &DiscoveryInputs::default(), true)
                .unwrap()
                .len(),
            2
        );
        assert!(cache.warnings().is_empty());
    }
    let lock = fs::OpenOptions::new()
        .read(true)
        .share_mode(0)
        .custom_flags(0x02000000)
        .open(&docs)
        .unwrap();
    assert!(setup::validate(&selected).is_err());
    assert!(discovery::discover(&docs, &DiscoveryInputs::default()).is_err());
    drop(lock);
    setup::validate(&selected).unwrap();
}

#[test]
fn broken_automatic_steam_sources_do_not_block_selected_saves() {
    let temp = sandbox();
    let docs = temp.path().join("documents");
    save(&docs.join("profiles"), "local", "1", "available");
    let steam = temp.path().join("Steam");
    let broken = file(&steam, "userdata", "not a directory");
    let inputs = DiscoveryInputs {
        steam_roots: vec![steam.clone(), temp.path().join("missing-steam")],
        ..Default::default()
    };
    let mut cache = SaveListCache::default();
    assert_eq!(cache.discover(&docs, &inputs, true).unwrap().len(), 1);
    assert_eq!(cache.warnings().len(), 1);
    assert!(cache.warnings()[0].contains("userdata"));
    fs::remove_file(broken).unwrap();
    let cloud = steam.join("userdata/1/227300/remote");
    file(&cloud, "profiles", "not a directory");
    assert_eq!(cache.discover(&docs, &inputs, true).unwrap().len(), 1);
    assert_eq!(cache.warnings().len(), 1);
    assert!(cache.warnings()[0].contains("remote/profiles"));
    fs::remove_file(cloud.join("profiles")).unwrap();
    save(&cloud.join("profiles"), "cloud", "1", "recovered");
    assert_eq!(cache.discover(&docs, &inputs, true).unwrap().len(), 2);
    assert!(cache.warnings().is_empty());
}

#[test]
fn unusable_selected_root_still_fails_even_when_automatic_sources_are_valid() {
    let temp = sandbox();
    let steam = temp.path().join("Steam");
    save(
        &steam.join("userdata/1/227300/remote/profiles"),
        "cloud",
        "1",
        "available",
    );
    let inputs = DiscoveryInputs {
        steam_roots: vec![steam],
        ..Default::default()
    };
    let selected_file = file(temp.path(), "selected-file", "not a directory");
    let mut cache = SaveListCache::default();
    for selected in [selected_file, temp.path().join("missing")] {
        assert!(cache.discover(&selected, &inputs, true).is_err());
        assert!(discovery::validate_documents(&selected).is_err());
    }
}

#[test]
fn broken_profile_and_slot_are_local_failures_and_warnings_clear_after_repair() {
    let temp = sandbox();
    let profiles = temp.path().join("profiles");
    save(&profiles, "good", "1", "available");
    let broken_profile = file(&profiles, "broken/save", "not a directory");
    let broken_slot = profiles.join("good/save/2/game.sii");
    fs::create_dir_all(&broken_slot).unwrap();
    let inputs = DiscoveryInputs::default();
    let mut cache = SaveListCache::default();
    let saves = cache.discover(temp.path(), &inputs, true).unwrap();
    assert_eq!(saves.len(), 1);
    assert_eq!(saves[0].name, "available");
    assert_eq!(cache.warnings().len(), 2);
    assert!(cache.warnings().iter().any(|s| s.contains("broken/save")));
    assert!(cache.warnings().iter().any(|s| s.contains("2/game.sii")));
    fs::remove_file(broken_profile).unwrap();
    fs::remove_dir(broken_slot).unwrap();
    save(&profiles, "broken", "1", "recovered profile");
    save(&profiles, "good", "2", "recovered slot");
    assert_eq!(cache.discover(temp.path(), &inputs, true).unwrap().len(), 3);
    assert!(cache.warnings().is_empty());
}
