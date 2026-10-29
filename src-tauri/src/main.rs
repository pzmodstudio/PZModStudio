// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.iter().any(|a| a == "--mcp" || a == "--mcp-server" || a == "mcp") {
        // The same 8 MB stack as pz-mcp-server.exe: the main thread's 1 MB is not enough for the
        // Lua parser on a deeply nested file.
        let server = std::thread::Builder::new()
            .name("pz-mcp-server-main".into())
            .stack_size(8 * 1024 * 1024)
            .spawn(pz_mod_studio_lib::mcp::run_stdio_server)
            .expect("Failed to spawn MCP server thread");
        let _ = server.join();
        return;
    }
    // Steam's API may only be started once in a process, so the window asks a copy of itself to
    // do the talking and exit: --steam-subscribe <game folder> <workshop id>...
    if let Some(at) = args.iter().position(|a| a.starts_with("--steam-")) {
        std::process::exit(steam_cli(&args[at..]));
    }

    pz_mod_studio_lib::run()
}

/// Answers on stdout as JSON, or on stderr with the reason; 0 when it worked.
fn steam_cli(args: &[String]) -> i32 {
    use pz_mod_studio_lib::steam_client;
    let folder = args.get(1).cloned().unwrap_or_default();
    let ids: Vec<String> = args[2.min(args.len())..].to_vec();
    let answer = match args[0].as_str() {
        "--steam-check" => steam_client::check(&folder).map(|n| n.to_string()),
        // Each batch on its own line as it comes, so the window can show servers while more answer.
        "--steam-servers" => steam_client::internet_servers(&folder, |batch| {
            use std::io::Write;
            println!("{}{}", pz_mod_studio_lib::STEAM_PART_PREFIX, serde_json::to_string(&batch).unwrap_or_default());
            let _ = std::io::stdout().flush();
        })
        .map(|n| n.to_string()),
        "--steam-subscribe" => steam_client::set_subscriptions(&folder, &ids, true).map(|o| serde_json::to_string(&o).unwrap_or_default()),
        "--steam-unsubscribe" => steam_client::set_subscriptions(&folder, &ids, false).map(|o| serde_json::to_string(&o).unwrap_or_default()),
        other => Err(format!("{} is not a Steam command.", other)),
    };
    // Steam's library writes its own lines to stdout, so the answer is marked to stand out.
    match answer {
        Ok(text) => {
            println!("{}{}", pz_mod_studio_lib::STEAM_ANSWER_PREFIX, text);
            0
        }
        Err(e) => {
            eprintln!("{}{}", pz_mod_studio_lib::STEAM_ANSWER_PREFIX, e);
            1
        }
    }
}
