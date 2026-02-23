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
  icon_path?: string | null;
  generic_name?: string | null;
}

interface CalcResult {
  expression: string;
  result: number;
  display: string;
  has_variable: boolean;
}

interface GraphPoint {
  x: number;
  y: number;
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
  huggingface_key: string | null;
  openrouter_key: string | null;
  active_provider: string;
  active_model: string;
  index_roots: string[];
  max_results: number;
}

// ─── Provider → Model Map ────────────────────

const PROVIDER_MODELS: Record<string, { value: string; label: string }[]> = {
  openai: [
    { value: "gpt-4o-mini", label: "GPT-4o Mini (Fast)" },
    { value: "gpt-4o", label: "GPT-4o (Smart)" },
    { value: "o3-mini", label: "o3 Mini (Reasoning)" },
  ],
  anthropic: [
    { value: "claude-sonnet-4-20250514", label: "Claude Sonnet (Smart)" },
    { value: "claude-haiku-4-20250414", label: "Claude Haiku (Fast)" },
  ],
  google: [
    { value: "gemini-2.0-flash", label: "Gemini Flash (Fast)" },
    { value: "gemini-1.5-pro", label: "Gemini Pro (Smart)" },
  ],
  huggingface: [
    { value: "mistralai/Mistral-7B-Instruct-v0.3", label: "Mistral 7B Instruct" },
    { value: "meta-llama/Meta-Llama-3.1-8B-Instruct", label: "Llama 3.1 8B" },
    { value: "microsoft/Phi-3-mini-4k-instruct", label: "Phi-3 Mini 4K" },
    { value: "Qwen/Qwen2.5-72B-Instruct", label: "Qwen 2.5 72B" },
  ],
  openrouter: [
    { value: "mistralai/mistral-7b-instruct", label: "Mistral 7B" },
    { value: "meta-llama/llama-3.1-8b-instruct", label: "Llama 3.1 8B" },
    { value: "google/gemma-2-9b-it", label: "Gemma 2 9B" },
    { value: "qwen/qwen-2.5-72b-instruct", label: "Qwen 2.5 72B" },
    { value: "deepseek/deepseek-chat-v3-0324", label: "DeepSeek V3" },
  ],
};

// ─── State ───────────────────────────────────

type AppMode = "search" | "shell" | "chat";

let mode: AppMode = "search";
let results: SearchResult[] = [];
let selectedIndex = 0;
let searchTimeout: ReturnType<typeof setTimeout> | null = null;
let currentCalcResult: CalcResult | null = null;

// Icon data-URI cache (icon_path → data:image/... string)
const iconCache = new Map<string, string>();

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
const graphCanvas = document.getElementById("graph-canvas") as HTMLCanvasElement;

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
  hideGraph();

  const hasMath = currentCalcResult !== null;
  const hasResults = results.length > 0;

  if (!hasMath && !hasResults) {
    resultsContainer.classList.remove("expanded");
    resizeWindow(BASE_HEIGHT);
    return;
  }

  // ── Math result row (always first) ─────────
  if (hasMath && currentCalcResult) {
    const li = document.createElement("li");
    li.className = "result-item math-result" + (selectedIndex === -1 ? " selected" : "");
    li.dataset.index = "-1";

    if (currentCalcResult.has_variable) {
      li.innerHTML = `
        <div class="result-icon math-icon">=</div>
        <div class="result-info">
          <div class="result-name">${escHtml(currentCalcResult.display)}</div>
          <div class="result-path">Press Enter to plot graph</div>
        </div>
        <span class="result-kind">GRAPH</span>
      `;
      li.addEventListener("click", () => showGraph(currentCalcResult!.expression));
    } else {
      li.innerHTML = `
        <div class="result-icon math-icon">=</div>
        <div class="result-info">
          <div class="result-name math-value">${escHtml(currentCalcResult.display)}</div>
          <div class="result-path">${escHtml(currentCalcResult.expression)}</div>
        </div>
        <span class="result-kind">CALC</span>
      `;
      li.addEventListener("click", () => {
        // Copy result to clipboard
        navigator.clipboard.writeText(currentCalcResult!.display).catch(() => {});
      });
    }

    resultsList.appendChild(li);
  }

  // ── File / App result rows ─────────────────
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
    const displayPath = r.path
      .replace(/^\/home\/[^/]+/, "~")
      .replace(/^[A-Za-z]:\\Users\\[^\\]+/, "~");

    // Subtitle: generic name if available
    const subtitle = r.generic_name
      ? `${escHtml(r.generic_name)} — ${escHtml(displayPath)}`
      : escHtml(displayPath);

    // Icon: use a placeholder; real icon loaded async for App entries
    const iconId = `icon-${i}`;
    const iconHtml =
      r.kind === "App" && r.icon_path
        ? `<div class="result-icon" id="${iconId}"><img class="result-icon-img" src="" alt="" /></div>`
        : `<div class="result-icon">${getIcon(r.kind)}</div>`;

    li.innerHTML = `
      ${iconHtml}
      <div class="result-info">
        <div class="result-name">${nameHtml}</div>
        <div class="result-path">${subtitle}</div>
      </div>
      <span class="result-kind">${r.kind}</span>
      <span class="result-tab-hint">Tab to chat</span>
    `;

    li.addEventListener("click", () => openResult(i));
    resultsList.appendChild(li);

    // Load icon asynchronously for App entries
    if (r.kind === "App" && r.icon_path) {
      loadAppIcon(r.icon_path, iconId);
    }
  });

  resultsContainer.classList.add("expanded");

  const itemCount = results.length + (hasMath ? 1 : 0);
  const contentHeight = BASE_HEIGHT + Math.min(itemCount * 50, 680) + 12;
  resizeWindow(contentHeight);
}

