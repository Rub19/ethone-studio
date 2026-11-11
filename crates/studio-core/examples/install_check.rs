fn main() {
    let catalog = match studio_core::Catalog::fetch() {
        Ok(c) => c,
        Err(e) => {
            println!("catalogue: ERREUR {e}");
            return;
        }
    };
    for app in &catalog.apps {
        println!("{} published={} version={:?} url={:?}", app.id, app.published(), app.version, app.url);
    }
    let Some(app) = catalog.apps.iter().find(|a| a.id == "sound") else { return };
    let last = std::cell::Cell::new(0u64);
    let r = studio_core::install(app, |d, t| {
        if d - last.get() > 2_000_000 || d == t {
            println!("{d}/{t}");
            last.set(d);
        }
    });
    println!("install: {r:?}");
    println!("exe: {} existe={}", studio_core::exe_path(&app.id).display(), studio_core::exe_path(&app.id).exists());
    println!("installed: {:?}", studio_core::Installed::load().apps);
}
