fn main() {
    #[cfg(target_os = "windows")]
    {
        let mut winres = winres::WindowsResource::new();
        winres.set_icon("assets/icons/icon.ico");
        winres.compile().unwrap();
    }
}