// ─── App Icon Loader ─────────────────────────

async function loadAppIcon(iconPath: string, elementId: string) {
  // Check cache first
  let dataUri = iconCache.get(iconPath);
  if (!dataUri) {
    try {
      const result = await invoke<string | null>("get_app_icon", { path: iconPath });
      if (result) {
        dataUri = result;
        iconCache.set(iconPath, dataUri);
      }
    } catch {
      return; // icon load failed — keep text fallback
    }
  }

  if (!dataUri) return;

  const container = document.getElementById(elementId);
  if (!container) return;
  const img = container.querySelector(".result-icon-img") as HTMLImageElement | null;
  if (img) {
    img.src = dataUri;
  }
}

function escHtml(s: string): string {
  return s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// ─── Window Resize ───────────────────────────

const WINDOW_WIDTH = 700;
const BASE_HEIGHT = 72; // drag-region (8) + search-bar (56) + padding (8)

async function resizeWindow(height: number) {
  const appWindow = getCurrentWindow();
  try {
    await appWindow.setSize(new LogicalSize(WINDOW_WIDTH, Math.max(BASE_HEIGHT, Math.round(height))));
  } catch (e) {
    // Ignore resize errors during init
  }
}

// ─── Scroll selected result into view ────────

function scrollSelectedIntoView() {
  const selected = resultsContainer.querySelector(".result-item.selected") as HTMLElement | null;
  if (selected) {
    selected.scrollIntoView({ block: "nearest", behavior: "smooth" });
  }
}

// ─── Graph Rendering ─────────────────────────

async function showGraph(expression: string) {
  try {
    const points = await invoke<GraphPoint[]>("evaluate_graph", {
      query: expression,
      xMin: -10,
      xMax: 10,
      steps: 200,
    });

    if (points.length < 2) return;

    graphCanvas.classList.remove("hidden");
    drawGraph(points, expression);

    // Resize to fit graph
    const itemCount = results.length + 1; // +1 for math row
    const graphHeight = 220;
    const contentHeight = BASE_HEIGHT + Math.min(itemCount * 50, 300) + graphHeight + 24;
    resizeWindow(contentHeight);
  } catch (e) {
    console.error("[trace] Graph error:", e);
  }
}

function drawGraph(points: GraphPoint[], expression: string) {
  const canvas = graphCanvas;
  const dpr = window.devicePixelRatio || 1;
  const cssW = canvas.clientWidth || 660;
  const cssH = 200;
  canvas.width = cssW * dpr;
  canvas.height = cssH * dpr;
  canvas.style.height = `${cssH}px`;

  const ctx = canvas.getContext("2d")!;
  ctx.scale(dpr, dpr);

  const W = cssW;
  const H = cssH;
  const pad = 40;

  // Clear
  ctx.clearRect(0, 0, W, H);
  ctx.fillStyle = "#0a0a0a";
  ctx.fillRect(0, 0, W, H);

  // Bounds
  const xs = points.map((p) => p.x);
  const ys = points.map((p) => p.y);
  const xMin = Math.min(...xs);
  const xMax = Math.max(...xs);
  let yMin = Math.min(...ys);
  let yMax = Math.max(...ys);
  if (yMin === yMax) { yMin -= 1; yMax += 1; }

  const xRange = xMax - xMin || 1;
  const yRange = yMax - yMin || 1;

  const toX = (x: number) => pad + ((x - xMin) / xRange) * (W - 2 * pad);
  const toY = (y: number) => H - pad - ((y - yMin) / yRange) * (H - 2 * pad);

  // Grid lines
  ctx.strokeStyle = "#1a1a1a";
  ctx.lineWidth = 1;
  for (let i = 0; i <= 4; i++) {
    const gy = pad + (i / 4) * (H - 2 * pad);
    ctx.beginPath(); ctx.moveTo(pad, gy); ctx.lineTo(W - pad, gy); ctx.stroke();
    const gx = pad + (i / 4) * (W - 2 * pad);
    ctx.beginPath(); ctx.moveTo(gx, pad); ctx.lineTo(gx, H - pad); ctx.stroke();
  }

  // Axes (if visible)
  ctx.strokeStyle = "#333";
  ctx.lineWidth = 1;
  if (xMin <= 0 && xMax >= 0) {
    const x0 = toX(0);
    ctx.beginPath(); ctx.moveTo(x0, pad); ctx.lineTo(x0, H - pad); ctx.stroke();
  }
  if (yMin <= 0 && yMax >= 0) {
    const y0 = toY(0);
    ctx.beginPath(); ctx.moveTo(pad, y0); ctx.lineTo(W - pad, y0); ctx.stroke();
  }

  // Plot curve
  ctx.strokeStyle = "#ffffff";
  ctx.lineWidth = 2;
  ctx.beginPath();
  let started = false;
  for (const p of points) {
    const sx = toX(p.x);
    const sy = toY(p.y);
    if (!started) { ctx.moveTo(sx, sy); started = true; } else { ctx.lineTo(sx, sy); }
  }
  ctx.stroke();

  // Labels
  ctx.fillStyle = "#555";
  ctx.font = "11px JetBrains Mono, monospace";
  ctx.textAlign = "center";
  ctx.fillText(xMin.toFixed(1), pad, H - 8);
  ctx.fillText(xMax.toFixed(1), W - pad, H - 8);
  ctx.textAlign = "right";
  ctx.fillText(yMax.toFixed(1), pad - 6, pad + 4);
  ctx.fillText(yMin.toFixed(1), pad - 6, H - pad + 4);

  // Title
  ctx.fillStyle = "#888";
  ctx.font = "12px Poppins, sans-serif";
  ctx.textAlign = "left";
  ctx.fillText(`y = ${expression}`, pad + 8, pad - 8);
}

function hideGraph() {
  graphCanvas.classList.add("hidden");
}

// ─── Search Logic ────────────────────────────

async function doSearch(query: string) {
  if (!query || query.startsWith(">") || query.startsWith("?")) return;

  try {
    // Run math evaluation and fuzzy search in parallel
    const [mathResult, searchResults] = await Promise.all([
      invoke<CalcResult | null>("evaluate_math", { query }).catch(() => null),
      invoke<SearchResult[]>("search_files", { query }),
    ]);

    currentCalcResult = mathResult ?? null;
    results = searchResults;
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
  resizeWindow(BASE_HEIGHT);
}

// ─── Chat Mode ───────────────────────────────

async function enterChatMode(index: number) {
  const r = results[index];
  if (!r || r.kind === "App") return;

  const fileName = r.path.split(/[\/\\]/).pop() || r.name;

  // Open the chat panel immediately so errors are visible
  mode = "chat";
  chatFilename.textContent = fileName;
  modeIndicator.textContent = "CHAT";
  modeIndicator.classList.add("visible");
  resultsContainer.classList.remove("expanded");
  chatPanel.classList.remove("hidden");
  chatMessages.innerHTML = "";
  searchInput.value = "";
  searchInput.placeholder = `Ask about ${fileName}...`;
  resizeWindow(440);

  try {
    const preview = await invoke<string>("enter_chat_mode", { path: r.path });
    addChatMessage("assistant", preview);
  } catch (e: any) {
    addChatMessage("assistant", `⚠️ Could not load file: ${e}`);
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
  if (role === "assistant") {
    div.innerHTML = renderMarkdown(text);
  } else {
    div.textContent = text;
  }
  chatMessages.appendChild(div);
  chatMessages.scrollTop = chatMessages.scrollHeight;
}

/** Convert a subset of Markdown to safe HTML for LLM response display.
 *  Uses a line-by-line state machine so code fences are never touched
 *  by inline formatting or paragraph conversion.
 */
function renderMarkdown(raw: string): string {
  const esc = (s: string) =>
    s.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");

  /** Apply inline formatting to an already-HTML-escaped string. */
  function inline(s: string): string {
    // Inline code (must come before bold/italic so backticks win)
    s = s.replace(/`([^`]+)`/g, "<code>$1</code>");
    // Bold
    s = s.replace(/\*\*(.+?)\*\*/g, "<strong>$1</strong>");
    // Italic — avoid matching inside bold markers
    s = s.replace(/\*([^*\n]+?)\*/g, "<em>$1</em>");
    s = s.replace(/_([^_\n]+?)_/g, "<em>$1</em>");
    return s;
  }

  const lines = raw.split("\n");
  const out: string[] = [];
  let inFence = false;
  let fenceLines: string[] = [];
  let listOpen: "ul" | "ol" | null = null;

  function closeList() {
    if (listOpen) {
      out.push(listOpen === "ol" ? "</ol>" : "</ul>");
      listOpen = null;
    }
  }

  for (const line of lines) {
    // ── fenced code block ──────────────────────────────────────────
    if (!inFence && /^[ \t]*```/.test(line)) {
      closeList();
      inFence = true;
      fenceLines = [];
      continue;
    }
    if (inFence) {
      if (/^[ \t]*```/.test(line)) {
        out.push(`<pre><code>${fenceLines.join("\n")}</code></pre>`);
        fenceLines = [];
        inFence = false;
      } else {
        fenceLines.push(esc(line));
      }
      continue;
    }

    // ── blank line ─────────────────────────────────────────────────
    if (line.trim() === "") {
      closeList();
      out.push(`<div class="md-spacer"></div>`);
      continue;
    }

    const e = esc(line);

    // ── ATX headers ───────────────────────────────────────────────
    const h = e.match(/^(#{1,3}) (.+)$/);
    if (h) {
      closeList();
      const tag = h[1].length === 1 ? "h4" : "h5";
      out.push(`<${tag}>${inline(h[2])}</${tag}>`);
      continue;
    }

    // ── unordered list ────────────────────────────────────────────
    const ul = e.match(/^[ \t]*[-*] (.+)$/);
    if (ul) {
      if (listOpen !== "ul") { closeList(); out.push("<ul>"); listOpen = "ul"; }
      out.push(`<li>${inline(ul[1])}</li>`);
      continue;
    }

    // ── ordered list ─────────────────────────────────────────────
    const ol = e.match(/^[ \t]*\d+[.)]\s+(.+)$/);
    if (ol) {
      if (listOpen !== "ol") { closeList(); out.push("<ol>"); listOpen = "ol"; }
      out.push(`<li>${inline(ol[1])}</li>`);
      continue;
    }

    // ── regular paragraph line ───────────────────────────────────
    closeList();
    out.push(`<p>${inline(e)}</p>`);
  }

  closeList();
  // Close any unclosed fence (malformed input)
  if (inFence && fenceLines.length) {
    out.push(`<pre><code>${fenceLines.join("\n")}</code></pre>`);
  }

  return out.join("");
}

async function exitChatMode() {
  mode = "search";
  modeIndicator.classList.remove("visible");
  chatPanel.classList.add("hidden");
  chatMessages.innerHTML = "";
  searchInput.value = "";
  searchInput.placeholder = "Search files, apps, or type > for commands...";
  results = [];
  resizeWindow(BASE_HEIGHT);

  try {
    await invoke("exit_chat_mode");
  } catch (_) {}
}

// ─── Settings ────────────────────────────────

function populateModels(provider: string, currentModel?: string) {
  const modelSelect = document.getElementById("setting-model") as HTMLSelectElement;
  modelSelect.innerHTML = "";
  const models = PROVIDER_MODELS[provider] || [];
  models.forEach((m) => {
    const opt = document.createElement("option");
    opt.value = m.value;
    opt.textContent = m.label;
    modelSelect.appendChild(opt);
  });
  // Restore selection if the model exists in this provider
  if (currentModel && models.some((m) => m.value === currentModel)) {
    modelSelect.value = currentModel;
  } else if (models.length > 0) {
    modelSelect.value = models[0].value;
  }
}

async function openSettings() {
  settingsOverlay.classList.remove("hidden");
  resizeWindow(600);

  try {
    const s = await invoke<Settings>("get_settings");
    const providerSelect = document.getElementById("setting-provider") as HTMLSelectElement;
    providerSelect.value = s.active_provider;
    populateModels(s.active_provider, s.active_model);
    (document.getElementById("setting-openai") as HTMLInputElement).value = s.openai_key || "";
    (document.getElementById("setting-anthropic") as HTMLInputElement).value = s.anthropic_key || "";
    (document.getElementById("setting-google") as HTMLInputElement).value = s.google_key || "";
    (document.getElementById("setting-huggingface") as HTMLInputElement).value = s.huggingface_key || "";
    (document.getElementById("setting-openrouter") as HTMLInputElement).value = s.openrouter_key || "";
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
    huggingface_key: (document.getElementById("setting-huggingface") as HTMLInputElement).value || null,
    openrouter_key: (document.getElementById("setting-openrouter") as HTMLInputElement).value || null,
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
  resizeWindow(mode === "search" ? BASE_HEIGHT : 440);
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
    currentCalcResult = null;
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
      if (mode === "search" && (results.length > 0 || currentCalcResult)) {
        selectedIndex = Math.min(selectedIndex + 1, results.length - 1);
        renderResults();
        scrollSelectedIntoView();
      }
      break;

    case "ArrowUp":
      e.preventDefault();
      if (mode === "search" && (results.length > 0 || currentCalcResult)) {
        const minIdx = currentCalcResult ? -1 : 0;
        selectedIndex = Math.max(selectedIndex - 1, minIdx);
        renderResults();
        scrollSelectedIntoView();
      }
      break;

    case "Enter":
      e.preventDefault();
      if (mode === "search") {
        const val = searchInput.value;
        if (val.startsWith(">")) {
          enterShellMode(val.slice(1).trim());
        } else if (currentCalcResult?.has_variable && selectedIndex === -1) {
          // Graph mode: plot the equation
          showGraph(currentCalcResult.expression);
        } else if (currentCalcResult && !currentCalcResult.has_variable && selectedIndex === -1) {
          // Copy calc result to clipboard
          navigator.clipboard.writeText(currentCalcResult.display).catch(() => {});
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

// Provider change → update available models
document.getElementById("setting-provider")!.addEventListener("change", (e) => {
  const provider = (e.target as HTMLSelectElement).value;
  populateModels(provider);
});

// ─── Init ────────────────────────────────────

document.addEventListener("DOMContentLoaded", () => {
  searchInput.focus();
  resizeWindow(BASE_HEIGHT);

  // Show which system shortcut was registered
  showRegisteredShortcut();
});

async function showRegisteredShortcut() {
  try {
    const shortcut = await invoke<string>("get_registered_shortcut");
    if (!shortcut || shortcut === "(none)") return;

    const toast = document.getElementById("shortcut-toast");
    if (!toast) return;

    // Build key badges: "Super+T" → <span>Super</span> + <span>T</span>
    const keys = shortcut.split("+").map(
      (k) => `<span class="shortcut-key">${escHtml(k.trim())}</span>`
    );
    toast.innerHTML = `Press ${keys.join(" + ")} to launch Trace from anywhere`;

    // Slide in, hold, slide out
    requestAnimationFrame(() => {
      toast.classList.add("visible");
      setTimeout(() => toast.classList.remove("visible"), 4000);
    });
  } catch {
    // No shortcut registered — silently skip
  }
}

// Keep focus on search input
window.addEventListener("focus", () => searchInput.focus());
