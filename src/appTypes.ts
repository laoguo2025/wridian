export type WorkspaceInfo = {
  vaultPath: string;
  runtimePath: string;
  filesRootPath: string;
  activeWorkRoot?: string | null;
  workRootConfigured: boolean;
  files: WorkFileNode[];
  knowledgeRootPath: string;
  activeKnowledgeRoot?: string | null;
  knowledgeRootConfigured: boolean;
  knowledgeFiles: WorkFileNode[];
};

export type WorkFileNode = {
  name: string;
  path: string;
  relativePath: string;
  library: "works" | "knowledge";
  folder: boolean;
  children: WorkFileNode[];
};

export type OpenFileResponse = {
  path: string;
  name: string;
  content: string;
};

export type SaveFileResponse = {
  ok: boolean;
  savedAt: string;
};

export type CreativeSkillSource = {
  available: boolean;
  path?: string | null;
};

export type CreativeSkillSources = {
  knowledgeOps: CreativeSkillSource;
};

export type CustomApiSettingsStatus = {
  configured: boolean;
  baseUrl?: string | null;
  model?: string | null;
  maskedKey?: string | null;
};

export type TestCustomApiResponse = {
  ok: boolean;
  message: string;
};

export type MemoryTreeNode = {
  id: string;
  kind: string;
  label: string;
  description: string;
  path?: string | null;
  content?: string | null;
  children: MemoryTreeNode[];
};

export type MemoryTreeState = {
  roots: MemoryTreeNode[];
};

export type KnowledgeGraphNode = {
  id: string;
  label: string;
  kind: "folder" | "card" | string;
  path?: string | null;
  group: string;
  size: number;
};

export type KnowledgeGraphEdge = {
  source: string;
  target: string;
  kind: string;
};

export type KnowledgeGraphState = {
  nodes: KnowledgeGraphNode[];
  edges: KnowledgeGraphEdge[];
  warnings: string[];
};
