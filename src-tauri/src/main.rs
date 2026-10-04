#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use serde_json::{json, Value};
use std::{
    path::Path,
    sync::{Arc, Mutex},
};
use tauri::Emitter;
use workshop_core::{
    catalog::Catalog,
    edit::Operation,
    storage::{self, Session, Settings},
    Result,
};
struct Engine {
    settings: Settings,
    catalog: Catalog,
    session: Option<Session>,
    save_list_cache: workshop_core::discovery::SaveListCache,
    pending_operations: bool,
}
type Shared = Arc<Mutex<Engine>>;
fn same_game(a: &str, b: &str) -> bool {
    match (std::fs::canonicalize(a), std::fs::canonicalize(b)) {
        (Ok(a), Ok(b)) => a == b,
        _ => a == b,
    }
}
fn change_settings(
    engine: &mut Engine,
    mut s: Settings,
    prepare: impl FnOnce(Settings) -> Result<(Settings, Catalog)>,
    save: impl FnOnce(&Settings) -> Result<()>,
) -> Result<bool> {
    let changed = !same_game(&s.game, &engine.settings.game);
    if changed && engine.pending_operations {
        return Err("请先保存或移除待保存修改，再更换游戏安装或重新配置".into());
    }
    if changed {
        let (published, catalog) = prepare(s)?;
        engine.settings = published;
        engine.catalog = catalog;
        engine.session = None;
        engine.pending_operations = false;
    } else {
        s.catalog_file = engine.settings.catalog_file.clone();
        s.onboarding_version = engine.settings.onboarding_version;
        s.extractor = engine.settings.extractor.clone();
        save(&s)?;
        engine.settings = s;
    }
    engine.save_list_cache = Default::default();
    Ok(changed)
}
fn field<'a>(v: &'a Value, key: &str) -> Result<&'a str> {
    v.get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("缺少参数 {key}"))
}
fn request(engine: &mut Engine, v: Value, progress: &dyn Fn(&str)) -> Result<Value> {
    let ops = || -> Result<Vec<Operation>> {
        serde_json::from_value(v.get("operations").cloned().unwrap_or(json!([])))
            .map_err(|e| e.to_string())
    };
    match field(&v, "action")? {
        "init" => Ok(
            json!({"settings":engine.settings,"catalog_count":engine.catalog.definitions.len(),"catalog_name_schema":engine.catalog.name_schema,"catalog_warnings":engine.catalog.warnings,"catalog_rebuild_reason":engine.catalog.rebuild_reason(),"data_dir":storage::app_dir(),"needs_setup":engine.settings.onboarding_version!=workshop_core::setup::SETUP_VERSION || engine.catalog.definitions.is_empty()}),
        ),
        "detect" => Ok(json!(workshop_core::setup::detect())),
        "setup" => {
            if engine.pending_operations {
                return Err("请先保存或移除待保存修改，再更换游戏安装或重新配置".into());
            }
            let s: Settings =
                serde_json::from_value(v["settings"].clone()).map_err(|e| e.to_string())?;
            let (s, c) = workshop_core::setup::prepare(s, progress)?;
            engine.settings = s.clone();
            engine.catalog = c;
            engine.session = None;
            engine.pending_operations = false;
            engine.save_list_cache = Default::default();
            Ok(json!({"settings":s,"count":engine.catalog.definitions.len()}))
        }
        "settings" => {
            let s: Settings =
                serde_json::from_value(v["settings"].clone()).map_err(|e| e.to_string())?;
            let changed = change_settings(
                engine,
                s,
                |s| workshop_core::setup::prepare(s, progress),
                |s| {
                    workshop_core::setup::validate(s)?;
                    storage::save_settings(s)
                },
            )?;
            Ok(json!({"settings":engine.settings,"catalog_changed":changed}))
        }
        "discover" => {
            let saves = engine.save_list_cache.discover(
                Path::new(&engine.settings.documents),
                &workshop_core::discovery::DiscoveryInputs::from_host(),
                v["include_autosaves"].as_bool().unwrap_or(true),
            )?;
            Ok(json!({"saves":saves,"warnings":engine.save_list_cache.warnings()}))
        }
        "open" => {
            engine.session = None;
            engine.pending_operations = false;
            let s = Session::open(Path::new(field(&v, "path")?), &engine.settings)?;
            let view = s.view(&engine.catalog)?;
            engine.session = Some(s);
            Ok(json!(view))
        }
        "preview" => {
            let s = engine.session.as_ref().ok_or("请先打开存档")?;
            let operations = ops()?;
            let preview = s.preview(&engine.catalog, &operations)?.1;
            engine.pending_operations = !operations.is_empty();
            Ok(json!(preview))
        }
        "commit" => {
            let s = engine.session.as_ref().ok_or("请先打开存档")?;
            let r = storage::commit(
                s,
                &engine.catalog,
                &ops()?,
                field(&v, "mode")?,
                v["name"].as_str().unwrap_or("workshop"),
                &engine.settings,
            )?;
            engine.session = None;
            engine.pending_operations = false;
            Ok(json!(r))
        }
        "catalog" => {
            let mut list: Vec<_> = engine.catalog.definitions.values().collect();
            list.sort_by(|a, b| a.path.cmp(&b.path));
            Ok(json!(list))
        }
        "index" => {
            if engine.pending_operations {
                return Err("请先保存或移除待保存修改，再更新目录。".into());
            }
            let c = workshop_core::catalog::build_lazy(
                Path::new(&engine.settings.game),
                &storage::app_dir(),
                progress,
                &|| workshop_core::setup::ensure_extractor(progress),
            )?;
            let settings = workshop_core::setup::publish_catalog(
                engine.settings.clone(),
                &c,
                &storage::app_dir(),
            )?;
            let count = c.definitions.len();
            engine.settings = settings;
            engine.catalog = c;
            Ok(json!({"count":count,"warnings":engine.catalog.warnings}))
        }
        "history" => Ok(json!(storage::receipts())),
        "cleanup" => Ok(json!(storage::cleanup(field(&v, "id")?)?)),
        "restore" => {
            let path = storage::restore(field(&v, "id")?)?;
            engine.session = None;
            engine.pending_operations = false;
            Ok(json!(path))
        }
        "verify" => {
            let r = storage::receipt(field(&v, "id")?)?;
            let s = Session::open(Path::new(field(&v, "path")?), &engine.settings)?;
            Ok(json!(workshop_core::edit::verify(
                &s.doc,
                &engine.catalog,
                &r.changes
            )?))
        }
        _ => Err("未知命令".into()),
    }
}
#[tauri::command]
async fn rpc(
    app: tauri::AppHandle,
    state: tauri::State<'_, Shared>,
    payload: Value,
) -> Result<Value> {
    let shared = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        let mut e = shared.lock().map_err(|_| "后台状态不可用")?;
        request(&mut e, payload, &|message| {
            let _ = app.emit("setup-progress", message);
        })
    })
    .await
    .map_err(|e| e.to_string())?
}
fn main() {
    let args: Vec<_> = std::env::args().collect();
    if args.get(1).map(String::as_str) == Some("--decode-worker") {
        let result = if args.len() == 3 {
            workshop_core::decoder::worker(Path::new(&args[2]))
        } else {
            Err("参数错误".into())
        };
        if let Err(e) = result {
            eprintln!("{e}");
            std::process::exit(1);
        }
        return;
    }
    let settings = storage::settings();
    let catalog = std::fs::read(storage::catalog_path(&settings, &storage::app_dir()))
        .ok()
        .and_then(|b| serde_json::from_slice(&b).ok())
        .unwrap_or_default();
    let engine = Arc::new(Mutex::new(Engine {
        settings,
        catalog,
        session: None,
        save_list_cache: Default::default(),
        pending_operations: false,
    }));
    tauri::Builder::default()
        .manage(engine)
        .invoke_handler(tauri::generate_handler![rpc])
        .run(tauri::generate_context!())
        .expect("桌面应用启动失败");
}

