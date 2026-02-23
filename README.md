<div align="center">

<img src="src-tauri/icons/128x128@2x.png" width="96" alt="Trace icon" />

<h1>TRACE</h1>

<p><strong>The Intelligence Layer for Your OS</strong></p>

[![Built with Tauri](https://img.shields.io/badge/Built_with-Tauri_v2-24C8DB?style=flat-square&logo=tauri&logoColor=white)](https://tauri.app)
[![Rust](https://img.shields.io/badge/Rust-1.70+-CE422B?style=flat-square&logo=rust&logoColor=white)](https://www.rust-lang.org)
[![TypeScript](https://img.shields.io/badge/TypeScript-5.6-3178C6?style=flat-square&logo=typescript&logoColor=white)](https://www.typescriptlang.org)
[![Platform](https://img.shields.io/badge/Platform-Linux_|_Windows-4A90E2?style=flat-square&logo=linux&logoColor=white)](.)
[![License](https://img.shields.io/badge/License-MIT-22C55E?style=flat-square)](LICENSE)
[![Version](https://img.shields.io/badge/Version-0.3.0-8B5CF6?style=flat-square)](https://github.com/aarav0180/Trace/releases)

<br/>

> **Instant file search. Natural language shell. AI-powered document chat.**
> All from a single floating command bar — summoned with one keypress.

<br/>

<kbd>Super</kbd> + <kbd>F</kbd> → launch Trace from anywhere

</div>

---

## What is Trace?

Trace is a lightweight, keyboard-driven desktop launcher that bridges your operating system to cloud AI. It hovers over your desktop as a frameless floating bar, appears in under 100ms, and disappears the moment you're done.

- **Find any file** in milliseconds with fuzzy search
- **Launch any app** by typing its name
- **Chat with documents** — select a file, press Tab, ask anything
- **Translate plain English to shell commands** — type `>` and describe what you want
- **Evaluate math** inline — no calculator app needed
- Built entirely in **Rust + TypeScript** via **Tauri v2**

---

## Features

### ⚡ Instant File Search
Real-time fuzzy matching as you type — results in **< 50ms**. Multi-threaded indexer scans your home directory at startup; a live file watcher (`inotify` / `ReadDirectoryChanges`) keeps the index in sync with no polling. Matched characters are highlighted inline. Up to 20 results, scrollable.

### 🚀 App Launcher
Unified file + app search in a single bar.
- **Linux** — auto-discovers from `.desktop` files across `/usr/share/applications`, `~/.local/share/applications`, Flatpak, and Snap
- **Windows** — scans Start Menu `.lnk` shortcuts

### 💬 Document Chat  *(AI-powered)*
Select any file in results and press **Tab** to enter Chat Mode. File contents are injected into the LLM's context window. Ask questions, get summaries, request rewrites — all without opening another app. Supports source code, Markdown, config files, plain text, and **PDF** files.

### 🖥️ Natural Language → Shell
Type `>` and describe what you want in plain English. Trace translates it to a real shell command via your LLM. **Dangerous commands** (`rm -rf`, `mkfs`, `format`, etc.) are flagged with a warning. Commands are always shown for review — never auto-executed.

### 🧮 Inline Math
Type any expression (`2^10`, `sqrt(144)`, `sin(pi/4)`) and get an instant result without leaving the bar. Variables and equations open a graph panel automatically.

### 🔑 Bring Your Own Key (BYOK)
Plug in your API key for any supported provider. Switch models per-task from the built-in settings panel.

| Provider | Fast / Cheap | Smart |
| :--- | :--- | :--- |
| **OpenAI** | `gpt-4o-mini` | `gpt-4o` |
| **Anthropic** | `claude-haiku-4-20250414` | `claude-sonnet-4-20250514` |
| **Google** | `gemini-2.0-flash` | `gemini-1.5-pro` |
| **HuggingFace** | `mistralai/Mistral-7B-Instruct` | `Qwen/Qwen2.5-72B-Instruct` |
| **OpenRouter** | `google/gemma-3-4b-it:free` | `deepseek/deepseek-chat` |

### 🎨 Noir UI
Pure black (`#000000`) frameless window. **Playfair Display** headings, **Poppins** interface type. Always-on-top, draggable, resizable. Smooth transitions, no jank, feels native.

---

## Keyboard Shortcuts

| Key | Action |
| :--- | :--- |
| `Super + F` *(or auto-assigned)* | Toggle Trace window system-wide |
| `↑` / `↓` | Navigate results |
| `Enter` | Open file / launch app / send message / confirm command |
| `Tab` | Enter **Chat Mode** on the selected file |
| `Escape` | Exit chat / cancel command / hide window |
| `>` prefix | Activate **NLP → Shell** mode |

> The system shortcut is registered automatically on first launch. If `Super+F` is taken, Trace picks the next free key from `Super+J`, `Super+Y`, `Super+K` … and shows a toast notification with the result.

---

## Tech Stack

| Layer | Technology | Purpose |
| :--- | :--- | :--- |
| **Backend** | Rust | Indexing, search, IPC, API dispatch, PDF extraction |
| **Frontend** | TypeScript + HTML/CSS | UI, keyboard nav, markdown rendering |
| **Framework** | Tauri v2 | Windowing, IPC, single-instance, native shell |
| **Search** | `fuzzy-matcher` (Skim) | Sub-50ms fuzzy match with scored results |
| **Indexing** | `walkdir` + `notify` | Multi-threaded scan + real-time watcher |
| **PDF** | `pdf-extract` | Native Rust PDF text extraction |
| **AI** | OpenAI / Anthropic / Google / HF / OpenRouter | Shell translation, document Q&A |
| **HTTP** | `reqwest` | Async HTTP client for all cloud APIs |
| **Math** | `meval` | Expression evaluation + graphing |

---

## Project Structure

```
Trace/
├── index.html              # App shell
├── package.json
├── tsconfig.json
├── vite.config.ts
│
├── src/                    # ── Frontend ──────────────────────────────
│   ├── main.ts             # App logic, keyboard nav, markdown renderer
│   └── styles.css          # Noir theme, animations, chat styles
│
└── src-tauri/              # ── Backend ────────────────────────────────
    ├── Cargo.toml
    ├── tauri.conf.json
    └── src/
        ├── main.rs         # Entry point
        ├── lib.rs          # Bootstrap, plugins, single-instance toggle
        ├── commands.rs     # Tauri IPC command handlers
        ├── indexer.rs      # Multi-threaded filesystem scanner
        ├── watcher.rs      # Real-time file watcher
        ├── search.rs       # Fuzzy search engine
        ├── launcher.rs     # App discovery (.desktop / .lnk)
        ├── settings.rs     # BYOK settings (persisted to config dir)
        ├── llm.rs          # Unified LLM client (5 providers)
        ├── shell_cmd.rs    # NLP → Shell translation & safe execution
        └── doc_chat.rs     # Document chat — context trimming, PDF support
```

---

## Cross-Platform Support

| Feature | Linux | Windows |
| :--- | :---: | :---: |
| File search | ✅ | ✅ |
| Real-time file watcher | ✅ `inotify` | ✅ `ReadDirectoryChanges` |
| App discovery | ✅ `.desktop` | ✅ Start Menu `.lnk` |
| System shortcut | ✅ GNOME / KDE / i3 / Sway / Hyprland / XFCE | ✅ `Ctrl+Alt+T` |
| Shell commands | ✅ `sh -c` | ✅ `cmd /C` |
| PDF chat | ✅ | ✅ |
| Settings path | `~/.config/trace/` | `%APPDATA%\trace\` |

---

## Getting Started

### Prerequisites

- **Rust** ≥ 1.70 → [rustup.rs](https://rustup.rs)
- **Node.js** ≥ 18 → [nodejs.org](https://nodejs.org)
- **Linux system libraries:**
  ```bash
  # Debian / Ubuntu
  sudo apt install libwebkit2gtk-4.1-dev libappindicator3-dev librsvg2-dev patchelf

  # Arch Linux
  sudo pacman -S webkit2gtk-4.1 libappindicator-gtk3 librsvg
  ```

### Run in Development

```bash
git clone https://github.com/aarav0180/Trace.git
cd Trace
npm install
npm run tauri dev
```

### Build for Production

```bash
npm run tauri build
# → src-tauri/target/release/bundle/
```

---

## Configuration

Settings are stored at `~/.config/trace/settings.json` (Linux) or `%APPDATA%\trace\settings.json` (Windows).
Everything is also configurable from the **⚙** icon inside the app.

| Setting | Default | Description |
| :--- | :--- | :--- |
| `active_provider` | `"openai"` | AI provider (`openai`, `anthropic`, `google`, `huggingface`, `openrouter`) |
| `active_model` | `"gpt-4o-mini"` | Model identifier |
| `openai_key` | `""` | OpenAI API key |
| `anthropic_key` | `""` | Anthropic API key |
| `google_key` | `""` | Google AI (Gemini) API key |
| `huggingface_key` | `""` | HuggingFace Inference API key |
| `openrouter_key` | `""` | OpenRouter API key |
| `max_results` | `20` | Max search results shown |

---

## Usage

### Chat with a File
1. Search for any file (text, code, Markdown, PDF…)
2. Select it with `↑` / `↓`
3. Press **Tab**
4. Type your question and press **Enter**
5. Press **Escape** to return to search

### Natural Language Shell
1. Type `>` → describe what you want: `> find all files larger than 100MB`
2. Review the generated command
3. Press **▶ Run** or **✕ Cancel**

### Inline Math
Just type an expression: `sqrt(2) * pi` — the result appears instantly below the input.

---

## Roadmap

- [x] Instant file search + fuzzy matching
- [x] Universal app launcher (Linux + Windows)
- [x] Noir UI — frameless, always-on-top
- [x] System-wide hotkey registration (GNOME / KDE / i3 / Sway / Hyprland / XFCE / Windows)
- [x] NLP → Shell translation with safety guards
- [x] Document Chat (RAG-Lite) — text, code, Markdown
- [x] Inline math evaluator + graphing
- [x] BYOK: OpenAI, Anthropic, Google, HuggingFace, OpenRouter
- [x] PDF support in Document Chat
- [ ] Screenshot → AI context query
- [ ] Smart clipboard history
- [ ] `trace://` deep-link protocol
- [ ] Shift-preview (peek file contents without entering chat)

---

## Development

### Useful Commands

```bash
# Type-check frontend only
npx tsc --noEmit

# Build Rust backend only
cargo build --manifest-path src-tauri/Cargo.toml

# Full dev mode (hot-reload)
npm run tauri dev

# Production build
npm run tauri build
```

### Recommended VS Code Extensions

- [Tauri](https://marketplace.visualstudio.com/items?itemName=tauri-apps.tauri-vscode)
- [rust-analyzer](https://marketplace.visualstudio.com/items?itemName=rust-lang.rust-analyzer)
- [ESLint](https://marketplace.visualstudio.com/items?itemName=dbaeumer.vscode-eslint)

---

## License

MIT — see [LICENSE](LICENSE).

---

<div align="center">
<br/>

<img src="src-tauri/icons/32x32.png" width="20" alt="" />

**Trace** — *Search instantly. Command naturally. Think faster.*

<br/>

[![GitHub](https://img.shields.io/badge/github-aarav0180%2FTrace-181717?style=flat-square&logo=github)](https://github.com/aarav0180/Trace)

</div>
