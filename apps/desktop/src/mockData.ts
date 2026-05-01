import type {
  ClassificationResult,
  EntryKind,
  ExecutionBatch,
  FileItem,
  OrganizationPlan,
} from "./types";

const fileNames = [
  ["发票-三月.pdf", "财务票据", "PDF"],
  ["产品截图 18.png", "图片素材", "PNG"],
  ["会议纪要.docx", "工作文档", "DOCX"],
  ["下载的安装包.dmg", "安装包", "DMG"],
  ["archive-final.zip", "压缩归档", "ZIP"],
] as const;

export function createMockFiles(mode: EntryKind): FileItem[] {
  const root = mode === "desktop" ? "~/Desktop" : "~/Downloads";
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

export function createMockClassifications(files: FileItem[]): ClassificationResult[] {
  const categories = ["票据", "图片", "文档", "安装包", "压缩包"];
  return files.map((file, index) => ({
    fileId: file.id,
    category: categories[index] ?? "其他",
    confidence: [0.96, 0.92, 0.89, 0.98, 0.86][index] ?? 0.82,
    evidence: [`扩展名与${categories[index] ?? "其他"}分类匹配`, "未读取文件正文"],
    risk: index === 3 ? "medium" : "low",
  }));
}

export function createMockPlan(mode: EntryKind, files: FileItem[]): OrganizationPlan {
  const rootPath = mode === "desktop" ? "~/Desktop" : "~/Downloads";
  const label = mode === "desktop" ? "桌面" : "文件夹";

  return {
    id: `plan-${mode}-mock`,
    mode,
    rootPath,
    summary: `建议整理 ${files.length} 个${label}项目，创建 4 个分类目录，移动 5 个文件。`,
    operations: [
      {
        id: "op-1",
        type: "CreateFolder",
        title: "创建票据目录",
        target: `${rootPath}/票据`,
        risk: "low",
      },
      {
        id: "op-2",
        type: "MoveFile",
        title: "归档发票文件",
        source: `${rootPath}/发票-三月.pdf`,
        target: `${rootPath}/票据/发票-三月.pdf`,
        risk: "low",
      },
      {
        id: "op-3",
        type: "CreateFolder",
        title: "创建素材目录",
        target: `${rootPath}/图片素材`,
        risk: "low",
      },
      {
        id: "op-4",
        type: "MoveFile",
        title: "移动截图素材",
        source: `${rootPath}/产品截图 18.png`,
        target: `${rootPath}/图片素材/产品截图 18.png`,
        risk: "low",
      },
      {
        id: "op-5",
        type: "MoveFile",
        title: "收纳安装包",
        source: `${rootPath}/下载的安装包.dmg`,
        target: `${rootPath}/安装包/下载的安装包.dmg`,
        risk: "medium",
      },
    ],
  };
}

export function createMockBatch(plan: OrganizationPlan): ExecutionBatch {
  return {
    id: `batch-${plan.id}`,
    executedAt: new Date().toLocaleString("zh-CN", {
      month: "2-digit",
      day: "2-digit",
      hour: "2-digit",
      minute: "2-digit",
    }),
    operationCount: plan.operations.length,
    rollbackAvailable: true,
  };
}
