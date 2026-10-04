//! Read-only truck assignments. Garage vehicle/driver arrays share slot indices;
//! AI `assigned_truck` describes transient work and is often null.
use crate::sii::Document;
use serde::Serialize;
use std::collections::{hash_map::Entry, HashMap, HashSet};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum DriverKind {
    Player,
    Employee,
    Unassigned,
    #[default]
    Unknown,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize)]
pub struct Driver {
    pub kind: DriverKind,
    pub id: Option<String>,
    pub name: Option<String>,
}

/// Build the assignment index once, without a driver scan for each truck.
/// Invalid or repeated slots are deliberately unknown instead of guessed.
pub fn assignments(
    doc: &Document,
    owned: &HashSet<&str>,
    current: &str,
    names: &HashMap<String, String>,
) -> HashMap<String, Driver> {
    let mut found: HashMap<String, Driver> = HashMap::with_capacity(owned.len());
    let mut driver_trucks: HashMap<&str, Option<&str>> = HashMap::new();
    for garage in doc.units.iter().filter(|unit| unit.kind == "garage") {
        let vehicles = garage.array("vehicles");
        let drivers = garage.array("drivers");
        let valid = matches!((&vehicles, &drivers), (Ok(v), Ok(d)) if v.len() == d.len());
        if !valid {
            // Even malformed arrays must invalidate references already found in
            // another garage. Do not infer ownership from unrelated fields.
            for field in &garage.fields {
                if field.key.starts_with("vehicles[") && owned.contains(field.value.as_str()) {
                    found.insert(field.value.clone(), Driver::default());
                }
            }
            continue;
        }
        for (vehicle, driver_id) in vehicles
            .as_ref()
            .unwrap()
            .iter()
            .zip(drivers.as_ref().unwrap())
        {
            if !owned.contains(vehicle.as_str()) {
                continue;
            }
            let mut driver = if driver_id == "null" {
                Driver {
                    kind: DriverKind::Unassigned,
                    ..Driver::default()
                }
            } else if let Ok(unit) = doc.unit(driver_id) {
                let kind = match unit.kind.as_str() {
                    "driver_ai" => DriverKind::Employee,
                    "driver_player" => DriverKind::Player,
                    _ => DriverKind::Unknown,
                };
                let name = if kind == DriverKind::Employee {
                    driver_id
                        .strip_prefix("driver.")
                        .filter(|index| {
                            !index.is_empty() && index.bytes().all(|b| b.is_ascii_digit())
                        })
                        .and_then(|index| index.parse::<u32>().ok())
                        .and_then(|index| names.get(&format!("driver.{index}")))
                        .filter(|name| !name.trim().is_empty())
                        .cloned()
                } else {
                    None
                };
                Driver {
                    kind,
                    id: (kind != DriverKind::Unknown).then(|| driver_id.clone()),
                    name,
                }
            } else {
                Driver::default()
            };
            if matches!(driver.kind, DriverKind::Player | DriverKind::Employee) {
                // IDs and vehicle references borrow the parsed document, not
                // temporary array strings, so this index stays linear in size.
                let driver_key = doc.unit(driver_id).unwrap().id.as_str();
                let vehicle_key = owned.get(vehicle.as_str()).copied().unwrap();
                match driver_trucks.entry(driver_key) {
                    Entry::Vacant(entry) => {
                        entry.insert(Some(vehicle_key));
                    }
                    Entry::Occupied(mut entry) => {
                        if let Some(previous) = *entry.get() {
                            if previous != vehicle_key {
                                found.insert(previous.to_owned(), Driver::default());
                                entry.insert(None);
                                driver = Driver::default();
                            }
                        } else {
                            driver = Driver::default();
                        }
                    }
                }
            }
            match found.entry(vehicle.clone()) {
                Entry::Vacant(entry) => {
                    entry.insert(driver);
                }
                Entry::Occupied(mut entry) => {
                    entry.insert(Driver::default());
                }
            }
        }
    }
    for &vehicle in owned {
        found.entry(vehicle.to_owned()).or_default();
    }
    // The player's active vehicle is independently identified by player data.
    if owned.contains(current) {
        let driver = found.entry(current.to_owned()).or_default();
        if driver.kind != DriverKind::Player {
            *driver = Driver {
                kind: DriverKind::Player,
                ..Driver::default()
            };
        }
    }
    found
}
