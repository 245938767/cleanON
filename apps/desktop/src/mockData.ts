import type {
  ClassificationResultDto,
  DesktopPreviewDto,
  EntryKind,
  ExecutionBatchDto,
  FileItem,
  HistorySummaryDto,
  ModelRuntimeTestResult,
  ModelSettingsDto,
  OperationRowDto,
  OrganizationPlanDto,
  RollbackResultDto,
  SkillDto,
  SkillUpdateProposalDto,
} from "./types";

const fileNames = [
  ["发票-三月.pdf", "财务票据", "PDF"],
  ["产品截图 18.png", "图片素材", "PNG"],
  ["会议纪要.docx", "工作文档", "DOCX"],
  ["下载的安装包.dmg", "安装包", "DMG"],
  ["archive-final.zip", "压缩归档", "ZIP"],
] as const;

export function createMockFiles(mode: EntryKind): FileItem[] {
  const root = defaultRoot(mode);
  return fileNames.map(([name, kind, ext], index) => ({
    id: `file-${index + 1}`,
    name,
    path: `${root}/${name}`,
    extension: ext.toLowerCase(),
    sizeBytes: [248_000, 1_800_000, 96_000, 142_000_000, 18_000_000][index],
    sizeLabel: ["248 KB", "1.8 MB", "96 KB", "142 MB", "18 MB"][index],
    modifiedAt: ["今天 10:24", "昨天 18:03", "周二 09:16", "3 天前", "上周"][index],
    kind: `${kind} · ${ext}`,
  }));
}

export function createMockClassifications(files: FileItem[]): ClassificationResultDto[] {
  const categories = [
    ["票据", "receipts"],
    ["图片", "images"],
    ["文档", "documents"],
    ["安装包", "installers"],
    ["压缩包", "archives"],
  ] as const;

  return files.map((file, index) => ({
    fileId: file.id,
    category: categories[index]?.[0] ?? "其他",
    categoryKey: categories[index]?.[1] ?? "other",
    confidence: [0.96, 0.92, 0.89, 0.98, 0.86][index] ?? 0.82,
    evidence: [`扩展名与${categories[index]?.[0] ?? "其他"}分类匹配`, "未读取文件正文"],
    risk: index === 3 ? "medium" : "low",
  }));
}

export function createMockPlan(
  mode: EntryKind,
  taskId: string,
  rootPath: string,
  files: FileItem[],
): OrganizationPlanDto {
  const rows: OperationRowDto[] = [
    row("op-folder-receipts", "create_folder", "创建票据目录", null, `${rootPath}/票据`, "票据类文件集中收纳", "low"),
    row(
      "op-move-receipt",
      "move_file",
      "移动发票-三月.pdf",
      `${rootPath}/发票-三月.pdf`,
      `${rootPath}/票据/发票-三月.pdf`,
      "命中票据分类，保留原文件名",
      "low",
      "file-1",
    ),
    row("op-folder-images", "create_folder", "创建图片素材目录", null, `${rootPath}/图片素材`, "图片素材集中收纳", "low"),
    row(
      "op-move-shot",
      "move_file",
      "移动产品截图 18.png",
      `${rootPath}/产品截图 18.png`,
      `${rootPath}/图片素材/产品截图 18.png`,
      "命中图片分类，适合归档到素材目录",
      "low",
      "file-2",
    ),
    row("op-folder-installers", "create_folder", "创建安装包目录", null, `${rootPath}/安装包`, "安装包与压缩归档分开", "low"),
    row(
      "op-move-installer",
      "move_file",
      "移动下载的安装包.dmg",
      `${rootPath}/下载的安装包.dmg`,
      `${rootPath}/安装包/下载的安装包.dmg`,
      "安装包体积较大，执行前建议确认目标",
      "medium",
      "file-4",
    ),
  ];

  if (mode === "desktop") {
    rows.push(
      row(
        "op-move-desktop-archive",
        "move_file",
        "桌面归档 archive-final.zip",
        `${rootPath}/archive-final.zip`,
        `${rootPath}/桌面归档/压缩包/archive-final.zip`,
        "桌面整理仅归档文件，不承诺图标坐标排布",
        "medium",
        "file-5",
      ),
    );
  }

  return {
    planId: `00000000-0000-4000-8000-${mode === "desktop" ? "000000000002" : "000000000001"}`,
    taskId,
    rootPath,
    mode: mode === "desktop" ? "desktop" : "by_category",
    rows,
    summary: {
      filesConsidered: files.length,
      foldersToCreate: rows.filter((operation) => operation.operationType === "create_folder").length,
      filesToMove: rows.filter((operation) => operation.operationType === "move_file").length,
      filesToRename: rows.filter((operation) => operation.operationType === "rename_file").length,
    },
    createdAt: new Date().toISOString(),
    desktopPreview: mode === "desktop" ? createDesktopPreview(mode, rootPath, null) : null,
  };
}

