fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../crates/studio-ui/assets/icons/page.ico");
        res.compile().expect("icône Windows");
    }
}
