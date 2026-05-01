import "./styles.css";
import { tauriClient } from "./tauriClient";
import type { EntryKind, WorkflowState } from "./types";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing #app root");
}

const appRoot = app;

const state: WorkflowState = {
  entry: null,
  currentTaskId: null,
  selectedRootPath: null,
  skippedCount: 0,
  errorCount: 0,
  errorMessage: null,
  files: [],
  classifications: [],
  plan: null,
  batch: null,
  status: "idle",
};

function formatRisk(risk: "low" | "medium" | "high"): string {
  return {
    low: "低风险",
    medium: "需留意",
    high: "高风险",
  }[risk];
}

function entryTitle(entry: EntryKind): string {
  return entry === "desktop" ? "桌面整理" : "文件整理";
}

function render(): void {
  if (!state.entry) {
    renderHome();
    return;
  }

  renderWorkspace(state.entry);
}

function renderHome(): void {
  appRoot.innerHTML = `
    <main class="home-window" aria-label="智能文件整理入口">
      <aside class="home-rail" aria-label="主导航">
        <div class="window-controls" aria-hidden="true">
          <span class="traffic red"></span>
          <span class="traffic yellow"></span>
          <span class="traffic green"></span>
        </div>
        <nav class="rail-nav" aria-hidden="true">
          <span class="rail-icon active folder-symbol"></span>
          <span class="rail-dot"></span>
          <span class="rail-icon monitor-symbol"></span>
          <span class="rail-dot"></span>
          <span class="rail-icon layers-symbol"></span>
          <span class="rail-dot"></span>
          <span class="rail-icon gear-symbol"></span>
        </nav>
      </aside>

      <section class="home-canvas">
        <header class="home-title">
          <div class="title-row">
            <img src="/visuals/title-logo.png" alt="" class="title-logo" />
            <h1>智能文件整理</h1>
          </div>
          <p>所有更改都会先生成方案，确认后才执行。</p>
        </header>

        <div class="entry-orbits">
          <article class="home-entry" role="button" tabindex="0" data-entry="files" aria-label="进入文件整理">
            <img src="/visuals/file-organizer-entry.png" alt="" class="entry-art" />
            <h2>文件整理</h2>
            <p>扫描并规划文件归类</p>
          </article>
          <article class="home-entry" role="button" tabindex="0" data-entry="desktop" aria-label="进入桌面整理">
            <img src="/visuals/desktop-organizer-entry.png" alt="" class="entry-art" />
            <h2>桌面整理</h2>
            <p>预览并整理桌面</p>
          </article>
        </div>
      </section>
    </main>
  `;
}

function renderWorkspace(entry: EntryKind): void {
  const isBusy =
    state.status === "selecting" ||
    state.status === "scanning" ||
    state.status === "classifying" ||
    state.status === "executing";

  appRoot.innerHTML = `
    <main class="shell workspace-shell">
      <nav class="topbar" aria-label="页面导航">
        <div>
          <p class="eyebrow">本地优先 · 用户确认后执行</p>
          <h1>${entryTitle(entry)}</h1>
        </div>
        <div class="nav-actions">
          <span class="status-pill">${statusLabel()}</span>
          <span class="text-action" role="button" tabindex="0" data-action="home">返回首页</span>
        </div>
      </nav>

      <section class="workspace-grid">
        <section class="panel scan-panel">
          <div class="panel-heading">
            <div>
              <p class="eyebrow">${state.selectedRootPath ?? "扫描结果"}</p>
              <h2>${state.files.length || "待扫描"} 个项目</h2>
            </div>
            <span class="icon-action ${isBusy ? "disabled" : ""}" role="button" tabindex="0" data-action="scan" aria-label="选择目录并扫描">${isBusy ? "…" : "↻"}</span>
          </div>
          ${renderScanMeta()}
          <div class="file-list">
            ${state.files.map(renderFileRow).join("") || renderEmptyScan(entry)}
          </div>
        </section>

        <section class="panel plan-panel">
          <div class="panel-heading">
            <div>
              <p class="eyebrow">分类结果</p>
              <h2>${state.classifications.length ? "候选分类已生成" : "等待规则分类"}</h2>
            </div>
            <span class="count-chip">${state.classifications.length} 项分类</span>
          </div>
          ${state.classifications.length ? renderClassifications() : renderEmptyPlan()}
        </section>

        <section class="panel action-panel">
          <p class="eyebrow">第三阶段</p>
          <h2>${actionTitle()}</h2>
          <p class="action-copy">${actionCopy()}</p>
          <div class="action-stack">
            <span class="primary-command ${isBusy ? "disabled" : ""}" role="button" tabindex="0" data-action="scan">
              选择目录并扫描
            </span>
            <span class="secondary-command ${canClassify() ? "" : "disabled"}" role="button" tabindex="0" data-action="classify">
              生成分类
            </span>
            <span class="secondary-command ${state.status !== "scanning" ? "disabled" : ""}" role="button" tabindex="0" data-action="cancel-scan">
              取消扫描
            </span>
          </div>
          ${state.batch ? renderBatch() : ""}
        </section>
      </section>
    </main>
  `;
}

