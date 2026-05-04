export type EntryKind = "files" | "desktop";

export type WorkflowView = "plan" | "history" | "skills" | "models" | "desktop";

export type FileRisk = "low" | "medium" | "high";

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

export type ClassificationResultDto = {
  fileId: string;
  category: string;
  categoryKey?: string;
  confidence: number;
  evidence: string[];
  risk: FileRisk;
};

export type GeneratePlanRequestDto = {
  taskId: string;
  rootPath: string;
  mode: "by_category" | "desktop";
  classifications?: ClassificationResultDto[];
};

export type PlanSummaryDto = {
  filesConsidered: number;
  foldersToCreate: number;
  filesToMove: number;
  filesToRename: number;
};

export type ValidationIssueDto = {
  operationId?: string | null;
  message: string;
};

export type OperationRowDto = {
  operationId: string;
  operationType: "create_folder" | "move_file" | "rename_file";
  title: string;
  source?: string | null;
  target: string;
  reason: string;
  risk: FileRisk;
  selected: boolean;
  editableTarget: string;
  validationIssues: ValidationIssueDto[];
  conflictStatus: "none" | "warning" | "blocked";
  fileId?: string | null;
};

export type OrganizationPlanDto = {
  planId: string;
  taskId: string;
  rootPath: string;
  mode: "by_category" | "desktop";
  rows: OperationRowDto[];
  summary: PlanSummaryDto;
  createdAt: string;
  desktopPreview?: DesktopPreviewDto | null;
};

export type UserApprovalDto = {
  approved: boolean;
  approvedPlanId: string;
  approvedAt: string;
  actor?: string | null;
};

export type ExecutedOperationDto = {
  operationId: string;
  operationType: OperationRowDto["operationType"];
  source?: string | null;
  target: string;
  completedAt: string;
};

export type RollbackEntryDto = {
  batchId: string;
  operationId: string;
  action: "remove_created_folder" | "move_file_back" | "rename_file_back";
  from?: string | null;
  to?: string | null;
  createdAt: string;
};

export type ExecutionErrorDto = {
  operationId?: string | null;
  message: string;
};

export type ExecutionBatchDto = {
  batchId: string;
  planId: string;
  status: "completed" | "partially_failed" | "rejected" | "rolled_back";
  executedOperations: ExecutedOperationDto[];
  rollbackEntries: RollbackEntryDto[];
  errors: ExecutionErrorDto[];
  startedAt: string;
  finishedAt: string;
};

export type HistorySummaryDto = {
  batchId: string;
  planId: string;
  status: ExecutionBatchDto["status"];
  operationCount: number;
  errorCount: number;
  startedAt: string;
  finishedAt: string;
};

export type RollbackResultDto = {
  batchId: string;
  rolledBack: string[];
  errors: ExecutionErrorDto[];
};

export type SkillRule = {
  extension?: string | null;
  fileNameContains?: string | null;
  mimePrefix?: string | null;
  category: "Documents" | "Images" | "Videos" | "Audio" | "Archives" | "Installers" | "Code" | "Spreadsheets" | "Presentations" | "Pdf" | "Other";
  destinationHint?: string | null;
};

export type SkillDto = {
  id: string;
  name: string;
  enabled: boolean;
  rule: SkillRule;
  createdAt: string;
};

export type SkillUpdateProposalDto = {
  name: string;
  rule: SkillRule;
  enabled: boolean;
  evidence?: string[];
  sourceEventIds?: string[];
};

export type ModelSettingsDto = {
  provider: "mock" | "ollama" | "openai-compatible";
  cloudEnabled: boolean;
  baseUrl?: string;
  model?: string | null;
};

export type ModelSettingsListDto = {
  providers: Array<{
    provider: ModelSettingsDto["provider"];
    label: string;
    requiresBaseUrl: boolean;
    requiresApiKey: boolean;
    cloud: boolean;
  }>;
  savedSettings: ModelSettingsDto[];
};

export type ModelRuntimeTestResult = {
  ok: boolean;
  message: string;
};

export type DesktopPreviewPlatform = "macos" | "windows" | "other";

export type DesktopCapabilityFlagsDto = {
  previewOnly: boolean;
  supportsFileArchivePlan: boolean;
  supportsDesktopCanvasPreview: boolean;
  supportsIconCoordinateWriteback: boolean;
  supportsPixelPerfectLayout: boolean;
};

export type DesktopPreviewCanvasDto = {
  width: number;
  height: number;
  columns: number;
  rows: number;
  coordinateSpace: string;
};

export type DesktopPreviewGroupDto = {
  groupId: string;
  title: string;
  categoryKey: string;
  fileCount: number;
  totalSizeBytes: number;
  files: Array<{
    fileId: string;
    name: string;
    path: string;
    sizeBytes: number;
  }>;
};

export type DesktopPreviewZoneDto = {
  zoneId: string;
  title: string;
  categoryKey: string;
  archiveFolder: string;
  fileCount: number;
  canvasRect: {
    x: number;
    y: number;
    width: number;
    height: number;
  };
  fileIds: string[];
};

export type DesktopPreviewDto = {
  platform: DesktopPreviewPlatform;
  capabilities: DesktopCapabilityFlagsDto;
  canvas: DesktopPreviewCanvasDto;
  beforeGroups: DesktopPreviewGroupDto[];
  afterZones: DesktopPreviewZoneDto[];
};

export type WorkflowState = {
  entry: EntryKind | null;
  view: WorkflowView;
  currentTaskId: string | null;
  selectedRootPath: string | null;
  skippedCount: number;
  errorCount: number;
  errorMessage: string | null;
  files: FileItem[];
  classifications: ClassificationResultDto[];
  plan: OrganizationPlanDto | null;
  batches: ExecutionBatchDto[];
  skills: SkillDto[];
  modelSettings: ModelSettingsDto;
  modelTestMessage: string | null;
  desktopPreview: DesktopPreviewDto | null;
  editedOperationIds: string[];
  status:
    | "idle"
    | "selecting"
    | "scanning"
    | "scanned"
    | "classifying"
    | "classified"
    | "planning"
    | "cancelled"
    | "planned"
    | "executing"
    | "done"
    | "rolling-back"
    | "rolled-back";
};
