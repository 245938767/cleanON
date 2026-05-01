export type EntryKind = "files" | "desktop";

export type FileItem = {
  id: string;
  name: string;
  path: string;
  extension?: string;
  sizeBytes?: number;
  sizeLabel: string;
  modifiedAt?: string | null;
  kind: string;
};

export type ScanFolderRequest = {
  taskId: string;
  rootPath: string;
  recursive: boolean;
  maxDepth?: number;
  includeHidden: boolean;
  followSymlinks: boolean;
};

export type ScanFolderResponse = {
  taskId: string;
  rootPath: string;
  files: FileItem[];
  status: "completed" | "cancelled";
  skippedCount: number;
  errorCount: number;
};

export type CancelScanResponse = {
  taskId: string;
  cancelled: boolean;
};

export type ClassificationResult = {
  fileId: string;
  category: string;
  confidence: number;
  evidence: string[];
  risk: "low" | "medium" | "high";
};

export type PlanOperation = {
  id: string;
  type: "CreateFolder" | "MoveFile" | "RenameFile";
  title: string;
  source?: string;
  target: string;
  risk: "low" | "medium" | "high";
};

export type OrganizationPlan = {
  id: string;
  mode: EntryKind;
  rootPath: string;
  summary: string;
  operations: PlanOperation[];
};

export type ExecutionBatch = {
  id: string;
  executedAt: string;
  operationCount: number;
  rollbackAvailable: boolean;
};

export type WorkflowState = {
  entry: EntryKind | null;
  currentTaskId: string | null;
  selectedRootPath: string | null;
  skippedCount: number;
  errorCount: number;
  errorMessage: string | null;
  files: FileItem[];
  classifications: ClassificationResult[];
  plan: OrganizationPlan | null;
  batch: ExecutionBatch | null;
  status:
    | "idle"
    | "selecting"
    | "scanning"
    | "scanned"
    | "classifying"
    | "classified"
    | "cancelled"
    | "planned"
    | "executing"
    | "done"
    | "rolled-back";
};
