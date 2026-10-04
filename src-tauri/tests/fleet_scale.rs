use std::{collections::BTreeMap, fmt::Write, time::Instant};
use workshop_core::{
    catalog::{Catalog, Definition},
    edit::{self, Operation},
    garage,
    sii::Document,
};

const MODEL: &str = "scania.s_2016";

fn path(category: &str, name: &str) -> String {
    format!(
        "/def/vehicle/truck/{MODEL}/{}{category}/{name}.sii",
        if category == "beacon" {
            "accessory/"
        } else {
            ""
        }
    )
}

// Entirely synthetic and in memory: no game saves, settings, or backups are accessed.
fn fleet(count: usize) -> (Document, Catalog) {
    let mut catalog = Catalog::default();
    for (category, name, kind) in [
        ("engine", "a", "accessory_engine_data"),
        ("engine", "b", "accessory_engine_data"),
        ("cabin", "a", "accessory_cabin_data"),
        ("chassis", "a", "accessory_chassis_data"),
        ("beacon", "a", "accessory_addon_data"),
        ("beacon", "b", "accessory_addon_data"),
    ] {
        let path = path(category, name);
        catalog.definitions.insert(
            path.clone(),
            Definition {
                raw_name: String::new(),
                names: std::collections::BTreeMap::new(),
                category_names: std::collections::BTreeMap::new(),
                name_alias: None,
                path,
                kind: kind.into(),
                unit: format!("{name}.{MODEL}.{category}"),
                name: name.into(),
                category: category.into(),
                model: MODEL.into(),
                source: "synthetic".into(),
                metrics: BTreeMap::new(),
                suitable: vec![],
                conflicts: vec![],
                requires: vec![],
            },
        );
    }
    let mut text = format!(
        "SiiNunit {{\nplayer : player {{\n trucks: {count}\n assigned_vehicles: assigned\n"
    );
    // Reverse field order exercises numeric indexing rather than text order.
    for i in (0..count).rev() {
        writeln!(text, " trucks[{i}]: truck{i}").unwrap();
    }
    text.push_str("}\nassignment : assigned {\n current: nested\n}\nassignment : nested {\n truck: truck0\n}\n");
    for i in 0..count {
        let donor = i == count - 1;
        writeln!(text, "vehicle : truck{i} {{\n license_plate: \"<color value='x'>SYNTH {i}</color>|germany\"\n accessories: {}\n accessories[0]: engine{i}\n accessories[1]: cabin{i}\n accessories[2]: chassis{i}\n future_field: &12345678", if donor { 4 } else { 3 }).unwrap();
        if donor {
            writeln!(text, " accessories[3]: beacon{i}").unwrap();
        }
        text.push_str("}\n");
        for category in ["engine", "cabin", "chassis", "beacon"] {
            if category == "beacon" && !donor {
                continue;
            }
            let kind = if category == "beacon" {
                "vehicle_addon_accessory"
            } else {
                "vehicle_accessory"
            };
            writeln!(text, "{kind} : {category}{i} {{\n data_path: \"{}\"\n refund: 1234\n wear: &00000000\n}}", path(category, "a")).unwrap();
        }
        // Repeated references in one garage must not duplicate its location label.
        writeln!(text, "garage : garage{i} {{\n vehicles: 1\n vehicles[0]: truck{i}\n last_vehicle: truck{i}\n}}").unwrap();
    }
    text.push_str("garage : overflow {\n parked: truck0\n}\n}\n");
    (Document::parse(text).unwrap(), catalog)
}

fn replace(truck: usize, category: &str, name: &str) -> Operation {
    Operation {
        truck_id: format!("truck{truck}"),
        action: "replace".into(),
        accessory_id: Some(format!("{category}{truck}")),
        candidate_path: path(category, name),
        donor_accessory: None,
    }
}

#[test]
fn inventory_scales_to_five_thousand_synthetic_trucks() {
    for count in [1_000, 5_000] {
        let start = Instant::now();
        let (doc, catalog) = fleet(count);
        let parse = start.elapsed();
        let start = Instant::now();
        let trucks = garage::inventory(&doc, &catalog).unwrap();
        eprintln!(
            "synthetic fleet={count}, bytes={}, units={}, generate+parse={parse:?}, inventory={:?}",
            doc.text.len(),
            doc.units.len(),
            start.elapsed()
        );
        assert_eq!(trucks.len(), count);
        assert_eq!(trucks[0].id, "truck0");
        assert_eq!(trucks[0].plate, "SYNTH 0");
        assert_eq!(trucks[0].location, "garage0, overflow");
        assert_eq!(trucks.iter().filter(|t| t.current).count(), 1);
        assert!(trucks[0].current);
        assert_eq!(trucks[count - 1].location, format!("garage{}", count - 1));
        assert_eq!(trucks[count - 1].accessories.len(), 4);
    }
}

