use std::{collections::BTreeMap, path::Path};
use workshop_core::{
    catalog::{Catalog, Definition},
    edit::{self, Operation},
    sii::Document,
    storage::{self, Session, Settings},
};
fn fixture() -> (Document, Catalog) {
    let text="SiiNunit {\nplayer : p {\n trucks: 2\n trucks[0]: t\n trucks[1]: donor\n}\nvehicle : t {\n license_plate: \"TEST 1|germany\"\n accessories: 4\n accessories[0]: eng\n accessories[1]: tank\n accessories[2]: cab\n accessories[3]: chassis\n future_field: &3f123456\n}\nvehicle : donor {\n license_plate: \"TEST 2|germany\"\n accessories: 4\n accessories[0]: engine2\n accessories[1]: beacon\n accessories[2]: cab2\n accessories[3]: chassis2\n}\nvehicle_accessory : eng {\n data_path: \"/def/vehicle/truck/scania.s_2016/engine/770.sii\"\n refund: 32076\n}\nvehicle_addon_accessory : tank {\n data_path: \"/def/vehicle/truck/scania.s_2016/accessory/tank/1000.sii\"\n refund: 1320\n}\nvehicle_accessory : cab {\n data_path: \"/def/vehicle/truck/scania.s_2016/cabin/highline.sii\"\n}\nvehicle_accessory : chassis {\n data_path: \"/def/vehicle/truck/scania.s_2016/chassis/6x4.sii\"\n}\nvehicle_accessory : cab2 {\n data_path: \"/def/vehicle/truck/scania.s_2016/cabin/highline.sii\"\n}\nvehicle_accessory : chassis2 {\n data_path: \"/def/vehicle/truck/scania.s_2016/chassis/6x4.sii\"\n}\nvehicle_accessory : engine2 {\n data_path: \"/def/vehicle/truck/volvo.fh_2024/engine/780.sii\"\n}\nvehicle_addon_accessory : beacon {\n data_path: \"/def/vehicle/truck/scania.s_2016/accessory/beacon/a.sii\"\n slot_name: 0\n slot_hookup: 0\n paint_color: (1, 1, 1)\n}\n}\n";
    let mut c = Catalog::default();
    for (model, cat, file, kind) in [
        ("scania.s_2016", "engine", "770", "accessory_engine_data"),
        ("volvo.fh_2024", "engine", "780", "accessory_engine_data"),
        ("scania.s_2016", "tank", "1000", "accessory_addon_tank_data"),
        ("daf.2021", "tank", "1465", "accessory_addon_tank_data"),
        ("scania.s_2016", "cabin", "highline", "accessory_cabin_data"),
        ("scania.s_2016", "chassis", "6x4", "accessory_chassis_data"),
        ("scania.s_2016", "beacon", "a", "accessory_addon_data"),
    ] {
        let path = format!(
            "/def/vehicle/truck/{model}/{}{cat}/{file}.sii",
            if cat == "tank" || cat == "beacon" {
                "accessory/"
            } else {
                ""
            }
        );
        c.definitions.insert(
            path.clone(),
            Definition {
                raw_name: String::new(),
                names: std::collections::BTreeMap::new(),
                category_names: std::collections::BTreeMap::new(),
                name_alias: None,
                path,
                kind: kind.into(),
                unit: format!("{file}.{model}.{cat}"),
                name: file.into(),
                category: cat.into(),
                model: model.into(),
                source: "fixture".into(),
                metrics: BTreeMap::new(),
                suitable: vec![],
                conflicts: vec![],
                requires: vec![],
            },
        );
    }
    (Document::parse(text.into()).unwrap(), c)
}
fn replace() -> Operation {
    Operation {
        truck_id: "t".into(),
        action: "replace".into(),
        accessory_id: Some("eng".into()),
        candidate_path: "/def/vehicle/truck/volvo.fh_2024/engine/780.sii".into(),
        donor_accessory: Some("engine2".into()),
    }
}
#[test]
fn replacement_changes_only_path() {
    let (d, c) = fixture();
    let (out, p) = edit::apply(&d, &c, &[replace()]).unwrap();
    assert_eq!(
        out.text,
        d.text.replace(
            "/scania.s_2016/engine/770.sii",
            "/volvo.fh_2024/engine/780.sii"
        )
    );
    assert_eq!(p.trucks[0].accessories.len(), 4);
    assert!(out.text.contains("refund: 32076"));
}

