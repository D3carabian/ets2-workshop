use std::{
    collections::{BTreeMap, HashSet},
    fmt::Write,
    time::Instant,
};
use workshop_core::{
    catalog::{Catalog, Definition},
    drivers::{self, DriverKind},
    edit::{self, Operation},
    garage,
    sii::Document,
};

// Entirely synthetic; no save, profile, game archive, or filesystem access.
fn fleet(count: usize) -> (Document, Catalog) {
    let mut text = format!("SiiNunit {{\nplayer : p {{\n trucks: {count}\n assigned_truck: t0\n");
    for i in 0..count {
        writeln!(text, " trucks[{i}]: t{i}").unwrap();
    }
    text.push_str("}\n");
    let mut catalog = Catalog::default();
    for i in 0..count {
        writeln!(text, "vehicle : t{i} {{\n accessories: 1\n accessories[0]: a{i}\n}}\nvehicle_accessory : a{i} {{\n data_path: \"/def/vehicle/truck/scania.s_2016/engine/a.sii\"\n}}\ndriver_ai : driver.{i} {{\n assigned_truck: null\n}}").unwrap();
        catalog
            .driver_names
            .insert(format!("driver.{i}"), format!("Synthetic Driver {i}"));
    }
    writeln!(text, "garage : g {{\n vehicles: {count}\n drivers: {count}").unwrap();
    // Interleaved fields, emitted in reverse: pair by numeric index, not order.
    for i in (0..count).rev() {
        writeln!(text, " vehicles[{i}]: t{i}\n drivers[{i}]: driver.{i}").unwrap();
    }
    text.push_str("}\n}\n");
    for name in ["a", "b"] {
        let path = format!("/def/vehicle/truck/scania.s_2016/engine/{name}.sii");
        catalog.definitions.insert(
            path.clone(),
            Definition {
                path,
                kind: "accessory_engine_data".into(),
                unit: format!("{name}.engine"),
                name: name.into(),
                raw_name: name.into(),
                names: BTreeMap::new(),
                category_names: BTreeMap::new(),
                name_alias: None,
                category: "engine".into(),
                model: "scania.s_2016".into(),
                source: "synthetic".into(),
                metrics: BTreeMap::new(),
                suitable: vec![],
                conflicts: vec![],
                requires: vec![],
            },
        );
    }
    (Document::parse(text).unwrap(), catalog)
}

#[test]
fn indexed_slots_resolve_names_and_ignore_transient_ai_assignment() {
    let (doc, catalog) = fleet(4);
    let trucks = garage::inventory(&doc, &catalog).unwrap();
    assert_eq!(trucks[0].driver.kind, DriverKind::Player);
    for (i, truck) in trucks.iter().enumerate().skip(1) {
        assert_eq!(truck.driver.kind, DriverKind::Employee);
        assert_eq!(
            truck.driver.id.as_deref(),
            Some(format!("driver.{i}").as_str())
        );
        assert_eq!(
            truck.driver.name.as_deref(),
            Some(format!("Synthetic Driver {i}").as_str())
        );
    }
    let no_names = garage::inventory(&doc, &Catalog::default()).unwrap();
    assert_eq!(no_names[1].driver.kind, DriverKind::Employee);
    assert!(no_names[1].driver.name.is_none());
}

#[test]
fn null_missing_wrong_type_and_unknown_driver_ids_are_distinct() {
    let (doc, catalog) = fleet(5);
    let text = doc
        .text
        .replace("drivers[1]: driver.1", "drivers[1]: null")
        .replace("drivers[2]: driver.2", "drivers[2]: missing")
        .replace("drivers[3]: driver.3", "drivers[3]: a3")
        .replace("driver.4", "custom_driver");
    let trucks = garage::inventory(&Document::parse(text).unwrap(), &catalog).unwrap();
    assert_eq!(trucks[1].driver.kind, DriverKind::Unassigned);
    assert_eq!(trucks[2].driver.kind, DriverKind::Unknown);
    assert_eq!(trucks[3].driver.kind, DriverKind::Unknown);
    assert_eq!(trucks[4].driver.kind, DriverKind::Employee);
    assert_eq!(trucks[4].driver.id.as_deref(), Some("custom_driver"));
    assert!(trucks[4].driver.name.is_none());
}

#[test]
fn name_indices_are_decimal_and_missing_slots_do_not_infer_ownership() {
    let (doc, mut catalog) = fleet(4);
    catalog.driver_names.insert("driver.3".into(), " ".into());
    let text = doc
        .text
        .replace("driver.1", "driver.001")
        .replace("vehicles[2]: t2", "vehicles[2]: null");
    let trucks = garage::inventory(&Document::parse(text).unwrap(), &catalog).unwrap();
    assert_eq!(trucks[1].driver.id.as_deref(), Some("driver.001"));
    assert_eq!(trucks[1].driver.name.as_deref(), Some("Synthetic Driver 1"));
    assert_eq!(trucks[2].driver.kind, DriverKind::Unknown);
    assert!(trucks[2].driver.name.is_none());
    assert_eq!(trucks[3].driver.kind, DriverKind::Employee);
    assert!(trucks[3].driver.name.is_none());
}