function renderFileRow(file: WorkflowState["files"][number]): string {
  return `
    <article class="file-row">
      <div class="file-token" aria-hidden="true">${file.name.slice(0, 1).toUpperCase()}</div>
      <div>
        <h3>${file.name}</h3>
        <p>${file.kind} · ${file.sizeLabel} · ${formatModifiedAt(file.modifiedAt)} · ${file.path}</p>
      </div>
      <span>${file.extension ? `.${file.extension}` : "无扩展名"}</span>
    </article>
  `;
}

function renderScanMeta(): string {
  if (state.errorMessage) {
    return `<div class="scan-meta error">${state.errorMessage}</div>`;
  }

  if (!state.files.length && state.status !== "cancelled") {
    return "";
  }

  return `
    <div class="scan-meta">
      <span>已跳过 ${state.skippedCount} 项</span>
      <span>读取失败 ${state.errorCount} 项</span>
      ${state.currentTaskId ? `<span>任务 ${state.currentTaskId}</span>` : ""}
    </div>
  `;
}

function renderEmptyScan(entry: EntryKind): string {
  return `
    <div class="empty-state">
      <div class="empty-orbit" aria-hidden="true"></div>
      <h3>${entry === "desktop" ? "准备扫描桌面" : "准备扫描目录"}</h3>
      <p>${emptyScanCopy()}</p>
    </div>
  `;
}

function emptyScanCopy(): string {
  if (state.status === "selecting") {
    return "请选择一个授权目录，扫描器只读取文件元数据。";
  }
  if (state.status === "scanning") {
    return "正在扫描目录，过程中可以取消，本阶段不会移动或重命名文件。";
  }
  if (state.status === "cancelled") {
    return "本次扫描已取消，未将不完整结果写入本地数据库。";
  }
  return "选择目录后会扫描文件名、扩展名、大小与修改时间，并写入本地 SQLite。";
}

function renderEmptyPlan(): string {
  return `
    <div class="plan-placeholder">
      <h3>规则分类只生成建议</h3>
      <p>分类器会读取扫描得到的元数据与本地 Skill，输出分类、置信度和证据；不会移动或重命名文件。</p>
    </div>
  `;
}

function renderClassifications(): string {
  const filesById = new Map(state.files.map((file) => [file.id, file]));
  return `
    <div class="plan-summary">已根据扩展名、文件名关键词、内置规则与已启用 Skill 生成分类建议。</div>
    <div class="classification-list">
      ${state.classifications
        .map((classification) => {
          const file = filesById.get(classification.fileId);
          return `
            <article class="classification-row">
              <div>
                <h3>${file?.name ?? classification.fileId}</h3>
                <p>${classification.evidence.join("；")}</p>
              </div>
              <div class="classification-meta">
                <strong>${classification.category}</strong>
                <span>${Math.round(classification.confidence * 100)}%</span>
                <span class="risk ${classification.risk}">${formatRisk(classification.risk)}</span>
              </div>
            </article>
          `;
        })
        .join("")}
    </div>
  `;
}

function renderBatch(): string {
  return `
    <div class="batch-card">
      <span>执行批次 ${state.batch?.id}</span>
      <strong>${state.batch?.operationCount} 项操作 · ${state.batch?.executedAt}</strong>
    </div>
  `;
}

function statusLabel(): string {
  return {
    idle: "待开始",
    selecting: "选择目录",
    scanning: "扫描中",
    scanned: "已扫描",
    classifying: "分类中",
    classified: "已分类",
    cancelled: "已取消",
    planned: "待确认",
    executing: "执行中",
    done: "已执行",
    "rolled-back": "已撤销",
  }[state.status];
}

function actionTitle(): string {
  if (state.status === "selecting") {
    return "选择授权目录";
  }
  if (state.status === "scanning") {
    return "正在只读扫描";
  }
  if (state.status === "scanned") {
    return "可以生成分类";
  }
  if (state.status === "classifying") {
    return "正在生成分类";
  }
  if (state.status === "classified") {
    return "分类建议已生成";
  }
  if (state.status === "cancelled") {
    return "扫描已取消";
  }
  if (state.status === "done") {
    return "整理已完成";
  }
  if (state.status === "rolled-back") {
    return "已撤销本次整理";
  }
  return "先选择目录";
}

function actionCopy(): string {
  if (state.status === "selecting") {
    return "目录选择由系统弹窗完成；取消选择不会启动扫描。";
  }
  if (state.status === "scanning") {
    return "当前任务只读取元数据，取消后不会把部分结果标记为成功。";
  }
  if (state.status === "scanned") {
    return "扫描结果已写入本地 SQLite。现在可以基于元数据、内置规则和已启用 Skill 生成分类建议。";
  }
  if (state.status === "classifying") {
    return "分类器只读取文件元数据并返回证据，本阶段不会创建目录或移动文件。";
  }
  if (state.status === "classified") {
    return "分类建议已生成。后续阶段才会把这些建议转成可确认的 Plan。";
  }
  if (state.status === "cancelled") {
    return "可以重新选择目录开始新的扫描任务。";
  }
  if (state.status === "done") {
    return "已生成执行批次和撤销入口。真实文件操作必须保留 rollback 记录。";
  }
  if (state.status === "rolled-back") {
    return "撤销入口已消费，本次整理恢复为可重新扫描状态。";
  }
  return "本阶段不会执行任何移动、重命名或目录创建操作。";
}