#[test]
fn independent_tank_replacement_preserves_existing_unit_and_count() {
    let (d, c) = fixture();
    let op = Operation {
        truck_id: "t".into(),
        action: "replace".into(),
        accessory_id: Some("tank".into()),
        candidate_path: "/def/vehicle/truck/daf.2021/accessory/tank/1465.sii".into(),
        donor_accessory: None,
    };
    let (out, _) = edit::apply(&d, &c, &[op]).unwrap();
    assert_eq!(
        out.text,
        d.text.replace(
            "/def/vehicle/truck/scania.s_2016/accessory/tank/1000.sii",
            "/def/vehicle/truck/daf.2021/accessory/tank/1465.sii"
        )
    );
    assert_eq!(
        out.unit("t").unwrap().array("accessories").unwrap().len(),
        4
    );
}

#[test]
fn transmission_replacement_preserves_original_state() {
    let (d, mut c) = fixture();
    let old = "/def/vehicle/truck/scania.s_2016/transmission/12od.sii";
    let new = "/def/vehicle/truck/volvo.fh_2024/transmission/12.sii";
    for path in [old, new] {
        let mut def = c.definitions["/def/vehicle/truck/scania.s_2016/engine/770.sii"].clone();
        def.path = path.into();
        def.category = "transmission".into();
        def.kind = "accessory_transmission_data".into();
        c.definitions.insert(path.into(), def);
    }
    let d = Document::parse(
        d.text
            .replace("/def/vehicle/truck/scania.s_2016/engine/770.sii", old),
    )
    .unwrap();
    let mut op = replace();
    op.candidate_path = new.into();
    let (out, _) = edit::apply(&d, &c, &[op]).unwrap();
    assert_eq!(out.text, d.text.replace(old, new));
}
#[test]
fn duplicate_core_rejected() {
    let (d, c) = fixture();
    for cat in ["engine", "chassis", "tank"] {
        let mut op = replace();
        op.action = "add".into();
        op.accessory_id = None;
        op.candidate_path = c
            .definitions
            .values()
            .find(|a| a.category == cat)
            .unwrap()
            .path
            .clone();
        assert!(edit::apply(&d, &c, &[op]).unwrap_err().contains("禁止追加"));
    }
}
#[test]
fn addon_clone_and_dependencies() {
    let (d, c) = fixture();
    let op = Operation {
        truck_id: "t".into(),
        action: "add".into(),
        accessory_id: None,
        candidate_path: "/def/vehicle/truck/scania.s_2016/accessory/beacon/a.sii".into(),
        donor_accessory: Some("beacon".into()),
    };
    let (out, _) = edit::apply(&d, &c, &[op.clone()]).unwrap();
    let refs = out.unit("t").unwrap().array("accessories").unwrap();
    assert_eq!(refs.len(), 5);
    assert_ne!(refs[4], "beacon");
    assert_eq!(
        out.unit("beacon").unwrap().fields.len(),
        d.unit("beacon").unwrap().fields.len()
    );
    out.validate_vehicles().unwrap();
    assert!(edit::apply(&out, &c, &[op.clone()]).is_err());
    let bad = d.replace("beacon", "slot_hookup", "1".into()).unwrap();
    assert!(edit::apply(&bad, &c, &[op]).is_err());
}
#[test]
fn rejects_electric_and_cross_category() {
    let (d, mut c) = fixture();
    c.definitions
        .get_mut(&replace().candidate_path)
        .unwrap()
        .metrics
        .insert("type".into(), "electric".into());
    assert!(edit::apply(&d, &c, &[replace()]).is_err());
    let mut op = replace();
    op.accessory_id = Some("tank".into());
    assert!(edit::apply(&d, &c, &[op]).is_err());
}
#[test]
fn ids_can_change_on_resave() {
    let (d, c) = fixture();
    let (out, p) = edit::apply(&d, &c, &[replace()]).unwrap();
    let renamed = Document::parse(
        out.text
            .replace(": t {", ": other {")
            .replace("trucks[0]: t\n", "trucks[0]: other\n"),
    )
    .unwrap();
    let result = edit::verify(&renamed, &c, &p.changes).unwrap();
    assert_eq!(result[0].status, "已保留");
}

