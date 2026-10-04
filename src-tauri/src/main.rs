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
}
type Shared = Arc<Mutex<Engine>>;
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
            json!({"settings":engine.settings,"catalog_count":engine.catalog.definitions.len(),"catalog_warnings":engine.catalog.warnings,"data_dir":storage::app_dir(),"needs_setup":engine.settings.onboarding_version!=workshop_core::setup::SETUP_VERSION || engine.catalog.definitions.is_empty()}),
        ),
        "detect" => Ok(json!(workshop_core::setup::detect())),
        "setup" => {
            let s: Settings =
                serde_json::from_value(v["settings"].clone()).map_err(|e| e.to_string())?;
            let (s, c) = workshop_core::setup::prepare(s, progress)?;
            engine.settings = s.clone();
            engine.catalog = c;
            engine.session = None;
            Ok(json!({"settings":s,"count":engine.catalog.definitions.len()}))
        }
        "settings" => {
            let s: Settings =
                serde_json::from_value(v["settings"].clone()).map_err(|e| e.to_string())?;
            workshop_core::setup::validate(&s)?;
            if s.game != engine.settings.game {
                engine.catalog = Catalog::default();
                let _ = std::fs::remove_file(storage::catalog_path(
                    &engine.settings,
                    &storage::app_dir(),
                ));
            }
            storage::save_settings(&s)?;
            engine.settings = s;
            Ok(json!(true))
        }
        "discover" => Ok(json!(storage::discover(&engine.settings)?)),
        "open" => {
            engine.session = None;
            let s = Session::open(Path::new(field(&v, "path")?), &engine.settings)?;
            let view = s.view(&engine.catalog)?;
            engine.session = Some(s);
            Ok(json!(view))
        }
        "preview" => {
            let s = engine.session.as_ref().ok_or("请先打开存档")?;
            Ok(json!(s.preview(&engine.catalog, &ops()?)?.1))
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
            Ok(json!(r))
        }
        "catalog" => {
            let mut list: Vec<_> = engine.catalog.definitions.values().collect();
            list.sort_by(|a, b| a.path.cmp(&b.path));
            Ok(json!(list))
        }
        "index" => {
            engine.settings.extractor = workshop_core::setup::ensure_extractor(progress)?
                .to_string_lossy()
                .into();
            let s = &engine.settings;
            let c = workshop_core::catalog::build(
                Path::new(&s.game),
                Path::new(&s.extractor),
                &storage::app_dir(),
            )?;
            std::fs::create_dir_all(storage::app_dir()).map_err(|e| e.to_string())?;
            std::fs::write(
                storage::catalog_path(&engine.settings, &storage::app_dir()),
                serde_json::to_vec(&c).unwrap(),
            )
            .map_err(|e| e.to_string())?;
            let count = c.definitions.len();
            engine.catalog = c;
            Ok(json!({"count":count,"warnings":engine.catalog.warnings}))
        }
        "history" => Ok(json!(storage::receipts())),
        "restore" => Ok(json!(storage::restore(field(&v, "id")?)?)),
        "verify" => {
            let r = storage::receipts()
                .into_iter()
                .find(|r| r.id == v["id"].as_str().unwrap_or(""))
                .ok_or("找不到改装记录")?;
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
    }));
    tauri::Builder::default()
        .manage(engine)
        .invoke_handler(tauri::generate_handler![rpc])
        .run(tauri::generate_context!())
        .expect("桌面应用启动失败");
}
