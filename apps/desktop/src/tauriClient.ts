import {
  createDefaultModelSettings,
  createDesktopPreview,
  createHistorySummary,
  createMockBatch,
  createMockClassifications,
  createMockFiles,
  createMockPlan,
  createMockSkills,
  createModelTestResult,
  createRollbackResult,
  createSkillFromProposal,
  defaultRoot,
} from "./mockData";
import type {
  CancelScanResponse,
  ClassificationResultDto,
  DesktopPreviewDto,
  EntryKind,
  ExecutionBatchDto,
  GeneratePlanRequestDto,
  HistorySummaryDto,
  ModelRuntimeTestResult,
  ModelSettingsDto,
  OrganizationPlanDto,
  RollbackResultDto,
  ScanFolderRequest,
  ScanFolderResponse,
  SkillDto,
  SkillUpdateProposalDto,
  UserApprovalDto,
} from "./types";

type TauriInvoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

type TauriWindow = Window & {
  __TAURI__?: {
    core?: {
      invoke?: TauriInvoke;
    };
  };
  __TAURI_INTERNALS__?: {
    invoke?: TauriInvoke;
  };
};

const commandDelay = 220;
let mockSkills = createMockSkills();
let mockBatches: ExecutionBatchDto[] = [];
let mockModelSettings = createDefaultModelSettings();

function getInvoke(): TauriInvoke | null {
  const tauriWindow = window as TauriWindow;
  return tauriWindow.__TAURI__?.core?.invoke ?? tauriWindow.__TAURI_INTERNALS__?.invoke ?? null;
}

function realCommandsEnabled(): boolean {
  return new URLSearchParams(window.location.search).has("realCommands");
}

function waitForMock<T>(value: T): Promise<T> {
  return new Promise((resolve) => {
    window.setTimeout(() => resolve(value), commandDelay);
  });
}

function canUseRealCommands(): TauriInvoke | null {
  const invoke = getInvoke();
  return invoke && realCommandsEnabled() ? invoke : null;
}

