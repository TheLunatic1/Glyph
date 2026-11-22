fn main() {
    #[cfg(windows)]
    {
        println!("cargo:rerun-if-changed=../../assets/logo.ico");
        let mut res = winresource::WindowsResource::new();
        res.set_icon("../../assets/logo.ico");
        res.set("ProductName", "Glyph MCP Server");
        res.set(
            "FileDescription",
            "Glyph Standalone MCP Stdio Server (36 Tools)",
        );
        res.set("CompanyName", "TheLunatic1 (Salman Toha)");
        res.compile()
            .expect("Failed to embed Windows icon resource into glyph-mcp");
    }
}
