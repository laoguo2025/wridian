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
  editable: boolean;
  previewType: string;
};

export type PreviewFileResponse = {
  path: string;
  name: string;
  content?: string | null;
  editable: boolean;
  previewType: "image" | "pdf" | "text" | "external" | string;
};

export type PreviewAssetResponse = {
  url: string;
  mimeType: string;
};

export type SaveFileResponse = {
  ok: boolean;
  savedAt: string;
};

export type CreativeSkillSource = {
  available: boolean;
  source: "builtin" | string;
  label: string;
  path?: string | null;
};

export type CreativeSkillSources = {
  workDecompose: CreativeSkillSource;
  knowledgeCard: CreativeSkillSource;
  authorDistill: CreativeSkillSource;
};

export type TestModelProviderResponse = {
  ok: boolean;
  message: string;
};

export type ConfiguredModelStatus = {
  id: string;
  label: string;
  providerId: string;
  providerName: string;
  protocol: "openai-compatible" | "anthropic" | "google" | string;
  model: string;
};

export type ModelProviderStatus = {
  id: string;
  presetKey?: string | null;
  providerName: string;
  providerType?: string | null;
  protocol: "openai-compatible" | "anthropic" | "google" | string;
  authStyle?: "api_key" | "auth_token" | "oauth_external" | string;
  configured: boolean;
  baseUrl?: string | null;
  models: string[];
  maskedKey?: string | null;
  extraEnv?: Record<string, string>;
};

export type ModelAccountsStatus = {
  activeModelId?: string | null;
  activeModelLabel?: string | null;
  configuredModels: ConfiguredModelStatus[];
  providers: ModelProviderStatus[];
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
  relativePath?: string | null;
  group: string;
  size: number;
  aliases?: string[];
  tags?: string[];
  sourceRefs?: string[];
  outgoingCount?: number;
  backlinkCount?: number;
  unresolvedCount?: number;
  backlinkSources?: string[];
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

export type KnowledgeCacheResponse = {
  ok: boolean;
  manifestPath: string;
  generatedAt: string;
  fileCount: number;
  linkCount: number;
  unresolvedLinkCount: number;
  warnings: string[];
};

export type KnowledgeHotCacheResponse = {
  ok: boolean;
  path: string;
  updatedAt: string;
  fileCount: number;
  lineCount: number;
  warnings: string[];
};

export type KnowledgeFoldResponse = {
  ok: boolean;
  path: string;
  createdAt: string;
  sourceCount: number;
  warnings: string[];
};

export type KnowledgeHealthResponse = {
  ok: boolean;
  checkedAt: string;
  score: number;
  skillMaturityScore: number;
  summary: KnowledgeHealthSummary;
  issues: KnowledgeHealthIssue[];
  skillCandidates: KnowledgeSkillCandidate[];
  warnings: string[];
};

export type KnowledgeHealthWorkflowResponse = KnowledgeHealthResponse & {
  reportPath: string;
  reportRelativePath: string;
  hotPath: string;
  foldPath: string;
  manifestPath: string;
  autoFixes: KnowledgeHealthFixItem[];
  pendingFixes: KnowledgeHealthFixItem[];
};

export type KnowledgeHealthFixResponse = KnowledgeHealthWorkflowResponse & {
  appliedFixes: KnowledgeHealthFixItem[];
};

export type KnowledgeHealthFixItem = {
  id: string;
  title: string;
  detail: string;
  path?: string | null;
  risk: "low" | "high" | string;
};

export type KnowledgeHealthSummary = {
  fileCount: number;
  linkCount: number;
  unresolvedLinkCount: number;
  frontmatterFileCount: number;
  taggedFileCount: number;
  sourceCoverageCount: number;
  formalSkillFileCount: number;
  skillCandidateCount: number;
  orphanFileCount: number;
  generatedFileCount: number;
};

export type KnowledgeHealthIssue = {
  severity: "high" | "medium" | "low" | string;
  title: string;
  detail: string;
  path?: string | null;
};

export type KnowledgeSkillCandidate = {
  path: string;
  relativePath: string;
  title: string;
  score: number;
  reasons: string[];
  missing: string[];
};

export type KnowledgeSearchHit = {
  path: string;
  relativePath: string;
  title: string;
  snippet: string;
  score: number;
  tags: string[];
  aliases: string[];
  outgoingCount: number;
  backlinkCount: number;
  unresolvedCount: number;
  reasons: string[];
};

export type RelevantNote = {
  kind: "knowledge" | "draft" | string;
  path: string;
  relativePath?: string | null;
  title: string;
  snippet: string;
  score: number;
  hasOutgoingLinks: boolean;
  hasBacklinks: boolean;
  reasons: string[];
};

export type BridgeRelationAction =
  | "referencesKnowledge"
  | "adoptsKnowledge"
  | "derivedFromKnowledge"
  | "abstractedFromDraft"
  | "excerptedFromProject"
  | "distilledFromMemory";

export type BridgeRelationInput = {
  action: BridgeRelationAction;
  targetLibrary: "works" | "knowledge";
  targetPath: string;
  sourceLibrary: "works" | "knowledge" | "creative_memory";
  sourceRelativePath: string;
  sourceTitle?: string | null;
};

export type BridgeRelationResponse = {
  ok: boolean;
  targetPath: string;
  field: string;
  value: string;
  inserted: boolean;
  message: string;
  warnings: string[];
};

export type MetadataIndexState = {
  libraries: MetadataLibraryIndex[];
  warnings: string[];
};

export type MetadataLibraryIndex = {
  library: "works" | "knowledge" | string;
  rootPath?: string | null;
  files: MetadataFile[];
  links: MetadataLink[];
  backlinks: MetadataBacklink[];
  unresolvedLinks: MetadataUnresolvedLink[];
};

export type MetadataFile = {
  id: string;
  library: "works" | "knowledge" | string;
  path: string;
  relativePath: string;
  title: string;
  aliases: string[];
  tags: string[];
  frontmatter: Record<string, string[]>;
  outgoingLinks: MetadataLink[];
  backlinks: MetadataBacklink[];
};

export type MetadataLink = {
  sourceId: string;
  sourceLibrary: "works" | "knowledge" | string;
  sourcePath: string;
  sourceRelativePath: string;
  rawTarget: string;
  normalizedTarget: string;
  displayText?: string | null;
  section?: string | null;
  embed: boolean;
  frontmatterField?: string | null;
  targetId?: string | null;
  targetLibrary?: "works" | "knowledge" | string | null;
  targetPath?: string | null;
  targetRelativePath?: string | null;
  resolved: boolean;
  ambiguous: boolean;
};

export type MetadataBacklink = {
  targetId: string;
  targetLibrary: "works" | "knowledge" | string;
  targetPath: string;
  targetRelativePath: string;
  sourceId: string;
  sourceLibrary: "works" | "knowledge" | string;
  sourcePath: string;
  sourceRelativePath: string;
  rawTarget: string;
  frontmatterField?: string | null;
  embed: boolean;
};

export type MetadataUnresolvedLink = {
  sourceId: string;
  sourceLibrary: "works" | "knowledge" | string;
  sourcePath: string;
  sourceRelativePath: string;
  rawTarget: string;
  normalizedTarget: string;
  frontmatterField?: string | null;
  embed: boolean;
  reason: "not_found" | "ambiguous" | string;
};