export function createMockBatch(plan: OrganizationPlanDto): ExecutionBatchDto {
  const selectedRows = plan.rows.filter((operation) => operation.selected);
  const now = new Date().toISOString();

  return {
    batchId: `batch-${Date.now()}`,
    planId: plan.planId,
    status: "completed",
    executedOperations: selectedRows.map((operation) => ({
      operationId: operation.operationId,
      operationType: operation.operationType,
      source: operation.source,
      target: operation.editableTarget || operation.target,
      completedAt: now,
    })),
    rollbackEntries: selectedRows.map((operation) => ({
      batchId: `batch-${Date.now()}`,
      operationId: operation.operationId,
      action:
        operation.operationType === "create_folder"
          ? "remove_created_folder"
          : operation.operationType === "rename_file"
            ? "rename_file_back"
            : "move_file_back",
      from: operation.editableTarget || operation.target,
      to: operation.source ?? null,
      createdAt: now,
    })),
    errors: [],
    startedAt: now,
    finishedAt: now,
  };
}

export function createHistorySummary(batch: ExecutionBatchDto): HistorySummaryDto {
  return {
    batchId: batch.batchId,
    planId: batch.planId,
    status: batch.status,
    operationCount: batch.executedOperations.length,
    errorCount: batch.errors.length,
    startedAt: batch.startedAt,
    finishedAt: batch.finishedAt,
  };
}

export function createRollbackResult(batch: ExecutionBatchDto): RollbackResultDto {
  return {
    batchId: batch.batchId,
    rolledBack: batch.rollbackEntries.map((entry) => entry.operationId),
    errors: [],
  };
}

export function createMockSkills(): SkillDto[] {
  return [
    {
      id: "skill-receipts",
      name: "发票归入票据",
      enabled: true,
      rule: { fileNameContains: "发票", category: "Documents", destinationHint: "票据" },
      createdAt: new Date(Date.now() - 86400000).toISOString(),
    },
    {
      id: "skill-installers",
      name: "安装包独立收纳",
      enabled: true,
      rule: { extension: "dmg", category: "Installers", destinationHint: "安装包" },
      createdAt: new Date(Date.now() - 172800000).toISOString(),
    },
  ];
}

export function createSkillFromProposal(proposal: SkillUpdateProposalDto): SkillDto {
  return {
    id: `skill-${Date.now()}`,
    name: proposal.name,
    enabled: proposal.enabled,
    rule: proposal.rule,
    createdAt: new Date().toISOString(),
  };
}

export function createDefaultModelSettings(): ModelSettingsDto {
  return {
    provider: "ollama",
    cloudEnabled: false,
    baseUrl: "http://127.0.0.1:11434",
    model: "llama3.2",
  };
}