#[test]
fn rejects_reverse_conflict_from_existing_accessory() {
    let (d, mut c) = fixture();
    let old = "/def/vehicle/truck/scania.s_2016/accessory/beacon/a.sii";
    let mut new = c.definitions[old].clone();
    new.path = old.replace("/a.sii", "/b.sii");
    new.unit = "b.scania.s_2016.beacon".into();
    c.definitions
        .get_mut("/def/vehicle/truck/scania.s_2016/engine/770.sii")
        .unwrap()
        .conflicts
        .push(new.unit.clone());
    c.definitions
        .get_mut("/def/vehicle/truck/scania.s_2016/engine/770.sii")
        .unwrap()
        .suitable
        .push("missing.factory.chassis".into());
    c.definitions.insert(new.path.clone(), new.clone());
    let d = Document::parse(d.text.replace(
        "/def/vehicle/truck/scania.s_2016/accessory/tank/1000.sii",
        old,
    ))
    .unwrap();
    let op = Operation {
        truck_id: "t".into(),
        action: "replace".into(),
        accessory_id: Some("tank".into()),
        candidate_path: new.path,
        donor_accessory: None,
    };
    assert!(edit::apply(&d, &c, &[op]).unwrap_err().contains("现有配件"));
}

#[test]
fn wheel_verification_uses_axle_offset_not_any_matching_tire() {
    let (d, c) = fixture();
    let mut text = d.text.replace(
        "accessories: 4\n accessories[0]: eng",
        "accessories: 6\n accessories[4]: tire1\n accessories[5]: tire2\n accessories[0]: eng",
    );
    let pos = text.rfind('}').unwrap();
    text.insert_str(pos,"vehicle_wheel_accessory : tire1 {\n offset: 0\n data_path: \"/def/vehicle/r_tire/a.sii\"\n}\nvehicle_wheel_accessory : tire2 {\n offset: 2\n data_path: \"/def/vehicle/r_tire/b.sii\"\n}\n");
    let d = Document::parse(text).unwrap();
    let mut position = BTreeMap::new();
    position.insert("offset".into(), "0".into());
    let mut change = edit::Change {
        truck_id: "t".into(),
        model: "scania.s_2016".into(),
        plate: "TEST 1".into(),
        category: "r_tire".into(),
        action: "replace".into(),
        before: "/def/vehicle/r_tire/a.sii".into(),
        after: "/def/vehicle/r_tire/b.sii".into(),
        position,
    };
    assert_eq!(
        edit::verify(&d, &c, &[change.clone()]).unwrap()[0].status,
        "未保留 / 已被替换"
    );
    change.position.clear();
    assert!(edit::verify(&d, &c, &[change]).unwrap()[0]
        .status
        .contains("无法唯一匹配"));
}
#[test]
fn backup_new_slot_restore_and_concurrent_save() {
    let temp = tempfile::tempdir().unwrap();
    std::env::set_var("ETS2_WORKSHOP_DATA_DIR", temp.path().join("app"));
    let (d, mut c) = fixture();
    let game = temp.path().join("synthetic-game");
    std::fs::create_dir_all(&game).unwrap();
    let pack = game.join("def.scs");
    std::fs::write(&pack, b"synthetic game identity").unwrap();
    c.parser_version = workshop_core::catalog::BUILD_CACHE_VERSION;
    c.game_path = game.canonicalize().unwrap().to_string_lossy().into();
    c.source_fingerprint = workshop_core::catalog::current_source_fingerprint(&game).unwrap();
    c.scan_complete = true;
    let meta = std::fs::metadata(&pack).unwrap();
    c.archives = vec![(
        pack.to_string_lossy().into(),
        meta.len(),
        meta.modified()
            .unwrap()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_secs(),
    )];
    c.fresh().unwrap();
    let root = temp.path().join("save");
    let slot = root.join("1");
    std::fs::create_dir_all(&slot).unwrap();
    std::fs::write(slot.join("game.sii"), &d.text).unwrap();
    std::fs::write(
        slot.join("info.sii"),
        "SiiNunit {\nsave_container : info {\n name: original\n file_time: 1\n}\n}\n",
    )
    .unwrap();
    let settings = Settings::default();
    let s = Session::open(&slot.join("game.sii"), &settings).unwrap();
    let r = storage::commit(&s, &c, &[replace()], "new", "new test", &settings).unwrap();
    assert_eq!(
        std::fs::read_to_string(slot.join("game.sii")).unwrap(),
        d.text
    );
    assert!(Path::new(&r.backup).join("game.sii").exists());
    let check = Session::open(Path::new(&r.output), &settings).unwrap();
    assert_eq!(
        edit::verify(&check.doc, &c, &r.changes).unwrap()[0].status,
        "已保留"
    );
    storage::restore(&r.id).unwrap();
    assert_eq!(std::fs::read_to_string(&r.output).unwrap(), d.text);
    std::fs::write(slot.join("game.sii"), d.text.replace("TEST 1", "UPDATED")).unwrap();
    assert!(
        storage::commit(&s, &c, &[replace()], "overwrite", "", &settings)
            .unwrap_err()
            .contains("更新")
    );
}
