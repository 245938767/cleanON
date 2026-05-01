import {
  createMockBatch,
  createMockClassifications,
  createMockFiles,
  createMockPlan,
} from "./mockData";
import type {
  CancelScanResponse,
  ClassificationResult,
  EntryKind,
  ExecutionBatch,
  FileItem,
  OrganizationPlan,
  ScanFolderRequest,
  ScanFolderResponse,
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

const commandDelay = 260;

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

export const tauriClient = {
  usesMockCommands(): boolean {
    return !getInvoke() || !realCommandsEnabled();
  },

  async selectScanFolder(): Promise<string | null> {
    const invoke = getInvoke();
    if (!invoke || !realCommandsEnabled()) {
      return waitForMock(null);
    }

    return invoke<string | null>("select_scan_folder");
  },

  async scanFolder(mode: EntryKind, request: ScanFolderRequest): Promise<ScanFolderResponse> {
    const invoke = getInvoke();
    if (!invoke || !realCommandsEnabled()) {
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
    const invoke = getInvoke();
    if (!invoke || !realCommandsEnabled()) {
      return waitForMock({ taskId, cancelled: true });
    }

    return invoke<CancelScanResponse>("cancel_scan", { taskId });
  },

  async classifyFiles(taskId: string, rootPath: string, files: FileItem[]): Promise<ClassificationResult[]> {
    const invoke = getInvoke();
    if (!invoke || !realCommandsEnabled()) {
      return waitForMock(createMockClassifications(files));
    }

    return invoke<ClassificationResult[]>("classify_files", {
      request: {
        taskId,
        rootPath,
      },
    });
  },

  async generatePlan(
    mode: EntryKind,
    files: FileItem[],
    classifications: ClassificationResult[],
  ): Promise<OrganizationPlan> {
    const invoke = getInvoke();
    if (!invoke || !realCommandsEnabled()) {
      return waitForMock(createMockPlan(mode, files));
    }

    return invoke<OrganizationPlan>("generate_plan", {
      taskId: `task-${Date.now()}`,
      rootPath: mode === "desktop" ? "~/Desktop" : "~/Downloads",
      classifications,
      mode: mode === "desktop" ? "Desktop" : "ByCategory",
    });
  },

  async executeConfirmedPlan(plan: OrganizationPlan): Promise<ExecutionBatch> {
    const invoke = getInvoke();
    if (!invoke || !realCommandsEnabled()) {
      return waitForMock(createMockBatch(plan));
    }

    return invoke<ExecutionBatch>("execute_confirmed_plan", {
      plan,
      approval: {
        plan_id: plan.id,
        approved_at: new Date().toISOString(),
        approved_operation_ids: plan.operations.map((operation) => operation.id),
      },
    });
  },

  async rollbackBatch(batch: ExecutionBatch): Promise<ExecutionBatch> {
    const invoke = getInvoke();
    if (!invoke || !realCommandsEnabled()) {
      return waitForMock({ ...batch, rollbackAvailable: false });
    }

    await invoke("rollback_batch", { batch });
    return { ...batch, rollbackAvailable: false };
  },
};
