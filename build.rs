#[cfg(windows)]
fn main() {
    let mut res = winres::WindowsResource::new();
    res.set_icon("assets/sensidoc_win.ico");
    res.set("ProductName", "SensiDoc");
    res.set("FileDescription", "SensiDoc - 信息审计与脱敏工具");
    res.set("LegalCopyright", "Copyright (c) 2026 SensiDoc Team");
    if let Err(e) = res.compile() {
        eprintln!("Failed to compile windows resource icon: {}", e);
    }
}

#[cfg(not(windows))]
fn main() {}
