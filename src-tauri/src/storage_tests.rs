use super::*;
use crate::catalog::Definition;
use std::collections::BTreeMap;

fn fixture(root: &Path) -> (Session, Catalog, Vec<Operation>) {
    std::fs::create_dir_all(root).unwrap();
    let text = "SiiNunit {\nplayer : p {\n trucks: 1\n trucks[0]: t\n}\nvehicle : t {\n accessories: 1\n accessories[0]: eng\n license_plate: \"SYNTHETIC\"\n}\nvehicle_accessory : eng {\n data_path: \"/def/vehicle/truck/test/engine/a.sii\"\n refund: 100\n}\n}\n";
    std::fs::write(root.join("game.sii"), text).unwrap();
    std::fs::write(
        root.join("info.sii"),
        "SiiNunit {\nsave_container : info {\n name: original\n file_time: 1\n}\n}\n",
    )
    .unwrap();
    std::fs::write(root.join("preview.png"), b"synthetic image placeholder").unwrap();
    let mut catalog = Catalog::default();
    for name in ["a", "b"] {
        let path = format!("/def/vehicle/truck/test/engine/{name}.sii");
        catalog.definitions.insert(
            path.clone(),
            Definition {
                raw_name: String::new(),
                names: std::collections::BTreeMap::new(),
                category_names: std::collections::BTreeMap::new(),
                name_alias: None,
                path,
                kind: "accessory_engine_data".into(),
                unit: name.into(),
                name: name.into(),
                category: "engine".into(),
                model: "test".into(),
                source: "fixture".into(),
                metrics: BTreeMap::new(),
                suitable: vec![],
                conflicts: vec![],
                requires: vec![],
            },
        );
    }
    let s = Session::open(&root.join("game.sii"), &Settings::default()).unwrap();
    let ops = vec![Operation {
        truck_id: "t".into(),
        action: "replace".into(),
        accessory_id: Some("eng".into()),
        candidate_path: "/def/vehicle/truck/test/engine/b.sii".into(),
        donor_accessory: None,
    }];
    (s, catalog, ops)
}