#[test]
fn malformed_or_repeated_slots_and_shared_drivers_are_unknown() {
    let (doc, catalog) = fleet(4);
    for malformed in [
        doc.text.replace(" drivers: 4", " drivers: 3"),
        doc.text.replace(" drivers[2]: driver.2\n", ""),
        doc.text.replace("vehicles[2]: t2", "vehicles[2]: t1"),
        doc.text
            .replace("drivers[2]: driver.2", "drivers[2]: driver.1"),
    ] {
        let trucks = garage::inventory(&Document::parse(malformed).unwrap(), &catalog).unwrap();
        assert_eq!(trucks[0].driver.kind, DriverKind::Player);
        assert_eq!(trucks[1].driver.kind, DriverKind::Unknown);
        assert_eq!(trucks[2].driver.kind, DriverKind::Unknown);
    }
    let duplicate =
        "garage : extra {\n vehicles: 1\n vehicles[0]: t1\n drivers: 1\n drivers[0]: driver.1\n}\n";
    let bad = "garage : bad {\n vehicles: 1\n vehicles[0]: t1\n drivers: 0\n}\n";
    for extra in [duplicate, bad] {
        for before in [false, true] {
            let offset = if before {
                doc.text.find("garage : g").unwrap()
            } else {
                doc.closing
            };
            let mut text = doc.text.clone();
            text.insert_str(offset, extra);
            let trucks = garage::inventory(&Document::parse(text).unwrap(), &catalog).unwrap();
            assert_eq!(trucks[1].driver.kind, DriverKind::Unknown);
        }
    }
}

#[test]
fn modern_current_vehicle_and_garage_player_remain_player() {
    let (doc, catalog) = fleet(3);
    let text = doc
        .text
        .replace(" assigned_truck: t0", " assigned_vehicles: active")
        .replace("drivers[0]: driver.0", "drivers[0]: player_driver");
    let mut doc = Document::parse(text).unwrap();
    let extra = "player_vehicles : active {\n vehicle: t2\n trailer: null\n}\ndriver_player : player_driver {}\n";
    doc = doc
        .patch(vec![(doc.closing..doc.closing, extra.into())])
        .unwrap();
    let trucks = garage::inventory(&doc, &catalog).unwrap();
    assert!(trucks[2].current);
    assert!(!trucks[0].current);
    assert_eq!(trucks[2].driver.kind, DriverKind::Player);
    assert_eq!(trucks[0].driver.kind, DriverKind::Player);
    assert_eq!(trucks[0].driver.id.as_deref(), Some("player_driver"));
}

#[test]
fn preview_and_undo_preserve_driver_identity_without_touching_save_fields() {
    let (doc, catalog) = fleet(3);
    let original = garage::inventory(&doc, &catalog).unwrap();
    let op = Operation {
        truck_id: "t1".into(),
        action: "replace".into(),
        accessory_id: Some("a1".into()),
        candidate_path: "/def/vehicle/truck/scania.s_2016/engine/b.sii".into(),
        donor_accessory: None,
    };
    let (edited, preview) = edit::apply(&doc, &catalog, &[op]).unwrap();
    for (before, after) in original.iter().zip(&preview.trucks) {
        assert_eq!(before.driver, after.driver);
    }
    assert_eq!(
        preview.trucks[1].accessories[0].path,
        "/def/vehicle/truck/scania.s_2016/engine/b.sii"
    );
    let original_garage = doc.unit("g").unwrap();
    let edited_garage = edited.unit("g").unwrap();
    assert_eq!(
        &doc.text[original_garage.span.clone()],
        &edited.text[edited_garage.span.clone()]
    );
    let (undone, undo_preview) = edit::apply(&doc, &catalog, &[]).unwrap();
    assert_eq!(undone.text, doc.text);
    assert_eq!(
        serde_json::to_value(undo_preview.trucks).unwrap(),
        serde_json::to_value(original).unwrap()
    );
    assert_eq!(
        serde_json::to_value(preview.trucks).unwrap(),
        serde_json::to_value(garage::inventory(&edited, &catalog).unwrap()).unwrap()
    );
}

#[test]
fn thousands_of_driver_assignments_remain_complete() {
    for count in [1_000, 5_000] {
        let (doc, catalog) = fleet(count);
        let trucks = garage::inventory(&doc, &catalog).unwrap();
        assert_eq!(trucks.len(), count);
        assert_eq!(
            trucks
                .iter()
                .filter(|truck| truck.driver.kind == DriverKind::Employee)
                .count(),
            count - 1
        );
        assert_eq!(
            trucks[count - 1].driver.name.as_deref(),
            Some(format!("Synthetic Driver {}", count - 1).as_str())
        );
    }
}

#[test]
#[ignore = "Explicit release benchmark; reports timings without machine-dependent assertions"]
fn driver_assignment_benchmark() {
    fn median(samples: &mut [std::time::Duration; 30]) -> std::time::Duration {
        samples.sort_unstable();
        (samples[14] + samples[15]) / 2
    }
    for count in [1_000, 5_000] {
        let (doc, catalog) = fleet(count);
        let owned_ids: Vec<_> = (0..count).map(|i| format!("t{i}")).collect();
        let owned: HashSet<_> = owned_ids.iter().map(String::as_str).collect();
        let mut parsing = [std::time::Duration::ZERO; 30];
        let mut assignments = parsing;
        let mut inventory = parsing;
        for run in 0..30 {
            // Fixture/catalog generation and owned input cloning stay outside
            // each measurement; each parser run receives identical source text.
            let source = doc.text.clone();
            let start = Instant::now();
            let parsed = Document::parse(source).unwrap();
            parsing[run] = start.elapsed();
            std::hint::black_box(parsed);

            let start = Instant::now();
            let assigned = drivers::assignments(&doc, &owned, "t0", &catalog.driver_names);
            assignments[run] = start.elapsed();
            std::hint::black_box(assigned);

            let start = Instant::now();
            let trucks = garage::inventory(&doc, &catalog).unwrap();
            inventory[run] = start.elapsed();
            std::hint::black_box(trucks);
        }
        eprintln!(
            "synthetic drivers={count}, parse median={:?}, assignments median={:?}, full inventory median={:?} (30 runs each)",
            median(&mut parsing),
            median(&mut assignments),
            median(&mut inventory)
        );
    }
}
