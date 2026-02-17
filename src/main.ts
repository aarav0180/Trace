// ─────────────────────────────────────────────
// TRACE — Frontend Logic
// The Intelligence Layer for your OS
// ─────────────────────────────────────────────

import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow, LogicalSize } from "@tauri-apps/api/window";

// ─── Types ───────────────────────────────────

interface SearchResult {
  name: string;
  path: string;
  kind: "File" | "Directory" | "App";
  score: number;
  matched_indices: number[];
}

interface ShellTranslation {
  command: string;
  is_dangerous: boolean;
}

interface ShellOutput {
  stdout: string;
  stderr: string;
  exit_code: number;
}

interface Settings {
  openai_key: string | null;
  anthropic_key: string | null;
  google_key: string | null;
  active_provider: string;
  active_model: string;
  index_roots: string[];
  max_results: number;
}

// ─── State ───────────────────────────────────

type AppMode = "search" | "shell" | "chat";

let mode: AppMode = "search";
let results: SearchResult[] = [];
let selectedIndex = 0;
let searchTimeout: ReturnType<typeof setTimeout> | null = null;

// ─── DOM Elements ────────────────────────────

const searchInput = document.getElementById("search-input") as HTMLInputElement;
const modeIndicator = document.getElementById("mode-indicator") as HTMLElement;
const resultsContainer = document.getElementById("results-container") as HTMLElement;
const resultsList = document.getElementById("results-list") as HTMLElement;
const shellPanel = document.getElementById("shell-panel") as HTMLElement;
const shellCommand = document.getElementById("shell-command") as HTMLElement;
const shellWarning = document.getElementById("shell-warning") as HTMLElement;
const shellRun = document.getElementById("shell-run") as HTMLElement;
const shellCancel = document.getElementById("shell-cancel") as HTMLElement;
const shellOutput = document.getElementById("shell-output") as HTMLElement;
const chatPanel = document.getElementById("chat-panel") as HTMLElement;
const chatFilename = document.getElementById("chat-filename") as HTMLElement;
const chatClose = document.getElementById("chat-close") as HTMLElement;
const chatMessages = document.getElementById("chat-messages") as HTMLElement;
const chatLoading = document.getElementById("chat-loading") as HTMLElement;
const settingsBtn = document.getElementById("settings-btn") as HTMLElement;
const settingsOverlay = document.getElementById("settings-overlay") as HTMLElement;
const settingsSave = document.getElementById("settings-save") as HTMLElement;
const settingsCloseBtn = document.getElementById("settings-close") as HTMLElement;

// ─── Icon Helper ─────────────────────────────

function getIcon(kind: string): string {
  switch (kind) {
    case "App": return "◆";
    case "Directory": return "▸";
    default: return "○";
  }
}

// ─── Render Results ──────────────────────────

function renderResults() {
  resultsList.innerHTML = "";

  if (results.length === 0) {
    resultsContainer.classList.remove("expanded");
    resizeWindow(64);
    return;
  }

  results.forEach((r, i) => {
    const li = document.createElement("li");
    li.className = `result-item${i === selectedIndex ? " selected" : ""}`;
    li.dataset.index = String(i);

    // Build name with matched character highlights
    let nameHtml = "";
    const matchSet = new Set(r.matched_indices);
    for (let c = 0; c < r.name.length; c++) {
      if (matchSet.has(c)) {
        nameHtml += `<span class="match">${escHtml(r.name[c])}</span>`;
      } else {
        nameHtml += escHtml(r.name[c]);
      }
    }

    // Shorten path for display
    const displayPath = r.path.replace(/^\/home\/[^/]+/, "~");

    li.innerHTML = `
      <div class="result-icon">${getIcon(r.kind)}</div>
      <div class="result-info">
        <div class="result-name">${nameHtml}</div>
        <div class="result-path">${escHtml(displayPath)}</div>
      </div>
      <span class="result-kind">${r.kind}</span>
      <span class="result-tab-hint">Tab to chat</span>
    `;

    li.addEventListener("click", () => openResult(i));
    resultsList.appendChild(li);
  });

  resultsContainer.classList.add("expanded");

  // Resize window to fit results
  const contentHeight = 64 + Math.min(results.length * 50, 420) + 12;
  resizeWindow(contentHeight);
}

function escHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// ─── Window Resize ───────────────────────────

