fn main() {
    // Run standalone Project Zomboid Mod Studio MCP Server over stdio with an 8MB stack
    let builder = std::thread::Builder::new().name("pz-mcp-server-main".into()).stack_size(8 * 1024 * 1024);
    let handle = builder
        .spawn(|| {
            pz_mod_studio_lib::mcp::run_stdio_server();
        })
        .expect("Failed to spawn MCP server thread");
    let _ = handle.join();
}

