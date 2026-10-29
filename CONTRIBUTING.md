# Contributing to Project Zomboid Mod Studio

Thank you for your interest in contributing to **Project Zomboid Mod Studio (PZ Mod Studio)**! 

PZ Mod Studio is a free, open-source community suite designed to solve mod conflicts, manage load orders, and diagnose runtime errors across Project Zomboid builds. We welcome contributions of all kinds: bug reports, polyfill rules, documentation, translations, and code improvements.

---

## 🛠️ Development Setup

### Prerequisites
Make sure you have the following installed on your machine:
- **Node.js**: v22 or newer ([nodejs.org](https://nodejs.org/))
- **Rust Toolchain**: Stable release ([rustup.rs](https://rustup.rs/))
- **Git**: ([git-scm.com](https://git-scm.com/))
- **Microsoft Edge WebView2**: Pre-installed on modern Windows 10/11.

### 1. Clone the Repository
```bash
git clone https://github.com/pzmodstudio/PZModStudio.git
cd PZModStudio
```

### 2. Install Frontend Dependencies
```bash
npm install
```

### 3. Run in Development Mode
Launch the application with live hot-reloading for both the React frontend and Tauri Rust backend:
```bash
npm run tauri dev
```

### 4. Build Standalone Binaries
To compile the standalone release:
```bash
# Build production frontend and compile portable bundle
npm run build:portable
```

To compile only the standalone MCP Server (faster, no GUI):
```bash
cargo build --release --bin pz-mcp-server --manifest-path "src-tauri/Cargo.toml"
```

---

## 📂 Project Architecture

```
Project-Zomboid-Mod-Studio/
├── src/                               # React 19 Frontend (TypeScript + Tailwind CSS 4)
│   ├── components/
│   │   ├── instances/                 # Profile and instance manager
│   │   ├── layout/                    # Header, Sidebar, Splash screen
│   │   ├── load_order/                # Mod List, dependency inspector, .pzpack presets
│   │   ├── merger/                    # 3-Way AST Merge engine & Monaco Diff Editor
│   │   ├── sandbox/                   # Monitor Center, live log viewer, crash cards
│   │   ├── server/                    # Dedicated server manager & live console
│   │   └── settings/                  # Path configuration & polyfill toggles
│   ├── services/tauri.ts              # Typed Rust IPC bridge bindings
│   └── types/index.ts                 # Shared TypeScript types & interfaces
├── src-tauri/                         # Rust Backend (Tauri 2.0)
│   ├── src/
│   │   ├── bin/mcp_server.rs          # Standalone MCP CLI binary
│   │   ├── diff_engine/               # Lua AST parser (full_moon) & diffing algorithms
│   │   ├── load_order/                # mod.info parser & topological sort (Kahn's)
│   │   ├── mcp/                       # Model Context Protocol (MCP 2024-11-05) tools
│   │   ├── patch_generator/           # Master Patch mod packaging engine
│   │   ├── sandbox/                   # Game process lifecycle & file-based IPC bridge
│   │   ├── server_manager/            # Dedicated server manager & RCON
│   │   └── vfs/                       # Virtual File System conflict detector
│   └── Cargo.toml
├── docs/                              # 9-Chapter Technical Knowledge Base on PZ internals
├── AGENTS.md                          # Master integration guide for AI agents & MCP
└── .github/workflows/release.yml      # CI/CD auto-release pipeline
```

---

## 🎯 How You Can Contribute

### 1. 🛡️ Polyfill Rules for Build 42 Compatibility
If you encounter a specific API break between B41 and B42 (e.g., changed function signatures, moved require paths, or uninitialized global tables), you can contribute a new rule in `src/data/default_rules.ts`.

### 2. 🔀 Merge Engine & AST Improvements
Help refine the 3-Way AST parser (`src-tauri/src/diff_engine/lua.rs`) to handle complex table structures and edge-case Lua constructs.

### 3. 🌐 Internationalization / Translations
Help translate the UI into other languages (Spanish, French, German, Russian, Portuguese, Chinese, etc.).

### 4. 🐧 Linux / Steam Deck Support
Help test and optimize path resolution and dedicated server tools on Linux and Proton.

---

## 🚀 Submitting a Pull Request (PR)

1. **Fork** the repository on GitHub.
2. Create a **feature branch**:
   ```bash
   git checkout -b feature/your-feature-name
   ```
3. Commit your changes with clear, descriptive commit messages:
   ```bash
   git commit -m "Add polyfill rule for B42 item container API"
   ```
4. Push your branch:
   ```bash
   git push origin feature/your-feature-name
   ```
5. Open a **Pull Request** on the `main` branch with a summary of changes and testing instructions.

---

## ⚖️ License
By contributing to Project Zomboid Mod Studio, you agree that your contributions will be licensed under the **MIT License**.