async function resizeWindow(height: number) {
  const appWindow = getCurrentWindow();
  try {
    const currentSize = await appWindow.outerSize();
    await appWindow.setSize(new LogicalSize(currentSize.width, Math.max(64, Math.round(height))));
  } catch (e) {
    // Ignore resize errors during init
  }
}

// ─── Search Logic ────────────────────────────

async function doSearch(query: string) {
  if (!query || query.startsWith(">") || query.startsWith("?")) return;

  try {
    results = await invoke<SearchResult[]>("search_files", { query });
    selectedIndex = 0;
    renderResults();
  } catch (e) {
    console.error("[trace] Search error:", e);
  }
}

// ─── Open / Launch ───────────────────────────

async function openResult(index: number) {
  const r = results[index];
  if (!r) return;

  try {
    await invoke("open_result", { path: r.path, kind: r.kind });
    // Hide window after opening
    const appWindow = getCurrentWindow();
    await appWindow.hide();
  } catch (e) {
    console.error("[trace] Open error:", e);
  }
}

// ─── Shell Mode (NLP-to-Bash) ────────────────

async function enterShellMode(input: string) {
  mode = "shell";
  modeIndicator.textContent = "COMMAND";
  modeIndicator.classList.add("visible");
  resultsContainer.classList.remove("expanded");
  shellPanel.classList.remove("hidden");
  shellCommand.textContent = "Thinking...";
  shellWarning.classList.add("hidden");
  shellOutput.classList.add("hidden");

  resizeWindow(220);

  try {
    const translation = await invoke<ShellTranslation>("translate_command", { input });
    shellCommand.textContent = translation.command;
    if (translation.is_dangerous) {
      shellWarning.classList.remove("hidden");
      resizeWindow(260);
    }
  } catch (e: any) {
    shellCommand.textContent = `Error: ${e}`;
  }
}

async function runShellCommand() {
  const cmd = shellCommand.textContent || "";
  if (!cmd || cmd.startsWith("Error:") || cmd === "Thinking...") return;

  shellOutput.textContent = "Running...";
  shellOutput.classList.remove("hidden");
  resizeWindow(340);

  try {
    const output = await invoke<ShellOutput>("execute_shell", { command: cmd });
    let text = "";
    if (output.stdout) text += output.stdout;
    if (output.stderr) text += (text ? "\n" : "") + output.stderr;
    if (!text) text = `Exit code: ${output.exit_code}`;
    shellOutput.textContent = text;
  } catch (e: any) {
    shellOutput.textContent = `Error: ${e}`;
  }
}

function exitShellMode() {
  mode = "search";
  modeIndicator.classList.remove("visible");
  shellPanel.classList.add("hidden");
  shellOutput.classList.add("hidden");
  searchInput.value = "";
  resizeWindow(64);
}

// ─── Chat Mode ───────────────────────────────

async function enterChatMode(index: number) {
  const r = results[index];
  if (!r || r.kind === "App") return;

  try {
    const preview = await invoke<string>("enter_chat_mode", { path: r.path });
    mode = "chat";
    const fileName = r.path.split("/").pop() || r.name;
    chatFilename.textContent = fileName;
    modeIndicator.textContent = "CHAT";
    modeIndicator.classList.add("visible");
    resultsContainer.classList.remove("expanded");
    chatPanel.classList.remove("hidden");
    chatMessages.innerHTML = "";

    // Show file preview as first assistant message
    addChatMessage("assistant", preview);

    searchInput.value = "";
    searchInput.placeholder = `Ask about ${fileName}...`;

    resizeWindow(440);
  } catch (e: any) {
    console.error("[trace] Chat mode error:", e);
  }
}

async function sendChatMessage(question: string) {
  if (!question.trim()) return;

  addChatMessage("user", question);
  searchInput.value = "";
  chatLoading.classList.remove("hidden");

  try {
    const answer = await invoke<string>("chat_message", { question });
    addChatMessage("assistant", answer);
  } catch (e: any) {
    addChatMessage("assistant", `Error: ${e}`);
  } finally {
    chatLoading.classList.add("hidden");
  }
}

function addChatMessage(role: "user" | "assistant", text: string) {
  const div = document.createElement("div");
  div.className = `chat-msg ${role}`;
  div.textContent = text;
  chatMessages.appendChild(div);
  chatMessages.scrollTop = chatMessages.scrollHeight;
}

