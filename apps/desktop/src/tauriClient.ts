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
  ModelSettingsListDto,
  ModelSettingsDto,
  OperationRowDto,
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

function waitForMock<T>(value: T): Promise<T> {
  return new Promise((resolve) => {
    window.setTimeout(() => resolve(value), commandDelay);
  });
}

function canUseRealCommands(): TauriInvoke | null {
  return getInvoke();
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

  async patchPlan(plan: OrganizationPlanDto, operations: Array<Pick<OperationRowDto, "operationId"> & { selected?: boolean; editableTarget?: string }>): Promise<OrganizationPlanDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(applyMockPatch(plan, operations));
    }

    return invoke<OrganizationPlanDto>("patch_plan", {
      plan,
      patch: { operations },
    });
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

    return invoke<HistorySummaryDto[]>("list_execution_batches");
  },

  async loadExecutionBatch(batchId: string): Promise<ExecutionBatchDto | null> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(mockBatches.find((batch) => batch.batchId === batchId) ?? null);
    }

    return invoke<ExecutionBatchDto | null>("load_execution_batch", { batchId });
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

    return invoke<RollbackResultDto>("rollback_batch_by_id", { batchId: batch.batchId });
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

    return invoke<SkillDto>("set_skill_enabled", { id: skill.id, enabled });
  },

  async deleteSkill(skillId: string): Promise<void> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      mockSkills = mockSkills.filter((skill) => skill.id !== skillId);
      return waitForMock(undefined);
    }

    await invoke("delete_skill", { id: skillId });
  },

  async loadModelSettings(): Promise<ModelSettingsDto> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(mockModelSettings);
    }

    const list = await invoke<ModelSettingsListDto>("list_model_settings");
    return list.savedSettings[0] ?? mockModelSettings;
  },

  async saveModelSettings(settings: ModelSettingsDto): Promise<ModelSettingsDto> {
    const invoke = canUseRealCommands();
    const sanitized = { ...settings };
    mockModelSettings = sanitized;
    if (!invoke) {
      return waitForMock(sanitized);
    }

    return invoke<ModelSettingsDto>("save_model_settings", { settings: sanitized });
  },

  async testModelRuntime(settings: ModelSettingsDto, apiKey: string): Promise<ModelRuntimeTestResult> {
    const invoke = canUseRealCommands();
    if (!invoke) {
      return waitForMock(createModelTestResult(settings, apiKey));
    }

    const request = { settings, apiKey };
    const connection = await invoke<{ requestValid: boolean; message: string }>("test_model_connection", { request });
    if (!connection.requestValid) {
      return { ok: false, message: connection.message };
    }
    const jsonOutput = await invoke<{ valid: boolean; summary: string; categoriesCount: number }>("test_model_json_output", { request });
    return {
      ok: jsonOutput.valid,
      message: `${connection.message} JSON 输出 ${jsonOutput.categoriesCount} 项：${jsonOutput.summary}`,
    };
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

    if (plan?.desktopPreview) {
      return plan.desktopPreview;
    }
    return createDesktopPreview(mode, resolvedRoot, plan);
  },
};

type FileItemLike = Parameters<typeof createMockClassifications>[0][number];

function applyMockPatch(
  plan: OrganizationPlanDto,
  operations: Array<Pick<OperationRowDto, "operationId"> & { selected?: boolean; editableTarget?: string }>,
): OrganizationPlanDto {
  const patched = {
    ...plan,
    rows: plan.rows.map((row) => {
      const patch = operations.find((item) => item.operationId === row.operationId);
      if (!patch) {
        return row;
      }
      const editableTarget = patch.editableTarget ?? row.editableTarget;
      const validationIssues = editableTarget.trim()
        ? row.validationIssues.filter((issue) => issue.message !== "目标路径不能为空")
        : [{ operationId: row.operationId, message: "目标路径不能为空" }];
      return {
        ...row,
        selected: patch.selected ?? row.selected,
        editableTarget,
        validationIssues,
        conflictStatus: validationIssues.length ? "blocked" as const : "none" as const,
      };
    }),
  };
  return {
    ...patched,
    summary: {
      filesConsidered: patched.summary.filesConsidered,
      foldersToCreate: patched.rows.filter((row) => row.selected && row.operationType === "create_folder").length,
      filesToMove: patched.rows.filter((row) => row.selected && row.operationType === "move_file").length,
      filesToRename: patched.rows.filter((row) => row.selected && row.operationType === "rename_file").length,
    },
  };
}