#[test]
fn batch_edits_refresh_targets_and_donors_in_five_thousand_truck_fleet() {
    let (doc, catalog) = fleet(5_000);
    let operations = [
        replace(0, "engine", "b"),
        replace(0, "engine", "a"),
        replace(4_999, "engine", "b"),
        replace(4_999, "beacon", "b"),
        Operation {
            truck_id: "truck0".into(),
            action: "add".into(),
            accessory_id: None,
            candidate_path: path("beacon", "b"),
            donor_accessory: Some("beacon4999".into()),
        },
    ];
    let start = Instant::now();
    let (out, preview) = edit::apply(&doc, &catalog, &operations).unwrap();
    eprintln!(
        "synthetic fleet=5000, operations=5, apply={:?}",
        start.elapsed()
    );
    assert_eq!(preview.trucks.len(), 5_000);
    assert_eq!(preview.changes[1].before, path("engine", "b"));
    assert_eq!(preview.trucks[0].accessories.len(), 4);
    assert_eq!(preview.trucks[0].accessories[0].path, path("engine", "a"));
    assert_eq!(preview.trucks[0].accessories[3].path, path("beacon", "b"));
    assert_ne!(preview.trucks[0].accessories[3].id, "beacon4999");
    assert_eq!(
        preview.trucks[4_999].accessories[0].path,
        path("engine", "b")
    );
    assert_eq!(preview.trucks[0].location, "garage0, overflow");
    assert!(preview.trucks[0].current);
    assert_eq!(
        out.unit("truck0").unwrap().get("future_field"),
        Some("&12345678")
    );
    for id in [
        "truck2500",
        "engine2500",
        "cabin2500",
        "chassis2500",
        "garage2500",
    ] {
        assert_eq!(
            doc.text[doc.unit(id).unwrap().span.clone()],
            out.text[out.unit(id).unwrap().span.clone()]
        );
    }
    // The cached preview must exactly match a fresh inventory, including raw unit text.
    assert_eq!(
        serde_json::to_value(&preview.trucks).unwrap(),
        serde_json::to_value(garage::inventory(&out, &catalog).unwrap()).unwrap()
    );
}

#[test]
fn array_indexing_still_rejects_gaps_and_noncanonical_indices() {
    for fields in [
        " items: 1\n items[1]: x",
        " items: 1\n items[00]: x",
        " items: 2\n items[0]: x",
        " items: 0\n items[]: x",
    ] {
        let doc = Document::parse(format!("SiiNunit {{\na : a {{\n{fields}\n}}\n}}")).unwrap();
        assert!(doc.unit("a").unwrap().array("items").is_err());
    }
}

#[test]
fn later_operation_can_use_a_newly_added_accessory_as_donor() {
    let (doc, catalog) = fleet(3);
    let first = Operation {
        truck_id: "truck0".into(),
        action: "add".into(),
        accessory_id: None,
        candidate_path: path("beacon", "a"),
        donor_accessory: Some("beacon2".into()),
    };
    let (_, single) = edit::apply(&doc, &catalog, &[first.clone()]).unwrap();
    let second = Operation {
        truck_id: "truck1".into(),
        donor_accessory: Some(single.trucks[0].accessories[3].id.clone()),
        ..first.clone()
    };
    let (out, preview) = edit::apply(&doc, &catalog, &[first, second]).unwrap();
    assert_eq!(preview.trucks[1].accessories.len(), 4);
    assert_ne!(
        preview.trucks[0].accessories[3].id,
        preview.trucks[1].accessories[3].id
    );
    assert_eq!(
        serde_json::to_value(&preview.trucks).unwrap(),
        serde_json::to_value(garage::inventory(&out, &catalog).unwrap()).unwrap()
    );
}

#[test]
fn removing_unrelated_operation_keeps_added_accessory_identity() {
    let (doc, catalog) = fleet(3);
    let engine = Operation {
        truck_id: "truck0".into(),
        action: "replace".into(),
        accessory_id: Some("engine0".into()),
        candidate_path: path("engine", "b"),
        donor_accessory: None,
    };
    let add = Operation {
        truck_id: "truck0".into(),
        action: "add".into(),
        accessory_id: None,
        candidate_path: path("beacon", "a"),
        donor_accessory: Some("beacon2".into()),
    };
    let (_, initial) = edit::apply(&doc, &catalog, &[engine.clone(), add.clone()]).unwrap();
    let id = initial.trucks[0].accessories[3].id.clone();
    let change = Operation {
        truck_id: "truck0".into(),
        action: "replace".into(),
        accessory_id: Some(id.clone()),
        candidate_path: path("beacon", "b"),
        donor_accessory: None,
    };
    edit::apply(&doc, &catalog, &[engine, add.clone(), change.clone()]).unwrap();
    let (_, after_undo) = edit::apply(&doc, &catalog, &[add, change.clone()]).unwrap();
    assert_eq!(after_undo.trucks[0].accessories[3].id, id);
    assert_eq!(
        after_undo.trucks[0].accessories[3].path,
        path("beacon", "b")
    );
    assert_eq!(
        after_undo.trucks[0].accessories[0].path,
        path("engine", "a")
    );
    assert!(edit::apply(&doc, &catalog, &[change])
        .unwrap_err()
        .contains("追加操作"));
}

#[test]
fn operation_shape_is_validated_before_editing() {
    let (doc, catalog) = fleet(3);
    let mut op = Operation {
        truck_id: "truck0".into(),
        action: "add".into(),
        accessory_id: Some("engine0".into()),
        candidate_path: path("beacon", "a"),
        donor_accessory: Some("beacon2".into()),
    };
    assert!(edit::apply(&doc, &catalog, &[op.clone()])
        .unwrap_err()
        .contains("同时指定"));
    op.action = "replace".into();
    op.accessory_id = None;
    assert!(edit::apply(&doc, &catalog, &[op.clone()])
        .unwrap_err()
        .contains("未选择"));
    op.action = "unknown".into();
    assert!(edit::apply(&doc, &catalog, &[op])
        .unwrap_err()
        .contains("未知操作"));
}
