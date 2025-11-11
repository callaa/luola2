use winresource::WindowsResource;

fn main() -> std::io::Result<()> {
    if std::env::var("CARGO_CFG_WINDOWS").is_ok() {
        WindowsResource::new()
            .set_icon("pkg/luola2.ico")
            .compile()?;
    }
    Ok(())
}
