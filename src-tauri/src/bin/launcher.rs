#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
use workshop_core::{runtime, Result};
fn run() -> Result<()> {
    let exe = std::env::current_exe().map_err(|e| e.to_string())?;
    let app = exe
        .parent()
        .ok_or("无法读取程序目录")?
        .join("workshop-app.exe");
    if !app.is_file() {
        return Err(
            "发布包不完整：缺少 workshop-app.exe。请完整解压 ZIP 后启动 ETS2 Workshop.exe。".into(),
        );
    }
    runtime::ensure()?;
    std::process::Command::new(app)
        .args(std::env::args_os().skip(1))
        .spawn()
        .map_err(|e| e.to_string())?;
    Ok(())
}
fn main() {
    if let Err(e) = run() {
        runtime::message(&e, false);
        std::process::exit(1);
    }
}