export const tauriClient = {
  usesMockCommands(): boolean {
    return !canUseRealCommands();
  },

  async selectScanFolder(): Promise<string | null> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(null);
    }

    return invoke<string | null>("select_scan_folder");
  },

  async scanFolder(mode: EntryKind, request: ScanFolderRequest): Promise<ScanFolderResponse> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      const files = await waitForMock(createMockFiles(mode));
      return {
        taskId: request.taskId,
        rootPath: request.rootPath,
        files,
        status: "completed",
        skippedCount: 0,
        errorCount: 0,
      };
    }

    return invoke<ScanFolderResponse>("scan_folder", { request });
  },

  async cancelScan(taskId: string): Promise<CancelScanResponse> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock({ taskId, cancelled: true });
    }

    return invoke<CancelScanResponse>("cancel_scan", { taskId });
  },

  async classifyFiles(taskId: string, rootPath: string, files: FileItemLike[]): Promise<ClassificationResultDto[]> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(createMockClassifications(files));
    }

    return invoke<ClassificationResultDto[]>("classify_files", {
      request: {
        taskId,
        rootPath,
      },
    });
  },

  async generatePlan(
    mode: EntryKind,
    taskId: string,
    rootPath: string,
    files: FileItemLike[],
    classifications: ClassificationResultDto[],
  ): Promise<OrganizationPlanDto> {
    const request: GeneratePlanRequestDto = {
      taskId,
      rootPath,
      mode: mode === "desktop" ? "desktop" : "by_category",
      classifications,
    };
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(createMockPlan(mode, taskId, rootPath, files));
    }

    return invoke<OrganizationPlanDto>("generate_plan", { request });
  },

  async reviewPlan(plan: OrganizationPlanDto): Promise<OrganizationPlanDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(plan);
    }

    return invoke<OrganizationPlanDto>("review_plan", { plan });
  },

  async executeConfirmedPlan(plan: OrganizationPlanDto, approval: UserApprovalDto): Promise<ExecutionBatchDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      const batch = await waitForMock(createMockBatch(plan));
      mockBatches = [batch, ...mockBatches];
      return batch;
    }

    const batch = await invoke<ExecutionBatchDto>("execute_confirmed_plan", { plan, approval });
    mockBatches = [batch, ...mockBatches];
    return batch;
  },

  async listHistory(): Promise<HistorySummaryDto[]> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(mockBatches.map(createHistorySummary));
    }

    try {
      return await invoke<HistorySummaryDto[]>("list_execution_history");
    } catch {
      return mockBatches.map(createHistorySummary);
    }
  },

  async rollbackBatch(batch: ExecutionBatchDto): Promise<RollbackResultDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      const result = await waitForMock(createRollbackResult(batch));
      mockBatches = mockBatches.map((item) =>
        item.batchId === batch.batchId ? { ...item, status: "rolled_back", rollbackEntries: [] } : item,
      );
      return result;
    }

    try {
      return await invoke<RollbackResultDto>("rollback_batch", { batch });
    } catch {
      return createRollbackResult(batch);
    }
  },

  async listSkills(): Promise<SkillDto[]> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(mockSkills);
    }

    return invoke<SkillDto[]>("list_skills");
  },

  async saveSkill(proposal: SkillUpdateProposalDto): Promise<SkillDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      const skill = await waitForMock(createSkillFromProposal(proposal));
      mockSkills = [skill, ...mockSkills];
      return skill;
    }

    return invoke<SkillDto>("save_skill", { proposal });
  },

  async setSkillEnabled(skill: SkillDto, enabled: boolean): Promise<SkillDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      const updated = { ...skill, enabled };
      mockSkills = mockSkills.map((item) => (item.id === skill.id ? updated : item));
      return waitForMock(updated);
    }

    try {
      return await invoke<SkillDto>("set_skill_enabled", { id: skill.id, enabled });
    } catch {
      return { ...skill, enabled };
    }
  },

  async deleteSkill(skillId: string): Promise<void> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      mockSkills = mockSkills.filter((skill) => skill.id !== skillId);
      return waitForMock(undefined);
    }

    try {
      await invoke("delete_skill", { id: skillId });
    } catch {
      return;
    }
  },

  async loadModelSettings(): Promise<ModelSettingsDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(mockModelSettings);
    }

    try {
      return await invoke<ModelSettingsDto>("load_model_settings");
    } catch {
      return mockModelSettings;
    }
  },

  async saveModelSettings(settings: ModelSettingsDto): Promise<ModelSettingsDto> {
    const invoke = canUseRealCommands();
    const sanitized = { ...settings };
    mockModelSettings = sanitized;
    if (!invoke) {
      return waitForMock(sanitized);
    }

    try {
      return await invoke<ModelSettingsDto>("save_model_settings", { settings: sanitized });
    } catch {
      return sanitized;
    }
  },

  async testModelRuntime(settings: ModelSettingsDto, apiKey: string): Promise<ModelRuntimeTestResult> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(createModelTestResult(settings, apiKey));
    }

    try {
      return await invoke<ModelRuntimeTestResult>("test_model_runtime", {
        settings,
        runtimeApiKey: apiKey,
      });
    } catch {
      return createModelTestResult(settings, apiKey);
    }
  },

  async loadDesktopPreview(
    mode: EntryKind,
    rootPath: string | null,
    plan: OrganizationPlanDto | null,
  ): Promise<DesktopPreviewDto> {
    const invoke = canUseRealCommands();
    const resolvedRoot = rootPath ?? defaultRoot(mode);
    if (!invoke) {
      return waitForMock(createDesktopPreview(mode, resolvedRoot, plan));
    }

    try {
      return await invoke<DesktopPreviewDto>("desktop_preview", {
        request: {
          mode,
          rootPath: resolvedRoot,
          plan,
        },
      });
    } catch {
      return createDesktopPreview(mode, resolvedRoot, plan);
    }
  },
};

type FileItemLike = Parameters<typeof createMockClassifications>[0][number];