// One isolated test owns the process environment; fault hooks never affect production RPC.
#[test]
fn save_recovery_failure_matrix() {
    let temp = tempfile::tempdir().unwrap();
    let previous = std::env::var_os("ETS2_WORKSHOP_DATA_DIR");
    std::env::set_var("ETS2_WORKSHOP_DATA_DIR", temp.path().join("app"));
    struct Reset(Option<std::ffi::OsString>);
    impl Drop for Reset {
        fn drop(&mut self) {
            match &self.0 {
                Some(v) => std::env::set_var("ETS2_WORKSHOP_DATA_DIR", v),
                None => std::env::remove_var("ETS2_WORKSHOP_DATA_DIR"),
            }
        }
    }
    let _reset = Reset(previous);
    for mode in ["new", "overwrite"] {
        for step in ["backup", "record", "publish", "verify", "finalize"] {
            let root = temp.path().join(format!("{mode}-{step}")).join("1");
            let (s, c, ops) = fixture(&root);
            let result = commit_with(&s, &c, &ops, mode, "中文 new slot", &|at, r| {
                if at == "publish" {
                    assert!(cleanup(&r.id).unwrap_err().contains("另一实例"));
                    assert!(restore(&r.id).unwrap_err().contains("另一实例"));
                }
                if at == step {
                    // Write a partial private temporary file to simulate a disk-full interruption.
                    if at == "publish" && mode == "overwrite" {
                        std::fs::write(temp_path(r), b"partial").unwrap();
                    }
                    Err("injected disk full / access denied".into())
                } else {
                    Ok(())
                }
            });
            let written = step == "verify" || step == "finalize";
            if written {
                let r = result.unwrap();
                assert!(r.warning.as_ref().unwrap().contains("已"));
                assert_eq!(digest(Path::new(&r.output)).unwrap(), r.after_hash);
                // Even with receipt finalization unavailable, disk contains enough to recover after restart.
                let pending = receipt(&r.id).unwrap();
                assert_eq!(pending.state, "prepared");
                assert_eq!(pending.info_hash, r.info_hash);
                let info_before =
                    std::fs::read(Path::new(&r.output).with_file_name("info.sii")).unwrap();
                restore(&r.id).unwrap();
                restore(&r.id).unwrap();
                assert_eq!(digest(Path::new(&r.output)).unwrap(), r.before_hash);
                assert_eq!(
                    std::fs::read(Path::new(&r.output).with_file_name("info.sii")).unwrap(),
                    info_before
                );
                assert!(Path::new(&r.backup).join("game.sii").exists());
            } else {
                assert!(result.unwrap_err().contains("本工具未改写原档"));
                assert_eq!(digest(&s.path).unwrap(), s.original_hash);
                assert!(!root.parent().unwrap().join("2").exists());
            }
        }
    }
    // A process interruption leaves a durable journal and an owned staging directory.
    let (s, c, ops) = fixture(&temp.path().join("interrupted/1"));
    let interrupted = std::panic::catch_unwind(|| {
        let _ = commit_with(&s, &c, &ops, "new", "test", &|at, r| {
            if at == "publish" {
                assert!(stage_path(r).exists());
                panic!("simulated process interruption");
            }
            Ok(())
        });
    });
    assert!(interrupted.is_err());
    let pending = receipts()
        .into_iter()
        .find(|r| r.source.contains("interrupted"))
        .unwrap();
    assert_eq!(pending.state, "prepared");
    assert!(stage_path(&pending).exists());
    cleanup(&pending.id).unwrap();
    cleanup(&pending.id).unwrap();
    assert!(!stage_path(&pending).exists());
    assert_eq!(digest(&s.path).unwrap(), s.original_hash);
    // Different history IDs cannot publish or restore the same source concurrently.
    let locked = lock_output(&std::fs::canonicalize(&s.path).unwrap()).unwrap();
    assert!(commit_with(&s, &c, &ops, "overwrite", "", &|_, _| Ok(()))
        .unwrap_err()
        .contains("另一实例"));
    drop(locked);
    #[cfg(windows)]
    {
        let (s, c, ops) = fixture(&temp.path().join("receipt-readonly/1"));
        let r = commit_with(&s, &c, &ops, "overwrite", "", &|at, r| {
            if at == "finalize" {
                let path = history_dir(&r.id).unwrap().join("receipt.json");
                let mut permissions = std::fs::metadata(&path).unwrap().permissions();
                permissions.set_readonly(true);
                std::fs::set_permissions(path, permissions).unwrap();
            }
            Ok(())
        })
        .unwrap();
        assert!(r.warning.unwrap().contains("记录确认失败"));
        assert_eq!(receipt(&r.id).unwrap().state, "prepared");
        restore(&r.id).unwrap();
        assert_eq!(digest(&s.path).unwrap(), s.original_hash);
        let path = history_dir(&r.id).unwrap().join("receipt.json");
        let mut permissions = std::fs::metadata(&path).unwrap().permissions();
        permissions.set_readonly(false);
        std::fs::set_permissions(path, permissions).unwrap();
    }
    // A slot claimed after preview is never merged with our staged directory.
    let (s, c, ops) = fixture(&temp.path().join("occupied/1"));
    let err = commit_with(&s, &c, &ops, "new", "test", &|at, r| {
        if at == "publish" {
            let target = Path::new(&r.output).parent().unwrap();
            std::fs::create_dir(target).unwrap();
            std::fs::write(target.join("game.sii"), "other writer").unwrap();
        }
        Ok(())
    })
    .unwrap_err();
    assert!(err.contains("占用"));
    assert_eq!(
        std::fs::read_to_string(temp.path().join("occupied/2/game.sii")).unwrap(),
        "other writer"
    );
    assert_eq!(digest(&s.path).unwrap(), s.original_hash);

    // Game resave during preparation invalidates both overwrite and save-as.
    for mode in ["overwrite", "new"] {
        let (s, c, ops) = fixture(&temp.path().join(format!("resave-{mode}/1")));
        let err = commit_with(&s, &c, &ops, mode, "test", &|at, _| {
            if at == "publish" {
                std::fs::write(&s.path, "new game progress").unwrap();
            }
            Ok(())
        })
        .unwrap_err();
        assert!(err.contains("更新"));
        assert_eq!(
            std::fs::read_to_string(&s.path).unwrap(),
            "new game progress"
        );
    }
    // Real Windows sharing violations cover replacement failures, beyond injected errors.
    #[cfg(windows)]
    {
        use std::os::windows::fs::OpenOptionsExt;
        let (s, c, ops) = fixture(&temp.path().join("locked/1"));
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&s.path)
            .unwrap();
        let err = commit_with(&s, &c, &ops, "overwrite", "", &|_, _| Ok(())).unwrap_err();
        assert!(err.contains("本工具未改写原档"));
        assert_eq!(digest(&s.path).unwrap(), s.original_hash);
        drop(locked);
        let r = commit_with(&s, &c, &ops, "overwrite", "", &|_, _| Ok(())).unwrap();
        assert_eq!(r.state, "completed");
        let locked = std::fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&s.path)
            .unwrap();
        assert!(restore(&r.id).unwrap_err().contains("未恢复"));
        assert_eq!(digest(&s.path).unwrap(), r.after_hash);
        drop(locked);
        cleanup(&r.id).unwrap();
        cleanup(&r.id).unwrap();
        restore(&r.id).unwrap();
    }
    let (s, c, ops) = fixture(&temp.path().join("guards/1"));
    let r = commit_with(&s, &c, &ops, "new", "新名称", &|_, _| Ok(())).unwrap();
    let info = Path::new(&r.output).with_file_name("info.sii");
    let saved_info = std::fs::read(&info).unwrap();
    assert_eq!(
        unquote(
            Document::parse(String::from_utf8(saved_info.clone()).unwrap())
                .unwrap()
                .units[0]
                .get("name")
                .unwrap()
        ),
        "新名称"
    );
    std::fs::write(&info, "changed info").unwrap();
    assert!(restore(&r.id).unwrap_err().contains("更新"));
    std::fs::write(&info, saved_info).unwrap();
    std::fs::write(&r.output, "later progress").unwrap();
    assert!(restore(&r.id).unwrap_err().contains("更新"));
    std::fs::write(&r.output, s.preview(&c, &ops).unwrap().0.text).unwrap();
    std::fs::write(Path::new(&r.backup).join("game.sii"), "bad backup").unwrap();
    assert!(restore(&r.id).unwrap_err().contains("备份内容校验失败"));
    assert_eq!(digest(Path::new(&r.output)).unwrap(), r.after_hash);

    // Interrupted new-slot staging: precise, retryable cleanup leaves unrelated data alone.
    let staged = stage_path(&r);
    std::fs::create_dir(&staged).unwrap();
    std::fs::write(staged.join(".workshop-owner"), &r.id).unwrap();
    std::fs::write(staged.join("game.sii"), "partial").unwrap();
    std::fs::write(staged.join("foreign.txt"), "do not delete").unwrap();
    assert!(cleanup(&r.id).unwrap_err().contains("未知内容"));
    assert!(staged.join("game.sii").exists());
    std::fs::remove_file(staged.join("foreign.txt")).unwrap();
    cleanup(&r.id).unwrap();
    cleanup(&r.id).unwrap();
    assert!(!staged.exists());
    // Cleanup is retryable even if interruption occurred before/during marker creation,
    // or between deleting the marker and deleting the now-empty directory.
    std::fs::create_dir(&staged).unwrap();
    cleanup(&r.id).unwrap();
    std::fs::create_dir(&staged).unwrap();
    std::fs::write(staged.join(".workshop-owner"), &r.id[..3]).unwrap();
    cleanup(&r.id).unwrap();
    assert!(!staged.exists());
    assert!(Path::new(&r.output).exists());
    assert!(Path::new(&r.backup).exists());
    assert!(receipt("../outside").is_err());
    // Interrupted private slots are neither discoverable nor manually editable.
    let docs = temp.path().join("discovery");
    let hidden = docs.join("profiles/p/save/.workshop-deadbeef");
    let (visible, _, _) = fixture(&docs.join("profiles/p/save/1"));
    std::fs::create_dir_all(&hidden).unwrap();
    std::fs::copy(&visible.path, hidden.join("game.sii")).unwrap();
    std::fs::copy(
        visible.path.with_file_name("info.sii"),
        hidden.join("info.sii"),
    )
    .unwrap();
    let settings = Settings {
        documents: docs.to_string_lossy().into(),
        ..Settings::default()
    };
    assert!(
        !discover_with(&settings, &crate::discovery::DiscoveryInputs::default())
            .unwrap()
            .iter()
            .any(|s| s.path.contains(".workshop-"))
    );
    assert!(Session::open(&hidden.join("game.sii"), &settings)
        .err()
        .unwrap()
        .contains("暂存文件"));
    let constructed = Session {
        path: hidden.join("game.sii"),
        ..visible
    };
    assert!(
        commit_with(&constructed, &c, &ops, "overwrite", "", &|_, _| Ok(()))
            .unwrap_err()
            .contains("暂存文件")
    );
    // Older history remains readable without inventing an info hash.
    let mut legacy = serde_json::to_value(&r).unwrap();
    legacy.as_object_mut().unwrap().remove("state");
    legacy.as_object_mut().unwrap().remove("info_hash");
    let legacy: Receipt = serde_json::from_value(legacy).unwrap();
    assert_eq!(legacy.state, "completed");
    assert!(legacy.info_hash.is_none());
}