async function exitChatMode() {
  mode = "search";
  modeIndicator.classList.remove("visible");
  chatPanel.classList.add("hidden");
  chatMessages.innerHTML = "";
  searchInput.value = "";
  searchInput.placeholder = "Search files, apps, or type > for commands...";
  results = [];
  resizeWindow(64);

  try {
    await invoke("exit_chat_mode");
  } catch (_) {}
}

// ─── Settings ────────────────────────────────

async function openSettings() {
  settingsOverlay.classList.remove("hidden");
  resizeWindow(600);

  try {
    const s = await invoke<Settings>("get_settings");
    (document.getElementById("setting-provider") as HTMLSelectElement).value = s.active_provider;
    (document.getElementById("setting-model") as HTMLSelectElement).value = s.active_model;
    (document.getElementById("setting-openai") as HTMLInputElement).value = s.openai_key || "";
    (document.getElementById("setting-anthropic") as HTMLInputElement).value = s.anthropic_key || "";
    (document.getElementById("setting-google") as HTMLInputElement).value = s.google_key || "";
  } catch (e) {
    console.error("[trace] Settings load error:", e);
  }
}

async function saveSettings() {
  const newSettings: Settings = {
    active_provider: (document.getElementById("setting-provider") as HTMLSelectElement).value,
    active_model: (document.getElementById("setting-model") as HTMLSelectElement).value,
    openai_key: (document.getElementById("setting-openai") as HTMLInputElement).value || null,
    anthropic_key: (document.getElementById("setting-anthropic") as HTMLInputElement).value || null,
    google_key: (document.getElementById("setting-google") as HTMLInputElement).value || null,
    index_roots: ["~"], // default
    max_results: 20,
  };

  try {
    await invoke("save_settings", { newSettings });
    closeSettings();
  } catch (e) {
    console.error("[trace] Settings save error:", e);
  }
}

function closeSettings() {
  settingsOverlay.classList.add("hidden");
  resizeWindow(mode === "search" ? 64 : 440);
}

// ─── Event Wiring ────────────────────────────

// Input handler with debounce
searchInput.addEventListener("input", () => {
  const val = searchInput.value;

  if (searchTimeout) clearTimeout(searchTimeout);

  if (mode === "chat") return; // Don't search in chat mode

  if (val.startsWith(">")) {
    // Shell mode prefix detected — wait for Enter
    modeIndicator.textContent = "COMMAND";
    modeIndicator.classList.add("visible");
    resultsContainer.classList.remove("expanded");
    return;
  }

  if (val === "") {
    results = [];
    selectedIndex = 0;
    renderResults();
    modeIndicator.classList.remove("visible");
    return;
  }

  // Debounced search (30ms for near-instant feel)
  searchTimeout = setTimeout(() => doSearch(val), 30);
});

// Keyboard navigation
searchInput.addEventListener("keydown", (e: KeyboardEvent) => {
  switch (e.key) {
    case "ArrowDown":
      e.preventDefault();
      if (mode === "search" && results.length > 0) {
        selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
        renderResults();
      }
      break;

    case "ArrowUp":
      e.preventDefault();
      if (mode === "search" && results.length > 0) {
        selectedIndex = Math.max(selectedIndex - 1, 0);
        renderResults();
      }
      break;

    case "Enter":
      e.preventDefault();
      if (mode === "search") {
        const val = searchInput.value;
        if (val.startsWith(">")) {
          enterShellMode(val.slice(1).trim());
        } else if (results.length > 0) {
          openResult(selectedIndex);
        }
      } else if (mode === "chat") {
        sendChatMessage(searchInput.value);
      }
      break;

    case "Tab":
      e.preventDefault();
      if (mode === "search" && results.length > 0) {
        enterChatMode(selectedIndex);
      }
      break;

    case "Escape":
      e.preventDefault();
      if (mode === "chat") {
        exitChatMode();
      } else if (mode === "shell") {
        exitShellMode();
      } else {
        // Hide window
        getCurrentWindow().hide();
      }
      break;
  }
});

// Shell panel buttons
shellRun.addEventListener("click", runShellCommand);
shellCancel.addEventListener("click", exitShellMode);

// Chat close button
chatClose.addEventListener("click", exitChatMode);

// Settings buttons
settingsBtn.addEventListener("click", openSettings);
settingsSave.addEventListener("click", saveSettings);
settingsCloseBtn.addEventListener("click", closeSettings);

// ─── Init ────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  searchInput.focus();
  resizeWindow(64);
});

// Keep focus on search input
window.addEventListener("focus", () => searchInput.focus());