async function runScan(entry: EntryKind): Promise<void> {
  if (state.status === "selecting" || state.status === "scanning") {
    return;
  }

  state.status = "selecting";
  state.errorMessage = null;
  state.files = [];
  state.classifications = [];
  state.plan = null;
  state.batch = null;
  state.skippedCount = 0;
  state.errorCount = 0;
  render();

  const selectedRoot =
    (await tauriClient.selectScanFolder()) ??
    (tauriClient.usesMockCommands() ? defaultMockRoot(entry) : null);

  if (!selectedRoot) {
    state.status = "idle";
    render();
    return;
  }

  const taskId = `scan-${Date.now()}`;
  state.currentTaskId = taskId;
  state.selectedRootPath = selectedRoot;
  state.status = "scanning";
  render();

  try {
    const response = await tauriClient.scanFolder(entry, {
      taskId,
      rootPath: selectedRoot,
      recursive: entry === "files",
      includeHidden: false,
      followSymlinks: false,
    });

    state.files = response.files;
    state.skippedCount = response.skippedCount;
    state.errorCount = response.errorCount;
    state.classifications = [];
    state.status = response.status === "cancelled" ? "cancelled" : "scanned";
  } catch (error) {
    state.status = "idle";
    state.errorMessage = error instanceof Error ? error.message : String(error);
  } finally {
    render();
  }
}

function canClassify(): boolean {
  return (
    state.status === "scanned" &&
    Boolean(state.currentTaskId) &&
    Boolean(state.selectedRootPath) &&
    state.files.length > 0
  );
}

async function classifyCurrentScan(): Promise<void> {
  if (!canClassify() || !state.currentTaskId || !state.selectedRootPath) {
    return;
  }

  state.status = "classifying";
  state.errorMessage = null;
  render();

  try {
    state.classifications = await tauriClient.classifyFiles(
      state.currentTaskId,
      state.selectedRootPath,
      state.files,
    );
    state.status = "classified";
  } catch (error) {
    state.status = "scanned";
    state.errorMessage = error instanceof Error ? error.message : String(error);
  } finally {
    render();
  }
}

async function executePlan(): Promise<void> {
  if (!state.plan || state.batch) {
    return;
  }

  state.status = "executing";
  render();

  state.batch = await tauriClient.executeConfirmedPlan(state.plan);
  state.status = "done";
  render();
}

async function rollback(): Promise<void> {
  if (!state.batch?.rollbackAvailable) {
    return;
  }

  state.batch = await tauriClient.rollbackBatch(state.batch);
  state.status = "rolled-back";
  render();
}

async function cancelCurrentScan(): Promise<void> {
  if (!state.currentTaskId || state.status !== "scanning") {
    return;
  }

  await tauriClient.cancelScan(state.currentTaskId);
}

function handleAction(target: HTMLElement): void {
  const entry = target.dataset.entry as EntryKind | undefined;
  const action = target.dataset.action;

  if (entry) {
    state.entry = entry;
    state.status = "idle";
    render();
    return;
  }

  if (action === "home") {
    state.entry = null;
    state.status = "idle";
    state.currentTaskId = null;
    state.selectedRootPath = null;
    state.skippedCount = 0;
    state.errorCount = 0;
    state.errorMessage = null;
    state.files = [];
    state.classifications = [];
    state.plan = null;
    state.batch = null;
    render();
    return;
  }

  if (action === "scan" && state.entry) {
    void runScan(state.entry);
    return;
  }

  if (action === "cancel-scan") {
    void cancelCurrentScan();
    return;
  }

  if (action === "classify") {
    void classifyCurrentScan();
    return;
  }

  if (action === "execute") {
    void executePlan();
    return;
  }

  if (action === "rollback") {
    void rollback();
  }
}

appRoot.addEventListener("click", (event) => {
  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-entry], [data-action]");
  if (target) {
    handleAction(target);
  }
});

appRoot.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }

  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-entry], [data-action]");
  if (target) {
    event.preventDefault();
    handleAction(target);
  }
});

render();

function defaultMockRoot(entry: EntryKind): string {
  return entry === "desktop" ? "~/Desktop" : "~/Downloads";
}

function formatModifiedAt(value: string | null | undefined): string {
  if (!value) {
    return "修改时间未知";
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return parsed.toLocaleString("zh-CN", {
    month: "2-digit",
    day: "2-digit",
    hour: "2-digit",
    minute: "2-digit",
  });
}
