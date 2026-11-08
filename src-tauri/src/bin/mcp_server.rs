fn main() {
    // `--window-guard <prev_fg> <park> <keep_focus> <fit>`: the detached helper a launch starts to
    // look after the game window (it outlives a short-lived server).
    let args: Vec<String> = std::env::args().collect();
    if args.get(1).map(|a| a == "--window-guard").unwrap_or(false) {
        let prev = args.get(2).and_then(|v| v.parse::<isize>().ok()).unwrap_or(0);
        let flag = |i: usize| args.get(i).map(|v| v == "1").unwrap_or(false);
        pz_mod_studio_lib::claude_agent::run_window_guard(prev, flag(3), flag(4), flag(5));
        return;
    }
    // Run standalone Project Zomboid Mod Studio MCP Server over stdio with an 8MB stack
    let builder = std::thread::Builder::new().name("pz-mcp-server-main".into()).stack_size(8 * 1024 * 1024);
    let handle = builder
        .spawn(|| {
            pz_mod_studio_lib::mcp::run_stdio_server();
        })
        .expect("Failed to spawn MCP server thread");
    let _ = handle.join();
}

