fn main() {
    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let mut resource = winresource::WindowsResource::new();
        resource
            .set_icon("../assets/icons/icon.ico")
            .set("ProductName", "Font Converter")
            .set("FileDescription", "Native TTF / OTF / WOFF2 converter");
        resource
            .compile()
            .expect("Compile Windows icon and version resources");
    }
    println!("cargo:rerun-if-changed=../assets/icons/icon.ico");
}
