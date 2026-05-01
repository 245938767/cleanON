import "./styles.css";
import { defaultRoot } from "./mockData";
import { tauriClient } from "./tauriClient";
import type {
  EntryKind,
  ExecutionBatchDto,
  FileRisk,
  ModelSettingsDto,
  OperationRowDto,
  OrganizationPlanDto,
  SkillUpdateProposalDto,
  WorkflowState,
  WorkflowView,
} from "./types";

const app = document.querySelector<HTMLDivElement>("#app");

if (!app) {
  throw new Error("Missing #app root");
}

const appRoot = app;

const state: WorkflowState = {
  entry: null,
  view: "plan",
  currentTaskId: null,
  selectedRootPath: null,
  skippedCount: 0,
  errorCount: 0,
  errorMessage: null,
  files: [],
  classifications: [],
  plan: null,
  batches: [],
  skills: [],
  modelSettings: {
    provider: "local",
    cloudEnabled: false,
    baseUrl: "http://127.0.0.1:11434/v1",
    model: "local-classifier",
  },
  modelTestMessage: null,
  desktopPreview: null,
  editedOperationIds: [],
  status: "idle",
};

void hydrateSecondaryData();
render();

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
          <p>先生成整理方案，逐项确认后再执行。</p>
        </header>

        <div class="entry-orbits">
          <article class="home-entry" role="button" tabindex="0" data-entry="files" aria-label="进入文件整理">
            <img src="/visuals/file-organizer-entry.png" alt="" class="entry-art" />
            <h2>文件整理</h2>
            <p>扫描目录并规划归类</p>
          </article>
          <article class="home-entry" role="button" tabindex="0" data-entry="desktop" aria-label="进入桌面整理">
            <img src="/visuals/desktop-organizer-entry.png" alt="" class="entry-art" />
            <h2>桌面整理</h2>
            <p>预览桌面归档方案</p>
          </article>
        </div>
      </section>
    </main>
  `;
}

function renderWorkspace(entry: EntryKind): void {
  appRoot.innerHTML = `
    <main class="shell workspace-shell">
      <nav class="topbar" aria-label="页面导航">
        <div>
          <p class="eyebrow">本地优先 · 明确确认后执行</p>
          <h1>${entryTitle(entry)}</h1>
        </div>
        <div class="nav-actions">
          <span class="status-pill">${statusLabel()}</span>
          <span class="text-action" role="button" tabindex="0" data-action="home">返回首页</span>
        </div>
      </nav>

      <section class="workspace-layout">
        <aside class="side-rail" aria-label="工作区视图">
          ${viewTab("plan", "方案")}
          ${viewTab("history", "历史")}
          ${viewTab("skills", "习惯")}
          ${viewTab("models", "模型")}
          ${viewTab("desktop", "桌面")}
        </aside>
        <section class="workspace-content">
          ${renderProgress()}
          ${renderError()}
          ${renderActiveView(entry)}
        </section>
      </section>
    </main>
  `;
}

function renderActiveView(entry: EntryKind): string {
  if (state.view === "history") {
    return renderHistoryView();
  }
  if (state.view === "skills") {
    return renderSkillCenter();
  }
  if (state.view === "models") {
    return renderModelSettings();
  }
  if (state.view === "desktop") {
    return renderDesktopPreview(entry);
  }
  return renderPlanWorkspace(entry);
}

function renderPlanWorkspace(entry: EntryKind): string {
  return `
    <section class="workspace-grid">
      <section class="panel scan-panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">${escapeHtml(state.selectedRootPath ?? "扫描目录")}</p>
            <h2>${state.files.length || "待扫描"} 个项目</h2>
          </div>
          <span class="icon-action ${isBusy() ? "disabled" : ""}" role="button" tabindex="0" data-action="scan" aria-label="选择目录并扫描">↻</span>
        </div>
        ${renderScanMeta()}
        <div class="file-list">
          ${state.files.map(renderFileRow).join("") || renderEmptyScan(entry)}
        </div>
      </section>

      <section class="panel flow-panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">流程</p>
            <h2>${flowTitle()}</h2>
          </div>
          <span class="count-chip">${state.classifications.length} 项分类</span>
        </div>
        <div class="action-stack">
          <button class="primary-command" type="button" data-action="scan" ${isBusy() ? "disabled" : ""}>选择目录并扫描</button>
          <button class="secondary-command" type="button" data-action="classify" ${canClassify() ? "" : "disabled"}>生成分类</button>
          <button class="secondary-command" type="button" data-action="generate-plan" ${canGeneratePlan() ? "" : "disabled"}>生成整理方案</button>
          <button class="secondary-command" type="button" data-action="cancel-scan" ${state.status === "scanning" ? "" : "disabled"}>取消扫描</button>
        </div>
        ${state.classifications.length ? renderClassifications() : renderClassificationPlaceholder()}
      </section>
    </section>

    <section class="panel diff-panel">
      ${state.plan ? renderDiffBoard(state.plan) : renderPlanPlaceholder()}
    </section>
  `;
}

function renderDiffBoard(plan: OrganizationPlanDto): string {
  const selectedCount = plan.rows.filter((row) => row.selected).length;
  const blockingCount = plan.rows.filter((row) => row.conflictStatus === "blocked" || row.validationIssues.length).length;

  return `
    <div class="panel-heading">
      <div>
        <p class="eyebrow">Plan Diff Board</p>
        <h2>${selectedCount} / ${plan.rows.length} 项将进入确认</h2>
      </div>
      <div class="summary-strip">
        <span>${plan.summary.foldersToCreate} 文件夹</span>
        <span>${plan.summary.filesToMove} 移动</span>
        <span>${plan.summary.filesToRename} 重命名</span>
      </div>
    </div>
    <div class="plan-summary">
      计划 ${escapeHtml(plan.planId)} · ${escapeHtml(plan.mode)} · ${formatDate(plan.createdAt)}
    </div>
    <div class="operation-table">
      ${plan.rows.map(renderOperationRow).join("")}
    </div>
    <div class="confirm-bar">
      <div>
        <strong>显式确认执行</strong>
        <p>只执行已接受行，最终目标以编辑后的路径为准；执行会生成 rollback 记录。</p>
      </div>
      <button class="primary-command compact" type="button" data-action="execute" ${
        selectedCount === 0 || blockingCount > 0 || isBusy() ? "disabled" : ""
      }>确认执行 ${selectedCount} 项</button>
    </div>
  `;
}

function renderOperationRow(row: OperationRowDto): string {
  const changed = row.editableTarget !== row.target;
  return `
    <article class="operation-row ${row.selected ? "selected" : "rejected"}">
      <label class="toggle-line">
        <input type="checkbox" data-plan-toggle="${escapeAttr(row.operationId)}" ${row.selected ? "checked" : ""} />
        <span>${row.selected ? "接受" : "跳过"}</span>
      </label>
      <div class="operation-main">
        <div class="operation-title">
          <h3>${escapeHtml(row.title)}</h3>
          <span class="risk ${row.risk}">${formatRisk(row.risk)}</span>
          <span class="conflict ${row.conflictStatus}">${formatConflict(row.conflictStatus)}</span>
        </div>
        <p>${escapeHtml(row.reason)}</p>
        ${row.source ? `<p class="path-line">来源 ${escapeHtml(row.source)}</p>` : ""}
        <label class="target-editor">
          <span>目标</span>
          <input type="text" value="${escapeAttr(row.editableTarget)}" data-target-edit="${escapeAttr(row.operationId)}" />
        </label>
        ${row.validationIssues.length ? renderValidationIssues(row) : ""}
      </div>
      <button class="secondary-command compact" type="button" data-action="save-habit" data-operation-id="${escapeAttr(row.operationId)}" ${
        changed ? "" : "disabled"
      }>保存为习惯</button>
    </article>
  `;
}

function renderHistoryView(): string {
  return `
    <section class="panel history-panel">
      <div class="panel-heading">
        <div>
          <p class="eyebrow">执行历史</p>
          <h2>${state.batches.length || "暂无"} 个批次</h2>
        </div>
        <button class="secondary-command compact" type="button" data-action="refresh-history">刷新</button>
      </div>
      <div class="history-list">
        ${state.batches.map(renderBatchCard).join("") || `<div class="empty-state"><h3>还没有执行记录</h3><p>确认执行后，这里会显示批次摘要与撤销入口。</p></div>`}
      </div>
    </section>
  `;
}

function renderBatchCard(batch: ExecutionBatchDto): string {
  const rollbackAvailable = batch.rollbackEntries.length > 0 && batch.status !== "rolled_back";
  return `
    <article class="batch-card">
      <div>
        <span>${escapeHtml(batch.batchId)}</span>
        <strong>${batch.executedOperations.length} 项操作 · ${formatStatus(batch.status)}</strong>
        <p>${formatDate(batch.startedAt)} - ${formatDate(batch.finishedAt)}</p>
      </div>
      <div class="batch-actions">
        <span>${batch.errors.length} 个错误</span>
        <button class="secondary-command compact" type="button" data-action="rollback" data-batch-id="${escapeAttr(batch.batchId)}" ${
          rollbackAvailable ? "" : "disabled"
        }>撤销</button>
      </div>
    </article>
  `;
}

function renderSkillCenter(): string {
  const selectedEditedRow = getFirstEditedRow();
  const suggestedRule = selectedEditedRow ? buildRuleFromRow(selectedEditedRow) : "";

  return `
    <section class="workspace-grid two-column">
      <section class="panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">Skill Center</p>
            <h2>${state.skills.length} 条习惯</h2>
          </div>
          <button class="secondary-command compact" type="button" data-action="refresh-skills">刷新</button>
        </div>
        <div class="skill-list">
          ${state.skills.map(renderSkillRow).join("") || `<div class="empty-state"><h3>暂无习惯</h3><p>编辑计划目标后可以保存成长期规则。</p></div>`}
        </div>
      </section>
      <section class="panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">保存习惯</p>
            <h2>${selectedEditedRow ? "来自已编辑目标" : "等待目标编辑"}</h2>
          </div>
        </div>
        <form class="skill-form" data-form="skill">
          <label>
            <span>名称</span>
            <input name="skillName" value="${escapeAttr(selectedEditedRow ? `${selectedEditedRow.title} 的目标习惯` : "")}" />
          </label>
          <label>
            <span>规则 JSON</span>
            <textarea name="skillRule" rows="9">${escapeHtml(suggestedRule)}</textarea>
          </label>
          <label class="checkbox-line">
            <input name="skillEnabled" type="checkbox" checked />
            <span>启用这条习惯</span>
          </label>
          <button class="primary-command" type="submit" ${selectedEditedRow ? "" : "disabled"}>保存 Skill</button>
        </form>
      </section>
    </section>
  `;
}

function renderSkillRow(skill: WorkflowState["skills"][number]): string {
  return `
    <article class="skill-row">
      <div>
        <h3>${escapeHtml(skill.name)}</h3>
        <p>${escapeHtml(skill.rule)}</p>
        <span>${formatDate(skill.createdAt)}</span>
      </div>
      <div class="row-actions">
        <button class="secondary-command compact" type="button" data-action="toggle-skill" data-skill-id="${escapeAttr(skill.id)}">
          ${skill.enabled ? "停用" : "启用"}
        </button>
        <button class="danger-command compact" type="button" data-action="delete-skill" data-skill-id="${escapeAttr(skill.id)}">删除</button>
      </div>
    </article>
  `;
}

function renderModelSettings(): string {
  return `
    <section class="panel model-panel">
      <div class="panel-heading">
        <div>
          <p class="eyebrow">Model Settings</p>
          <h2>默认本地优先</h2>
        </div>
        <span class="status-pill">${state.modelSettings.cloudEnabled ? "云模型已显式启用" : "本地模式"}</span>
      </div>
      <form class="model-form" data-form="model">
        <label>
          <span>Provider</span>
          <select name="provider">
            ${modelOption("local", "本地")}
            ${modelOption("openai_compatible", "OpenAI Compatible")}
            ${modelOption("custom", "自定义")}
          </select>
        </label>
        <label>
          <span>Base URL</span>
          <input name="baseUrl" value="${escapeAttr(state.modelSettings.baseUrl ?? "")}" />
        </label>
        <label>
          <span>Model</span>
          <input name="model" value="${escapeAttr(state.modelSettings.model ?? "")}" />
        </label>
        <label class="checkbox-line">
          <input name="cloudEnabled" type="checkbox" ${state.modelSettings.cloudEnabled ? "checked" : ""} />
          <span>显式启用云模型</span>
        </label>
        <label>
          <span>运行时 API Key 测试</span>
          <input name="runtimeApiKey" type="password" autocomplete="off" placeholder="仅用于本次测试，不保存" />
        </label>
        <div class="form-actions">
          <button class="secondary-command compact" type="button" data-action="test-model">测试连接</button>
          <button class="primary-command compact" type="submit">保存设置</button>
        </div>
      </form>
      ${state.modelTestMessage ? `<div class="scan-meta"><span>${escapeHtml(state.modelTestMessage)}</span></div>` : ""}
    </section>
  `;
}

function renderDesktopPreview(entry: EntryKind): string {
  const preview = state.desktopPreview;
  return `
    <section class="workspace-grid two-column">
      <section class="panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">macOS Archive Preview</p>
            <h2>${escapeHtml(preview?.macosArchive?.archiveFolder ?? `${state.selectedRootPath ?? defaultRoot(entry)}/桌面归档`)}</h2>
          </div>
          <button class="secondary-command compact" type="button" data-action="refresh-preview">刷新预览</button>
        </div>
        <p class="action-copy">${escapeHtml(preview?.macosArchive?.note ?? "预览只展示归档目标。")}</p>
        <div class="operation-table slim">
          ${(preview?.macosArchive?.rows ?? state.plan?.rows ?? []).map(renderArchiveRow).join("") || `<div class="empty-state"><h3>暂无桌面计划</h3><p>生成整理方案后会展示归档预览。</p></div>`}
        </div>
      </section>
      <section class="panel">
        <div class="panel-heading">
          <div>
            <p class="eyebrow">Windows Partition Canvas</p>
            <h2>预览分区，不写回坐标</h2>
          </div>
          <span class="count-chip">${preview?.windowsPartition?.partitions.length ?? 0} 区</span>
        </div>
        ${renderWindowsCanvas()}
      </section>
    </section>
  `;
}

function renderWindowsCanvas(): string {
  const partition = state.desktopPreview?.windowsPartition;
  if (!partition) {
    return `<div class="desktop-canvas empty"><p>桌面整理模式会展示 Windows 分区预览。</p></div>`;
  }

  return `
    <div class="desktop-canvas" style="--canvas-w:${partition.width}; --canvas-h:${partition.height};">
      ${partition.partitions
        .map(
          (item) => `
            <div class="desktop-zone" style="left:${(item.x / partition.width) * 100}%; top:${(item.y / partition.height) * 100}%; width:${(item.width / partition.width) * 100}%; height:${(item.height / partition.height) * 100}%;">
              <strong>${escapeHtml(item.label)}</strong>
              <span>${item.fileCount} 项</span>
            </div>
          `,
        )
        .join("")}
    </div>
  `;
}

function renderArchiveRow(row: OperationRowDto): string {
  return `
    <article class="archive-row">
      <strong>${escapeHtml(row.title)}</strong>
      <span>${escapeHtml(row.editableTarget || row.target)}</span>
    </article>
  `;
}

function renderProgress(): string {
  const steps: Array<[WorkflowState["status"], string]> = [
    ["scanned", "scan"],
    ["classified", "classify"],
    ["planned", "generate plan"],
    ["executing", "confirm execute"],
    ["done", "history/rollback"],
  ];
  const currentIndex = progressIndex();
  return `
    <section class="flow-steps" aria-label="整理流程">
      ${steps
        .map(
          ([, label], index) => `
            <span class="${index <= currentIndex ? "active" : ""}">${label}</span>
          `,
        )
        .join("")}
    </section>
  `;
}

function renderFileRow(file: WorkflowState["files"][number]): string {
  return `
    <article class="file-row">
      <div class="file-token" aria-hidden="true">${escapeHtml(file.name.slice(0, 1).toUpperCase())}</div>
      <div>
        <h3>${escapeHtml(file.name)}</h3>
        <p>${escapeHtml(file.kind)} · ${escapeHtml(file.sizeLabel)} · ${escapeHtml(formatModifiedAt(file.modifiedAt))} · ${escapeHtml(file.path)}</p>
      </div>
      <span>${file.extension ? `.${escapeHtml(file.extension)}` : "无扩展名"}</span>
    </article>
  `;
}

function renderScanMeta(): string {
  if (!state.files.length && state.status !== "cancelled") {
    return "";
  }

  return `
    <div class="scan-meta">
      <span>已跳过 ${state.skippedCount} 项</span>
      <span>读取失败 ${state.errorCount} 项</span>
      ${state.currentTaskId ? `<span>任务 ${escapeHtml(state.currentTaskId)}</span>` : ""}
    </div>
  `;
}

function renderError(): string {
  return state.errorMessage ? `<div class="global-error">${escapeHtml(state.errorMessage)}</div>` : "";
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

function renderClassificationPlaceholder(): string {
  return `
    <div class="plan-placeholder">
      <h3>分类只生成建议</h3>
      <p>分类器读取扫描元数据与已启用 Skill，返回分类、证据和风险。</p>
    </div>
  `;
}

function renderClassifications(): string {
  const filesById = new Map(state.files.map((file) => [file.id, file]));
  return `
    <div class="classification-list compact-list">
      ${state.classifications
        .map((classification) => {
          const file = filesById.get(classification.fileId);
          return `
            <article class="classification-row">
              <div>
                <h3>${escapeHtml(file?.name ?? classification.fileId)}</h3>
                <p>${escapeHtml(classification.evidence.join("；"))}</p>
              </div>
              <div class="classification-meta">
                <strong>${escapeHtml(classification.category)}</strong>
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

function renderPlanPlaceholder(): string {
  return `
    <div class="plan-placeholder large">
      <h3>等待整理方案</h3>
      <p>完成扫描和分类后生成 Plan rows，在这里逐项接受、跳过、编辑目标并确认执行。</p>
    </div>
  `;
}

function renderValidationIssues(row: OperationRowDto): string {
  return `
    <div class="validation-list">
      ${row.validationIssues.map((issue) => `<span>${escapeHtml(issue.message)}</span>`).join("")}
    </div>
  `;
}

function viewTab(view: WorkflowView, label: string): string {
  return `<span class="side-tab ${state.view === view ? "active" : ""}" role="button" tabindex="0" data-view="${view}">${label}</span>`;
}

async function runScan(entry: EntryKind): Promise<void> {
  if (state.status === "selecting" || state.status === "scanning") {
    return;
  }

  state.status = "selecting";
  state.view = "plan";
  state.errorMessage = null;
  state.files = [];
  state.classifications = [];
  state.plan = null;
  state.desktopPreview = null;
  state.editedOperationIds = [];
  state.skippedCount = 0;
  state.errorCount = 0;
  render();

  const selectedRoot =
    (await tauriClient.selectScanFolder()) ??
    (tauriClient.usesMockCommands() ? defaultRoot(entry) : null);

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

async function generateCurrentPlan(): Promise<void> {
  if (!canGeneratePlan() || !state.entry || !state.currentTaskId || !state.selectedRootPath) {
    return;
  }

  state.status = "planning";
  state.errorMessage = null;
  render();

  try {
    const generated = await tauriClient.generatePlan(
      state.entry,
      state.currentTaskId,
      state.selectedRootPath,
      state.files,
      state.classifications,
    );
    state.plan = await tauriClient.reviewPlan(generated);
    state.status = "planned";
    state.desktopPreview = await tauriClient.loadDesktopPreview(state.entry, state.selectedRootPath, state.plan);
  } catch (error) {
    state.status = "classified";
    state.errorMessage = error instanceof Error ? error.message : String(error);
  } finally {
    render();
  }
}

async function executePlan(): Promise<void> {
  if (!state.plan || state.status === "executing") {
    return;
  }

  const selectedRows = state.plan.rows.filter((row) => row.selected);
  if (!selectedRows.length || selectedRows.some((row) => row.conflictStatus === "blocked" || row.validationIssues.length)) {
    return;
  }

  state.status = "executing";
  render();

  try {
    const approval = {
      approved: true,
      approvedPlanId: state.plan.planId,
      approvedAt: new Date().toISOString(),
      actor: "desktop-user",
    };
    const batch = await tauriClient.executeConfirmedPlan(state.plan, approval);
    state.batches = [batch, ...state.batches.filter((item) => item.batchId !== batch.batchId)];
    state.status = "done";
    state.view = "history";
  } catch (error) {
    state.status = "planned";
    state.errorMessage = error instanceof Error ? error.message : String(error);
  } finally {
    render();
  }
}

async function rollback(batchId: string): Promise<void> {
  const batch = state.batches.find((item) => item.batchId === batchId);
  if (!batch || !batch.rollbackEntries.length) {
    return;
  }

  state.status = "rolling-back";
  render();

  try {
    await tauriClient.rollbackBatch(batch);
    state.batches = state.batches.map((item) =>
      item.batchId === batchId ? { ...item, status: "rolled_back", rollbackEntries: [] } : item,
    );
    state.status = "rolled-back";
  } catch (error) {
    state.status = "done";
    state.errorMessage = error instanceof Error ? error.message : String(error);
  } finally {
    render();
  }
}

async function cancelCurrentScan(): Promise<void> {
  if (!state.currentTaskId || state.status !== "scanning") {
    return;
  }

  await tauriClient.cancelScan(state.currentTaskId);
  state.status = "cancelled";
  render();
}

async function hydrateSecondaryData(): Promise<void> {
  const [skills, settings] = await Promise.all([tauriClient.listSkills(), tauriClient.loadModelSettings()]);
  state.skills = skills;
  state.modelSettings = settings;
  render();
}

async function refreshHistory(): Promise<void> {
  const summaries = await tauriClient.listHistory();
  const knownById = new Map(state.batches.map((batch) => [batch.batchId, batch]));
  state.batches = summaries.map((summary) => knownById.get(summary.batchId) ?? summaryToBatch(summary));
  render();
}

async function refreshSkills(): Promise<void> {
  state.skills = await tauriClient.listSkills();
  render();
}

async function refreshPreview(): Promise<void> {
  if (!state.entry) {
    return;
  }
  state.desktopPreview = await tauriClient.loadDesktopPreview(state.entry, state.selectedRootPath, state.plan);
  render();
}

function updatePlanSelection(operationId: string, selected: boolean): void {
  if (!state.plan) {
    return;
  }
  state.plan = {
    ...state.plan,
    rows: state.plan.rows.map((row) => (row.operationId === operationId ? { ...row, selected } : row)),
  };
  render();
}

async function updatePlanTarget(operationId: string, target: string): Promise<void> {
  if (!state.plan || !state.entry) {
    return;
  }
  state.plan = {
    ...state.plan,
    rows: state.plan.rows.map((row) =>
      row.operationId === operationId
        ? {
            ...row,
            editableTarget: target,
            conflictStatus: target.trim() ? row.conflictStatus : "blocked",
            validationIssues: target.trim() ? row.validationIssues.filter((issue) => issue.message !== "目标路径不能为空") : [{ operationId, message: "目标路径不能为空" }],
          }
        : row,
    ),
  };
  if (!state.editedOperationIds.includes(operationId)) {
    state.editedOperationIds = [...state.editedOperationIds, operationId];
  }
  state.desktopPreview = await tauriClient.loadDesktopPreview(state.entry, state.selectedRootPath, state.plan);
  render();
}

async function saveHabitFromRow(operationId: string): Promise<void> {
  const row = state.plan?.rows.find((item) => item.operationId === operationId);
  if (!row || row.editableTarget === row.target) {
    return;
  }

  const skill = await tauriClient.saveSkill({
    name: `${row.title} 的目标习惯`,
    rule: buildRuleFromRow(row),
    enabled: true,
  });
  state.skills = [skill, ...state.skills];
  state.view = "skills";
  render();
}

async function toggleSkill(skillId: string): Promise<void> {
  const skill = state.skills.find((item) => item.id === skillId);
  if (!skill) {
    return;
  }
  const updated = await tauriClient.setSkillEnabled(skill, !skill.enabled);
  state.skills = state.skills.map((item) => (item.id === skillId ? updated : item));
  render();
}

async function deleteSkill(skillId: string): Promise<void> {
  await tauriClient.deleteSkill(skillId);
  state.skills = state.skills.filter((skill) => skill.id !== skillId);
  render();
}

function handleAction(target: HTMLElement): void {
  const entry = target.dataset.entry as EntryKind | undefined;
  const view = target.dataset.view as WorkflowView | undefined;
  const action = target.dataset.action;

  if (entry) {
    resetWorkspace(entry);
    return;
  }

  if (view) {
    state.view = view;
    if (view === "desktop") {
      void refreshPreview();
    } else {
      render();
    }
    return;
  }

  if (action === "home") {
    resetHome();
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

  if (action === "generate-plan") {
    void generateCurrentPlan();
    return;
  }

  if (action === "execute") {
    void executePlan();
    return;
  }

  if (action === "refresh-history") {
    void refreshHistory();
    return;
  }

  if (action === "rollback" && target.dataset.batchId) {
    void rollback(target.dataset.batchId);
    return;
  }

  if (action === "refresh-skills") {
    void refreshSkills();
    return;
  }

  if (action === "save-habit" && target.dataset.operationId) {
    void saveHabitFromRow(target.dataset.operationId);
    return;
  }

  if (action === "toggle-skill" && target.dataset.skillId) {
    void toggleSkill(target.dataset.skillId);
    return;
  }

  if (action === "delete-skill" && target.dataset.skillId) {
    void deleteSkill(target.dataset.skillId);
    return;
  }

  if (action === "refresh-preview") {
    void refreshPreview();
    return;
  }

  if (action === "test-model") {
    void testModelFromForm();
  }
}

appRoot.addEventListener("click", (event) => {
  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-entry], [data-view], [data-action]");
  if (target) {
    handleAction(target);
  }
});

appRoot.addEventListener("keydown", (event) => {
  if (event.key !== "Enter" && event.key !== " ") {
    return;
  }

  const target = (event.target as HTMLElement).closest<HTMLElement>("[data-entry], [data-view], [data-action]");
  if (target) {
    event.preventDefault();
    handleAction(target);
  }
});

appRoot.addEventListener("change", (event) => {
  const target = event.target as HTMLInputElement | HTMLSelectElement;
  if (target.dataset.planToggle) {
    updatePlanSelection(target.dataset.planToggle, (target as HTMLInputElement).checked);
  }
});

appRoot.addEventListener("input", (event) => {
  const target = event.target as HTMLInputElement;
  if (target.dataset.targetEdit) {
    void updatePlanTarget(target.dataset.targetEdit, target.value);
  }
});

appRoot.addEventListener("submit", (event) => {
  event.preventDefault();
  const form = event.target as HTMLFormElement;
  if (form.dataset.form === "skill") {
    void submitSkillForm(form);
  }
  if (form.dataset.form === "model") {
    void submitModelForm(form);
  }
});

async function submitSkillForm(form: HTMLFormElement): Promise<void> {
  const data = new FormData(form);
  const proposal: SkillUpdateProposalDto = {
    name: String(data.get("skillName") ?? "").trim(),
    rule: String(data.get("skillRule") ?? "").trim(),
    enabled: data.get("skillEnabled") === "on",
  };
  if (!proposal.name || !proposal.rule) {
    return;
  }
  const skill = await tauriClient.saveSkill(proposal);
  state.skills = [skill, ...state.skills];
  render();
}

async function submitModelForm(form: HTMLFormElement): Promise<void> {
  const settings = readModelSettingsForm(form);
  state.modelSettings = await tauriClient.saveModelSettings(settings);
  state.modelTestMessage = "模型设置已保存；API Key 未保存。";
  render();
}

async function testModelFromForm(): Promise<void> {
  const form = appRoot.querySelector<HTMLFormElement>('[data-form="model"]');
  if (!form) {
    return;
  }
  const data = new FormData(form);
  const apiKey = String(data.get("runtimeApiKey") ?? "");
  const result = await tauriClient.testModelRuntime(readModelSettingsForm(form), apiKey);
  form.reset();
  state.modelTestMessage = result.message;
  render();
}

function readModelSettingsForm(form: HTMLFormElement): ModelSettingsDto {
  const data = new FormData(form);
  return {
    provider: String(data.get("provider") ?? "local") as ModelSettingsDto["provider"],
    cloudEnabled: data.get("cloudEnabled") === "on",
    baseUrl: String(data.get("baseUrl") ?? "").trim(),
    model: String(data.get("model") ?? "").trim() || null,
  };
}

function resetWorkspace(entry: EntryKind): void {
  state.entry = entry;
  state.view = "plan";
  state.currentTaskId = null;
  state.selectedRootPath = null;
  state.skippedCount = 0;
  state.errorCount = 0;
  state.errorMessage = null;
  state.files = [];
  state.classifications = [];
  state.plan = null;
  state.desktopPreview = null;
  state.editedOperationIds = [];
  state.status = "idle";
  render();
}

function resetHome(): void {
  state.entry = null;
  state.status = "idle";
  state.currentTaskId = null;
  state.selectedRootPath = null;
  state.errorMessage = null;
  render();
}

function canClassify(): boolean {
  return state.status === "scanned" && Boolean(state.currentTaskId) && Boolean(state.selectedRootPath) && state.files.length > 0;
}

function canGeneratePlan(): boolean {
  return state.status === "classified" && state.classifications.length > 0;
}

function isBusy(): boolean {
  return ["selecting", "scanning", "classifying", "planning", "executing", "rolling-back"].includes(state.status);
}

function entryTitle(entry: EntryKind): string {
  return entry === "desktop" ? "桌面整理" : "文件整理";
}

function statusLabel(): string {
  return {
    idle: "待开始",
    selecting: "选择目录",
    scanning: "扫描中",
    scanned: "已扫描",
    classifying: "分类中",
    classified: "已分类",
    planning: "生成方案",
    cancelled: "已取消",
    planned: "待确认",
    executing: "执行中",
    done: "已执行",
    "rolling-back": "撤销中",
    "rolled-back": "已撤销",
  }[state.status];
}

function flowTitle(): string {
  if (state.status === "scanned") {
    return "下一步生成分类";
  }
  if (state.status === "classified") {
    return "下一步生成方案";
  }
  if (state.status === "planned") {
    return "在 Diff board 确认";
  }
  if (state.status === "done") {
    return "已写入执行历史";
  }
  return "scan -> classify -> generate plan";
}

function emptyScanCopy(): string {
  if (state.status === "selecting") {
    return "请选择授权目录，扫描器只读取文件元数据。";
  }
  if (state.status === "scanning") {
    return "正在扫描目录，过程中可以取消，本阶段不会移动或重命名文件。";
  }
  if (state.status === "cancelled") {
    return "本次扫描已取消，未将不完整结果标记为成功。";
  }
  return "选择目录后会扫描文件名、扩展名、大小与修改时间。";
}

function progressIndex(): number {
  if (state.status === "done" || state.status === "rolled-back") {
    return 4;
  }
  if (state.status === "executing" || state.status === "rolling-back") {
    return 3;
  }
  if (state.status === "planned") {
    return 2;
  }
  if (state.status === "classified" || state.status === "planning") {
    return 1;
  }
  if (state.files.length || state.status === "scanned" || state.status === "classifying") {
    return 0;
  }
  return -1;
}

function getFirstEditedRow(): OperationRowDto | null {
  return state.plan?.rows.find((row) => state.editedOperationIds.includes(row.operationId) && row.editableTarget !== row.target) ?? null;
}

function buildRuleFromRow(row: OperationRowDto): string {
  return JSON.stringify(
    {
      operationType: row.operationType,
      sourcePattern: row.source ? basename(row.source) : row.title,
      targetFolder: dirname(row.editableTarget || row.target),
      reason: "用户编辑目标后保存",
    },
    null,
    2,
  );
}

function summaryToBatch(summary: ReturnType<typeof makeHistorySummaryShape>): ExecutionBatchDto {
  return {
    batchId: summary.batchId,
    planId: summary.planId,
    status: summary.status,
    executedOperations: [],
    rollbackEntries: summary.status === "rolled_back" ? [] : [],
    errors: [],
    startedAt: summary.startedAt,
    finishedAt: summary.finishedAt,
  };
}

function makeHistorySummaryShape() {
  return {
    batchId: "",
    planId: "",
    status: "completed" as ExecutionBatchDto["status"],
    operationCount: 0,
    errorCount: 0,
    startedAt: "",
    finishedAt: "",
  };
}

function modelOption(value: ModelSettingsDto["provider"], label: string): string {
  return `<option value="${value}" ${state.modelSettings.provider === value ? "selected" : ""}>${label}</option>`;
}

function formatRisk(risk: FileRisk): string {
  return {
    low: "低风险",
    medium: "需留意",
    high: "高风险",
  }[risk];
}

function formatConflict(status: OperationRowDto["conflictStatus"]): string {
  return {
    none: "无冲突",
    warning: "需检查",
    blocked: "阻塞",
  }[status];
}

function formatStatus(status: ExecutionBatchDto["status"]): string {
  return {
    completed: "已完成",
    partially_failed: "部分失败",
    rejected: "已拒绝",
    rolled_back: "已撤销",
  }[status];
}

function formatDate(value: string): string {
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

function formatModifiedAt(value: string | null | undefined): string {
  if (!value) {
    return "修改时间未知";
  }

  const parsed = new Date(value);
  if (Number.isNaN(parsed.getTime())) {
    return value;
  }

  return formatDate(value);
}

function basename(path: string): string {
  return path.split("/").filter(Boolean).at(-1) ?? path;
}

function dirname(path: string): string {
  const parts = path.split("/");
  parts.pop();
  return parts.join("/") || path;
}

function escapeHtml(value: string | number): string {
  return String(value)
    .replaceAll("&", "&amp;")
    .replaceAll("<", "&lt;")
    .replaceAll(">", "&gt;")
    .replaceAll('"', "&quot;")
    .replaceAll("'", "&#039;");
}

function escapeAttr(value: string | number): string {
  return escapeHtml(value);
}
