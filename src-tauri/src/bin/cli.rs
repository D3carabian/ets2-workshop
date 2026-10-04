use std::path::Path;
use workshop_core::{catalog::Catalog, decoder, storage::Session};
fn run() -> workshop_core::Result<()> {
    let a: Vec<_> = std::env::args().collect();
    match a.get(1).map(String::as_str){
 Some("decode")=>{print!("{}",decoder::read(Path::new(&a[2]))?);Ok(())},
 Some("runtime-status")=>{println!("{}",workshop_core::runtime::installed());Ok(())},
 Some("prepare-extractor")=>{println!("{}",workshop_core::setup::ensure_extractor(&|s|eprintln!("{s}"))?.display());Ok(())},
 Some("detect")=>{println!("{}",serde_json::to_string_pretty(&workshop_core::setup::detect()).unwrap());Ok(())},
 Some("config")=>{println!("{}",serde_json::json!({"data_dir":workshop_core::storage::app_dir(),"settings":workshop_core::storage::settings()}));Ok(())},
 Some("--decode-worker")=>decoder::worker(Path::new(&a[2])),
 Some("inspect")=>{let s=workshop_core::storage::settings();let session=Session::open(Path::new(&a[2]),&s)?;let c:Catalog=if let Some(p)=a.get(3){serde_json::from_slice(&std::fs::read(p).map_err(|e|e.to_string())?).map_err(|e|e.to_string())?}else{Catalog::default()};println!("{}",serde_json::to_string_pretty(&session.view(&c)?).unwrap());Ok(())},
 Some("index")=>{let c=workshop_core::catalog::build(Path::new(&a[2]),Path::new(&a[3]),Path::new(&a[4]))?;std::fs::write(Path::new(&a[4]).join("catalog.json"),serde_json::to_vec(&c).unwrap()).map_err(|e|e.to_string())?;println!("Indexed {} definitions; {} warnings",c.definitions.len(),c.warnings.len());Ok(())},
 _=>Err("Usage: workshop-cli inspect game.sii [catalog.json] | index game-dir extractor.exe cache-dir".into())}
}
fn main() {
    if let Err(e) = run() {
        eprintln!("{e}");
        std::process::exit(1);
    }
}