export function createModelTestResult(settings: ModelSettingsDto, apiKey: string): ModelRuntimeTestResult {
  if (settings.cloudEnabled && apiKey.trim().length < 8) {
    return { ok: false, message: "云模型测试需要临时 API Key；不会保存。" };
  }
  return {
    ok: true,
    message: settings.cloudEnabled ? "云端连接测试通过，Key 已丢弃。" : "本地模型设置可用，无需 API Key。",
  };
}

export function createDesktopPreview(mode: EntryKind, rootPath: string, plan: OrganizationPlanDto | null): DesktopPreviewDto {
  const rows = plan?.rows.filter((operation) => operation.selected) ?? [];
  const zones = rows.length
    ? rows.map((row, index) => ({
        zoneId: `zone-${index}`,
        title: row.title,
        categoryKey: row.risk,
        archiveFolder: dirnameForMock(row.editableTarget || row.target),
        fileCount: row.operationType === "create_folder" ? 0 : 1,
        canvasRect: { x: 64 + index * 36, y: 70 + index * 24, width: 260, height: 120 },
        fileIds: row.fileId ? [row.fileId] : [],
      }))
    : [
        {
          zoneId: "zone-work",
          title: "工作文档",
          categoryKey: "documents",
          archiveFolder: `${rootPath}/Desktop Archive/Documents`,
          fileCount: mode === "desktop" ? 2 : 0,
          canvasRect: { x: 72, y: 76, width: 420, height: 300 },
          fileIds: [],
        },
        {
          zoneId: "zone-media",
          title: "图片素材",
          categoryKey: "images",
          archiveFolder: `${rootPath}/Desktop Archive/Images`,
          fileCount: mode === "desktop" ? 1 : 0,
          canvasRect: { x: 528, y: 76, width: 360, height: 300 },
          fileIds: [],
        },
        {
          zoneId: "zone-archive",
          title: "归档暂存",
          categoryKey: "archives",
          archiveFolder: `${rootPath}/Desktop Archive/Archives`,
          fileCount: mode === "desktop" ? 2 : 0,
          canvasRect: { x: 924, y: 76, width: 360, height: 300 },
          fileIds: [],
        },
      ];

  return {
    platform: mode === "desktop" ? "windows" : "macos",
    capabilities: {
      previewOnly: true,
      supportsFileArchivePlan: true,
      supportsDesktopCanvasPreview: true,
      supportsIconCoordinateWriteback: false,
      supportsPixelPerfectLayout: false,
    },
    canvas: {
      width: 1440,
      height: 900,
      columns: 12,
      rows: 8,
      coordinateSpace: "preview",
    },
    beforeGroups: [
      {
        groupId: "group-preview",
        title: "当前桌面项目",
        categoryKey: "mixed",
        fileCount: rows.length,
        totalSizeBytes: 0,
        files: rows
          .filter((row) => row.fileId)
          .map((row) => ({
            fileId: row.fileId ?? row.operationId,
            name: basenameForMock(row.source ?? row.target),
            path: row.source ?? row.target,
            sizeBytes: 0,
          })),
      },
    ],
    afterZones: zones,
  };
}

export function defaultRoot(entry: EntryKind): string {
  return entry === "desktop" ? "~/Desktop" : "~/Downloads";
}

function row(
  operationId: string,
  operationType: OperationRowDto["operationType"],
  title: string,
  source: string | null,
  target: string,
  reason: string,
  risk: OperationRowDto["risk"],
  fileId: string | null = null,
): OperationRowDto {
  return {
    operationId,
    operationType,
    title,
    source,
    target,
    reason,
    risk,
    selected: true,
    editableTarget: target,
    validationIssues: [],
    conflictStatus: "none",
    fileId,
  };
}

function basenameForMock(path: string): string {
  return path.split("/").filter(Boolean).pop() ?? path;
}

function dirnameForMock(path: string): string {
  const parts = path.split("/").filter(Boolean);
  parts.pop();
  if (path.startsWith("~")) {
    return `~/${parts.slice(1).join("/")}`;
  }
  return `/${parts.join("/")}`;
}
