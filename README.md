<div align="center">

# TRACE

### The Intelligence Layer for Your OS

[![Built with Tauri](https://img.shields.io/badge/Built_with-Tauri_v2-1B1F23?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.70+-000000?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-000000?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Platform](https://img.shields.io/badge/Platform-Linux_|_Windows-000000?style=flat-square)](.)
[![License](https://img.shields.io/badge/License-MIT-000000?style=flat-square)](LICENSE)

**Instant file search. Natural language shell. AI-powered document chat.**
**All from a single floating command bar.**

<br/>

<img src="https://img.shields.io/badge/Super+T-Launch_Trace-111111?style=for-the-badge&labelColor=000000" alt="Super+T to Launch"/>

</div>

---

## Overview

Trace is a lightweight, blazing-fast desktop launcher and productivity tool that bridges your operating system to cloud AI. It appears instantly with a single hotkey, finds any file in under 50ms, launches apps, translates plain English into shell commands, and lets you chat with the contents of any file — all without leaving the keyboard.

Built entirely in **Rust** and **TypeScript**, powered by **Tauri v2**, with cross-platform support for **Linux** and **Windows**.

---

## Features

### ⚡ Instant File Search
- Multi-threaded filesystem indexer scans your home directory at startup
- Real-time file watcher (`inotify` on Linux, `ReadDirectoryChanges` on Windows) keeps the index in sync
- Fuzzy matching via Skim algorithm — results appear **as you type** (< 50ms)
- Matched characters are highlighted inline
- Results scroll to show all matches (up to 20)

### 🚀 Universal App Launcher
- **Linux**: Auto-detects installed applications from `.desktop` files (`/usr/share/applications`, `~/.local/share/applications`, Flatpak, Snap)
- **Windows**: Scans Start Menu `.lnk` shortcuts for installed programs
- Apps and files are unified in a single search — type and hit Enter

### ⌨️ System Shortcut Registration
- On first launch, Trace registers a **system-level keyboard shortcut** so you can summon it from anywhere
- Automatically detects your desktop environment and picks the best free key:
  - **GNOME / Ubuntu / Pop!\_OS / Cinnamon**: `gsettings` custom keybinding (scans for first free `Super+KEY`)
  - **KDE Plasma**: `kwriteconfig5` / `kwriteconfig6` global shortcut
  - **i3**: Appends `bindsym $mod+KEY` to config, live-reloads
  - **Sway**: Appends `bindsym $mod+KEY` to config, live-reloads
  - **Hyprland**: Appends `bind = $mainMod, KEY` to config (hot-reloads automatically)
  - **XFCE**: `xfconf-query` keyboard shortcut
  - **Windows**: Start Menu `.lnk` with `Ctrl+Alt+T` hotkey
- Key preference order: `T → F → J → Y → K → G → B → N` (first available wins)
- A toast notification shows the registered shortcut on startup
- If Trace is already running, pressing the shortcut **toggles** the window (show/hide) via single-instance detection

### 💬 Document Chat (RAG-Lite)
- Select any file in search results and press `Tab` to enter **Chat Mode**
- File contents are injected into the LLM context window
- Ask questions, request rewrites, get explanations — directly from the command bar
- Supports plain text, source code, Markdown, and config files

### 🖥️ Natural Language Shell
- Prefix any query with `>` to describe a command in plain English
- Trace translates it to a shell command via your configured LLM (bash on Linux, cmd on Windows)
- Dangerous commands (`rm -rf`, `del /s`, `mkfs`, `format`, etc.) are flagged with a warning
- Commands are **never auto-executed** — always shown for confirmation first

### 🔑 Bring Your Own Key (BYOK)
- Plug in your own API keys for **OpenAI**, **Anthropic**, **Google Gemini**, **HuggingFace**, or **OpenRouter**
- Switch between providers and models from the built-in settings panel
- Model dropdown filters to show only models available for the selected provider
- Choose between fast/cheap models and smart/expensive ones per task
- Open source models available via HuggingFace (Mistral, Llama, Phi, Qwen) and OpenRouter (DeepSeek, Gemma)

### 🎨 Noir UI
- Pure black (`#000000`) floating window with high-contrast white text
- Typography: **Playfair Display** (headings) + **Poppins** (interface)
- Frameless, draggable, resizable, always-on-top — feels native to the desktop
- Smooth CSS transitions on result expansion, no jank

---

## Keyboard Shortcuts

| Shortcut | Action |
| :--- | :--- |
| `Super + T`* | Toggle Trace window (registered system-wide) |
| `↑` / `↓` | Navigate search results (auto-scrolls into view) |
| `Enter` | Open file / launch app / send chat message / confirm command |
| `Tab` | Enter Chat Mode on selected file |
| `Escape` | Dismiss window / exit Chat Mode / cancel command |
| `>` prefix | Activate NLP-to-Shell mode |

> *\*The exact key depends on your DE. Trace picks the first available key from `Super+T/F/J/Y/K/G/B/N`. On Windows, `Ctrl+Alt+T` is used. The registered shortcut is shown as a toast notification on startup.*

---

## Tech Stack

| Layer | Technology | Purpose |
| :--- | :--- | :--- |
| **Backend** | Rust | File indexing, search, system integration, API dispatch |
| **Frontend** | TypeScript + HTML/CSS | UI rendering, keyboard navigation, state management |
| **Framework** | Tauri v2 | Windowing, IPC, single-instance, native shell access |
| **Search** | `fuzzy-matcher` (Skim) | Sub-50ms fuzzy matching with scored results |
| **Indexing** | `walkdir` + `notify` | Multi-threaded scan + real-time file watcher |
| **AI** | OpenAI / Anthropic / Google | NLP-to-Shell translation, document Q&A |
| **HTTP** | `reqwest` | Async HTTP client for cloud API communication |

---

## Project Structure

```
trace/
├── index.html                        # App shell
├── package.json                      # Frontend dependencies
├── tsconfig.json                     # TypeScript configuration
├── vite.config.ts                    # Vite bundler config
│
├── src/                              # ── Frontend ──
│   ├── main.ts                       # App logic, keyboard nav, Tauri IPC
│   └── styles.css                    # Noir theme, animations, layout
│
└── src-tauri/                        # ── Backend ──
    ├── Cargo.toml                    # Rust dependencies
    ├── tauri.conf.json               # Window config, permissions, build
    ├── capabilities/
    │   └── default.json              # Tauri v2 capability permissions
    └── src/
        ├── main.rs                   # Entry point
        ├── lib.rs                    # App bootstrap, plugin init, single-instance toggle
        ├── autostart.rs              # System shortcut registration (per DE/OS)
        ├── indexer.rs                # Multi-threaded filesystem scanner
        ├── watcher.rs                # Real-time file watcher (inotify / ReadDirectoryChanges)
        ├── search.rs                 # Fuzzy search engine
        ├── launcher.rs               # App discovery (.desktop on Linux, .lnk on Windows)
        ├── settings.rs               # BYOK settings (persisted to config dir)
        ├── llm.rs                    # Unified LLM client (OpenAI/Anthropic/Google)
        ├── shell_cmd.rs              # NLP-to-Shell translation & safe execution
        ├── doc_chat.rs               # Document chat (RAG-Lite)
        └── commands.rs               # Tauri command handlers (frontend ↔ backend)
```

---

## Cross-Platform Support

| Feature | Linux | Windows |
| :--- | :--- | :--- |
| File search | ✅ `walkdir` | ✅ `walkdir` |
| File watcher | ✅ `inotify` | ✅ `ReadDirectoryChanges` |
| App discovery | ✅ `.desktop` files | ✅ Start Menu `.lnk` |
| App launch | ✅ `Exec=` field parsing | ✅ `open::that` (follows `.lnk`) |
| System shortcut | ✅ Per-DE registration | ✅ `.lnk` hotkey (`Ctrl+Alt+T`) |
| Shell commands | ✅ `sh -c` | ✅ `cmd /C` |
| Settings path | `~/.config/trace/` | `%APPDATA%/trace/` |

---

## Getting Started

### Prerequisites

- **Rust** ≥ 1.70 — [Install](https://rustup.rs)
- **Node.js** ≥ 18 — [Install](https://nodejs.org)
- **System libraries** (Linux only):
  ```bash
  # Debian / Ubuntu
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

  # Arch Linux
  sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 librsvg
  ```

### Install & Run

```bash
# Clone the repository
git clone https://github.com/aarav/trace.git
cd trace

# Install frontend dependencies
npm install

# Run in development mode (hot-reload)
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
```

The compiled binary and installer will be in `src-tauri/target/release/bundle/`.

---

## Configuration

On first launch, Trace creates a settings file at:

```
~/.config/trace/settings.json
```

You can also configure everything from the UI by clicking the **⚙** icon in the search bar.

| Setting | Default | Description |
| :--- | :--- | :--- |
| `active_provider` | `"openai"` | Cloud AI provider (`openai`, `anthropic`, `google`) |
| `active_model` | `"gpt-4o-mini"` | Model to use for AI features |
| `openai_key` | `null` | Your OpenAI API key |
| `anthropic_key` | `null` | Your Anthropic API key |
| `google_key` | `null` | Your Google AI (Gemini) API key |
| `index_roots` | `["~"]` | Directories to index |
| `max_results` | `20` | Maximum search results displayed |

### Supported Models

| Provider | Fast / Cheap | Smart / Visual |
| :--- | :--- | :--- |
| **OpenAI** | `gpt-4o-mini` | `gpt-4o` |
| **Anthropic** | `claude-haiku-4-20250414` | `claude-sonnet-4-20250514` |
| **Google** | `gemini-2.0-flash` | `gemini-1.5-pro` |

---

## Usage

### File Search
Just start typing. Results appear instantly with matched characters highlighted. All results scroll into view.

### Launch an App
Type the app name → press `Enter`.
- **Linux**: Scans `/usr/share/applications`, `~/.local/share/applications`, Flatpak, and Snap directories
- **Windows**: Scans Start Menu shortcuts

### Chat with a File
1. Search for a file
2. Use `↑`/`↓` to select it
3. Press `Tab` to enter Chat Mode
4. Ask anything — *"Rewrite this to use async/await"*, *"Summarize this config"*, etc.
5. Press `Escape` to exit

### Natural Language Commands
1. Type `>` followed by plain English — e.g. `> kill all node processes on port 3000`
2. Trace shows the generated shell command for review
3. Press `▶ Run` to execute, or `✕ Cancel` to discard
4. Output streams back into the panel

### System Shortcut
On first launch, Trace registers a system-wide keyboard shortcut and shows a toast notification:

> *Press **Super + T** to launch Trace from anywhere*

If `Super+T` is already taken by your DE, Trace automatically picks the next free key. You can check which shortcut was assigned in `~/.config/trace/shortcut`.

---

## Roadmap

- [x] **Phase 1** — Instant file search, app launcher, Noir UI, cross-platform (Linux + Windows)
- [x] **Phase 3** — NLP-to-Shell, Document Chat
- [ ] **Phase 2** — Context-aware screen query (screenshot → AI), smart clipboard
- [ ] **Phase 4** — God Mode dashboard, deep-linking (`trace://`), Shift-preview

---

## Development

### IDE Setup

- [VS Code](https://code.visualstudio.com/)
- [Tauri Extension](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)

### Useful Commands

```bash
# Type-check frontend
npx tsc --noEmit

# Build Rust backend only
cargo build --manifest-path src-tauri/Cargo.toml

# Build frontend only
npx vite build

# Run full app in dev mode
npm run tauri dev
```

---

## License

This project is licensed under the [MIT License](LICENSE).

---

<div align="center">
<br/>

**Trace** — *Search instantly. Command naturally. Think faster.*

<br/>
</div>
