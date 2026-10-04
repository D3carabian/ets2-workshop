//! Development-only fixture binding. Never built into the production CLI or release bundle.
use std::{fs, path::Path, time::UNIX_EPOCH};
use workshop_core::{catalog, hash, Result};

fn checked_path(path: &Path, verification: &Path) -> Result<std::path::PathBuf> {
    let path = path.canonicalize().map_err(|e| e.to_string())?;
    if !path.starts_with(verification) || path == verification {
        return Err(
            "Synthetic fixtures must remain inside this checkout's verification directory".into(),
        );
    }
    Ok(path)
}

fn run() -> Result<()> {
    let args: Vec<_> = std::env::args_os().skip(1).collect();
    if args.len() != 2 {
        return Err("Usage: synthetic_catalog <fixture-catalog.json> <synthetic-game>".into());
    }
    let repo = Path::new(env!("CARGO_MANIFEST_DIR")).parent().unwrap();
    let verification = repo
        .join("verification")
        .canonicalize()
        .map_err(|e| e.to_string())?;
    let json = checked_path(Path::new(&args[0]), &verification)?;
    let game = checked_path(Path::new(&args[1]), &verification)?;
    if json.extension().and_then(|s| s.to_str()) != Some("json") {
        return Err("Expected a synthetic JSON catalog".into());
    }
    let marker = checked_path(&game.join("bin/win_x64/eurotrucks2.exe"), &verification)?;
    if fs::read(&marker).map_err(|e| e.to_string())? != b"synthetic placeholder; never executed" {
        return Err("Synthetic game marker missing; real installations are not accepted".into());
    }
    let archive = checked_path(&game.join("def.scs"), &verification)?;
    if hash(&fs::read(&archive).map_err(|e| e.to_string())?)
        != "f7160d049b6104616373346bfd9fbf01d78a9f2a43985ef9a78521061e7ed9a8"
    {
        return Err("Expected the checked-in synthetic onboarding archive".into());
    }
    let mut fixture: catalog::Catalog =
        serde_json::from_slice(&fs::read(&json).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    if fixture.signature != "synthetic"
        || fixture.definitions.is_empty()
        || fixture
            .definitions
            .values()
            .any(|d| !d.source.starts_with("synthetic"))
    {
        return Err("Only explicitly synthetic catalogs can be bound".into());
    }
    let meta = archive.metadata().map_err(|e| e.to_string())?;
    fixture.archives = vec![(
        archive.to_string_lossy().into(),
        meta.len(),
        meta.modified()
            .map_err(|e| e.to_string())?
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_secs(),
    )];
    fixture.name_schema = 1;
    fixture.parser_version = catalog::BUILD_CACHE_VERSION;
    fixture.game_path = game.to_string_lossy().into();
    fixture.source_fingerprint = catalog::current_source_fingerprint(&game)?;
    fixture.scan_complete = true;
    fixture.fresh()?;
    fs::write(
        json,
        serde_json::to_vec_pretty(&fixture).map_err(|e| e.to_string())?,
    )
    .map_err(|e| e.to_string())?;
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