#[cfg(test)]
mod settings_tests {
    use super::*;
    #[test]
    fn failed_switch_preserves_state_and_success_clears_only_game_session() {
        let mut engine = Engine {
            settings: Settings {
                game: "old-install".into(),
                catalog_file: "catalog-old.json".into(),
                ..Settings::default()
            },
            catalog: Catalog {
                signature: "old".into(),
                ..Catalog::default()
            },
            session: Some(Session {
                path: "synthetic/game.sii".into(),
                original_hash: "original".into(),
                info_hash: "info".into(),
                doc: workshop_core::sii::Document::parse("SiiNunit {\n}\n".into()).unwrap(),
            }),
            save_list_cache: Default::default(),
            pending_operations: true,
        };
        let mut candidate = engine.settings.clone();
        candidate.game = "new-install".into();
        assert!(change_settings(
            &mut engine,
            candidate.clone(),
            |_| panic!("pending changes must block preparation"),
            |_| panic!("must not publish")
        )
        .unwrap_err()
        .contains("待保存"));
        engine.pending_operations = false;
        assert!(change_settings(
            &mut engine,
            candidate.clone(),
            |_| Err("publication failed".into()),
            |_| panic!("wrong path")
        )
        .is_err());
        assert_eq!(engine.settings.game, "old-install");
        assert_eq!(engine.catalog.signature, "old");
        assert!(engine.session.is_some());
        let mut documents_only = engine.settings.clone();
        documents_only.documents = "new-discovery-root".into();
        documents_only.catalog_file = "catalog-untrusted-draft.json".into();
        engine.pending_operations = true;
        assert!(!change_settings(
            &mut engine,
            documents_only,
            |_| panic!("should not rebuild"),
            |s| {
                assert_eq!(s.catalog_file, "catalog-old.json");
                Ok(())
            }
        )
        .unwrap());
        assert!(engine.session.is_some() && engine.pending_operations);
        assert_eq!(engine.catalog.signature, "old");
        engine.pending_operations = false;
        assert!(change_settings(
            &mut engine,
            candidate,
            |s| Ok((
                s,
                Catalog {
                    signature: "new".into(),
                    ..Catalog::default()
                }
            )),
            |_| panic!("wrong path")
        )
        .unwrap());
        assert_eq!(engine.catalog.signature, "new");
        assert!(engine.session.is_none());
    }
}
