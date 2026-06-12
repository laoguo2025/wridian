import { KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  restorePromptPillsFromMessage,
  type ChatMessage,
} from "./chat/messageRepository";
import { useChatManager, type ChatDraftEdit } from "./chat/chatManager";
import { ChatPanel } from "./chat/ChatPanel";
import {
  getProjectState,
  selectProject,
  type ProjectState,
} from "./chat/projectContext";
import {
  buildPromptSuggestions,
  createReferencedFileContentPromptPill,
  createFileContentPromptPill,
  createFilePromptPill,
  createImagePromptPill,
  createPromptPillFromSuggestion,
  createSelectionPromptPill,
  upsertPromptContextPill,
  type PromptContextPill,
} from "./chat/promptContext";
import {
  createDraftReplaceGuardReport,
  describeDraftReplaceSkip,
} from "./editor/draftReplaceGuard";
import { DraftEditor, readContentEditableSelection, setContentEditableCaret, type TextSelection } from "./editor/DraftEditor";
import { baseName, detectDraftKind } from "./editor/draftKind";
import { libraryFolderTooltip } from "./libraryToolbar";
import {
  CREATIVE_SKILLS,
  DEFAULT_CREATIVE_SKILL_STATE,
  type CreativeSkillId,
} from "./creativeSkills";
import { MemoryDrawer } from "./memory/MemoryDrawer";
import { KnowledgeGraphDrawer } from "./knowledge/KnowledgeGraphDrawer";
import { buildKnowledgeSuggestionIndex } from "./knowledge/knowledgeSuggestions";
import { ModelSettingsDialog } from "./settings/ModelSettingsDialog";
import { CreativeSkillsDrawer } from "./skills/CreativeSkillsDrawer";
import { FilePreviewViewer, type FilePreviewViewModel } from "./viewer/FilePreviewViewer";
import { FileContextMenuView, FileNodeView, type FileContextMenu } from "./files/FileTree";
import {
  DarkThemeIcon,
  FolderPlusIcon,
  FontSizeIcon,
  KnowledgeGraphIcon,
  LightThemeIcon,
  LightningIcon,
  MemoryTreeIcon,
  ModelConfigIcon,
  PencilIcon,
  SettingsIcon,
  WorkFolderIcon,
} from "./icons";
import { clamp } from "./numberUtils";
import {
  createWorkFile,
  createWorkFolder,
  duplicateWorkNode,
  initWorkspace,
  openWorkFile,
  previewWorkAsset,
  previewWorkFile,
  renameWorkNode,
  saveWorkFile,
  setLibraryRoot,
  trashWorkNode,
} from "./workspace/workspaceClient";
import type {
  BridgeRelationAction,
  BridgeRelationResponse,
  CreativeSkillSources,
  ConfiguredModelStatus,
  KnowledgeGraphState,
  KnowledgeHealthFixResponse,
  KnowledgeHealthWorkflowResponse,
  MemoryTreeState,
  ModelAccountsStatus,
  RelevantNote,
  WorkFileNode,
  WorkspaceInfo,
} from "./appTypes";
import "./App.css";

type Theme = "light" | "dark";
type FontSizeMode = "default" | "large" | "max";
type SaveStatus = "idle" | "dirty" | "saving" | "saved" | "error";
type WorkspaceLibrary = "works" | "knowledge";

type DraftEdit = ChatDraftEdit;
type FilePreviewState = FilePreviewViewModel;

type SendPromptSnapshotInput = {
  content: string;
  contextPills: PromptContextPill[];
  draftSelection: TextSelection;
  overrideSelectedText?: string;
  prompt: string;
  text?: string;
};

type SelectionActionPosition = {
  left: number;
  top: number;
};

type BridgeCandidate = {
  action: BridgeRelationAction;
  label: string;
  sourceLibrary: "works" | "knowledge" | "creative_memory";
  sourcePath: string;
  sourceRelativePath: string;
  sourceTitle: string;
  targetLibrary: WorkspaceLibrary;
  targetPath: string;
};

const DEFAULT_LEFT_PANE_WIDTH = 218;
const DEFAULT_RIGHT_PANE_WIDTH = 332;
const MIN_LEFT_PANE_WIDTH = 168;
const MAX_LEFT_PANE_WIDTH = 360;
const MIN_RIGHT_PANE_WIDTH = 240;
const MAX_RIGHT_PANE_WIDTH = 460;
const MIN_WRITING_PANE_WIDTH = 360;
const WORKSPACE_RESIZER_WIDTH = 12;
const WORKSPACE_RESIZER_COUNT = 2;
const BUILTIN_CREATIVE_SKILL_SOURCES: CreativeSkillSources = {
  workDecompose: {
    available: true,
    source: "builtin-resource",
    label: "作品拆解",
  },
  knowledgeCard: {
    available: true,
    source: "builtin-resource",
    label: "知识卡提炼",
  },
  authorDistill: {
    available: true,
    source: "builtin-resource",
    label: "大神蒸馏",
  },
};
const FONT_SIZE_SCALE: Record<FontSizeMode, number> = {
  default: 1,
  large: 1.12,
  max: 1.25,
};
const THEME_STORAGE_KEY = "wridian.theme";
const FONT_SIZE_STORAGE_KEY = "wridian.fontSizeMode";

function storedTheme(): Theme {
  try {
    return window.localStorage.getItem(THEME_STORAGE_KEY) === "dark" ? "dark" : "light";
  } catch {
    return "light";
  }
}

function storedFontSizeMode(): FontSizeMode {
  try {
    const value = window.localStorage.getItem(FONT_SIZE_STORAGE_KEY);
    return value === "large" || value === "max" ? value : "default";
  } catch {
    return "default";
  }
}

function fileExtension(path: string) {
  const match = /\.([^.\\/]+)$/.exec(path);
  return match ? match[1].toLowerCase() : "";
}

function isEditableWorkspaceFile(path: string) {
  return ["md", "markdown", "txt", "docx"].includes(fileExtension(path));
}

function isMarkdownWorkspaceFile(path: string) {
  return ["md", "markdown"].includes(fileExtension(path));
}

function isTextContextFile(path: string) {
  return ["md", "markdown", "txt", "docx", "csv", "json", "yaml", "yml"].includes(fileExtension(path));
}

function filePreviewType(path: string): FilePreviewState["type"] {
  const extension = fileExtension(path);
  if (["png", "jpg", "jpeg", "webp", "gif", "svg", "bmp"].includes(extension)) return "image";
  if (extension === "pdf") return "pdf";
  if (["csv", "json", "yaml", "yml"].includes(extension)) return "text";
  if (["doc", "wps"].includes(extension)) return "word-legacy";
  return "external";
}

function flattenFileNodes(nodes: WorkFileNode[]): WorkFileNode[] {
  return nodes.flatMap((node) => [node, ...flattenFileNodes(node.children)]);
}

function findFileNodeByPath(nodes: WorkFileNode[], path: string) {
  return flattenFileNodes(nodes).find((node) => !node.folder && node.path === path);
}

function flattenMemoryNodes(nodes: MemoryTreeState["roots"]): MemoryTreeState["roots"] {
  return nodes.flatMap((node) => [node, ...flattenMemoryNodes(node.children)]);
}

function findMemoryNodeByPath(nodes: MemoryTreeState["roots"], path: string) {
  return flattenMemoryNodes(nodes).find((node) => node.path === path);
}

function memoryRelativePath(path: string) {
  const normalized = path.replace(/\\/g, "/");
  const marker = "/memory-tree/";
  const index = normalized.toLowerCase().indexOf(marker);
  return index >= 0 ? normalized.slice(index + marker.length) : "";
}

function createCocreationRequestId() {
  return `cocreate-${Date.now()}-${Math.random().toString(36).slice(2, 8)}`;
}

function createSendPromptSnapshot(input: SendPromptSnapshotInput) {
  const selectionStart = Math.max(0, Math.min(input.draftSelection.start, input.content.length));
  const selectionEnd = Math.max(selectionStart, Math.min(input.draftSelection.end, input.content.length));
  const selectedText = (input.overrideSelectedText ?? input.content.slice(selectionStart, selectionEnd)).trim();
  const contextPills = input.contextPills.map(clonePromptContextPill);
  const selectionAlreadyIncluded = contextPills.some((pill) => pill.kind === "selection");
  if (selectedText && !selectionAlreadyIncluded) {
    contextPills.push(createSelectionPromptPill(selectedText, { start: selectionStart, end: selectionEnd }));
  }
  return {
    content: input.content,
    contextPills,
    selectedText,
    text: (input.text ?? input.prompt).trim(),
  };
}

function clonePromptContextPill(pill: PromptContextPill): PromptContextPill {
  return {
    ...pill,
    range: pill.range ? { ...pill.range } : undefined,
  };
}

function App() {
  const [theme, setTheme] = useState<Theme>(() => storedTheme());
  const [fontSizeMode, setFontSizeMode] = useState<FontSizeMode>(() => storedFontSizeMode());
  const [fontSizeMenuOpen, setFontSizeMenuOpen] = useState(false);
  const [memoryOpen, setMemoryOpen] = useState(false);
  const [knowledgeGraphOpen, setKnowledgeGraphOpen] = useState(false);
  const [creativeSkillsOpen, setCreativeSkillsOpen] = useState(false);
  const [knowledgeGraphState, setKnowledgeGraphState] = useState<KnowledgeGraphState>({ nodes: [], edges: [], warnings: [] });
  const [knowledgeGraphError, setKnowledgeGraphError] = useState("");
  const [knowledgeHealthResult, setKnowledgeHealthResult] = useState<KnowledgeHealthWorkflowResponse | KnowledgeHealthFixResponse | null>(null);
  const [creativeSkillEnabled, setCreativeSkillEnabled] = useState<Record<CreativeSkillId, boolean>>(DEFAULT_CREATIVE_SKILL_STATE);
  const [creativeSkillSources, setCreativeSkillSources] = useState<CreativeSkillSources>(BUILTIN_CREATIVE_SKILL_SOURCES);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [workspaceError, setWorkspaceError] = useState("");
  const [prompt, setPrompt] = useState("");
  const [pendingEdits, setPendingEdits] = useState<DraftEdit[]>([]);
  const [promptPills, setPromptPills] = useState<PromptContextPill[]>([]);
  const promptPillsRef = useRef<PromptContextPill[]>([]);
  const [promptFileContentCache, setPromptFileContentCache] = useState<Record<string, string>>({});
  const [selectedKnowledgeCategoryId, setSelectedKnowledgeCategoryId] = useState("");
  const [activeModelLabel, setActiveModelLabel] = useState("");
  const [configuredModels, setConfiguredModels] = useState<ConfiguredModelStatus[]>([]);
  const [selectedModelId, setSelectedModelId] = useState("");
  const [projectState, setProjectState] = useState<ProjectState>({ projects: [] });
  const [projectError, setProjectError] = useState("");
  const [relevantNotes, setRelevantNotes] = useState<RelevantNote[]>([]);
  const [relevantNotesError, setRelevantNotesError] = useState("");
  const [relevantNotesLoading, setRelevantNotesLoading] = useState(false);
  const [selectionActionPosition, setSelectionActionPosition] = useState<SelectionActionPosition | null>(null);
  const [selectedPath, setSelectedPath] = useState("");
  const [filePreview, setFilePreview] = useState<FilePreviewState | null>(null);
  const [loadingPath, setLoadingPath] = useState("");
  const [editorTitle, setEditorTitle] = useState("");
  const [editorContent, setEditorContent] = useState("");
  const [lastSavedContent, setLastSavedContent] = useState("");
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState("");
  const [bridgeStatus, setBridgeStatus] = useState("");
  const [bridgeApplying, setBridgeApplying] = useState(false);
  const [memoryError, setMemoryError] = useState("");
  const [memoryTreeState, setMemoryTreeState] = useState<MemoryTreeState>({ roots: [] });
  const [savingMemoryTree, setSavingMemoryTree] = useState(false);
  const [fileMenu, setFileMenu] = useState<FileContextMenu | null>(null);
  const [libraryTab, setLibraryTab] = useState<"works" | "knowledge">("works");
  const [workspacePaneWidths, setWorkspacePaneWidths] = useState({
    left: DEFAULT_LEFT_PANE_WIDTH,
    right: DEFAULT_RIGHT_PANE_WIDTH,
  });
  const workspaceRef = useRef<HTMLDivElement | null>(null);
  const draftEditorRef = useRef<HTMLDivElement | null>(null);
  const fontSizeControlRef = useRef<HTMLDivElement | null>(null);
  const draftSelectionRef = useRef<TextSelection>({ start: editorContent.length, end: editorContent.length });
  const openFileRequestSeqRef = useRef(0);
  const relevantNotesRequestSeqRef = useRef(0);
  const updatePromptPills = useCallback((
    next: PromptContextPill[] | ((current: PromptContextPill[]) => PromptContextPill[]),
  ) => {
    const value = typeof next === "function" ? next(promptPillsRef.current) : next;
    promptPillsRef.current = value;
    setPromptPills(value);
  }, []);
  const appendDraftEdits = useCallback((edits: DraftEdit[]) => {
    setPendingEdits((current) => [...current, ...edits]);
  }, []);

  useEffect(() => {
    if (!fontSizeMenuOpen) {
      return;
    }

    const closeFontSizeMenuOnOutsidePointerDown = (event: PointerEvent) => {
      const control = fontSizeControlRef.current;
      if (!control || !(event.target instanceof Node) || !control.contains(event.target)) {
        setFontSizeMenuOpen(false);
      }
    };

    document.addEventListener("pointerdown", closeFontSizeMenuOnOutsidePointerDown, true);
    return () => {
      document.removeEventListener("pointerdown", closeFontSizeMenuOnOutsidePointerDown, true);
    };
  }, [fontSizeMenuOpen]);
  const refreshWorkspaceState = useCallback(async () => {
    const response = await initWorkspace();
    setWorkspace(response);
    setWorkspaceError("");
    return response;
  }, []);
  const chatManager = useChatManager({
    onDraftEdits: appendDraftEdits,
    onWorkspaceChanged: () => {
      void refreshWorkspaceState().catch((error) => {
        setWorkspaceError(error instanceof Error ? error.message : String(error));
      });
    },
  });
  const draftKind = useMemo(() => detectDraftKind(selectedPath, editorContent), [editorContent, selectedPath]);

  const loadMemoryTree = useCallback(async () => {
    try {
      const response = await invoke<MemoryTreeState>("wridian_get_memory_tree");
      setMemoryTreeState(response);
      setMemoryError("");
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const loadKnowledgeGraph = useCallback(async () => {
    try {
      const response = await invoke<KnowledgeGraphState>("wridian_get_knowledge_graph");
      setKnowledgeGraphState(response);
      setKnowledgeGraphError("");
    } catch (error) {
      setKnowledgeGraphError(error instanceof Error ? error.message : String(error));
    }
  }, []);

  const refreshKnowledgeSurfaces = useCallback(async () => {
    await Promise.all([
      loadKnowledgeGraph(),
      refreshWorkspaceState(),
    ]);
  }, [loadKnowledgeGraph, refreshWorkspaceState]);

  const loadModelAccounts = useCallback(async () => {
    try {
      const status = await invoke<ModelAccountsStatus>("wridian_get_model_accounts");
      const activeId = status.activeModelId ?? status.configuredModels[0]?.id ?? "";
      setConfiguredModels(status.configuredModels);
      setSelectedModelId(activeId);
      setActiveModelLabel(status.activeModelLabel ?? status.configuredModels.find((model) => model.id === activeId)?.label ?? "未配置模型");
    } catch (error) {
      setConfiguredModels([]);
      setSelectedModelId("");
      setActiveModelLabel("未配置模型");
      setWorkspaceError(`模型账户读取失败：${error instanceof Error ? error.message : String(error)}`);
    }
  }, []);

  const sendPrompt = async (override?: { contextPills?: PromptContextPill[]; text: string; selectedText?: string }) => {
    const snapshot = createSendPromptSnapshot({
      content: editorContent,
      contextPills: override?.contextPills ?? promptPillsRef.current,
      draftSelection: draftSelectionRef.current,
      overrideSelectedText: override?.selectedText,
      prompt,
      text: override?.text,
    });
    const userInput = snapshot.text || (snapshot.contextPills.length ? "请按已选择的技能执行。" : "");
    if (!userInput || chatManager.pending) return;
    if (!override) setPrompt("");
    setMemoryOpen(false);
    const sent = await chatManager.sendPrompt({
      content: snapshot.content,
      contextPills: snapshot.contextPills,
      draftKind,
      requestId: createCocreationRequestId(),
      selectedText: snapshot.selectedText,
      selectedModelId,
      sourcePath: selectedPath,
      text: userInput,
      title: editorTitle,
    });
    if (sent && !override) {
      updatePromptPills([]);
    }
  };

  const updateDraftSelection = useCallback(() => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection || selection.start === selection.end) {
      setSelectionActionPosition(null);
      return;
    }
    const browserSelection = window.getSelection();
    const range = browserSelection?.rangeCount ? browserSelection.getRangeAt(0) : null;
    const rect = range?.getBoundingClientRect();
    if (!rect || rect.width <= 0 || rect.height <= 0) {
      setSelectionActionPosition(null);
      return;
    }
    const { start, end } = selection;
    draftSelectionRef.current = { start, end };
    setSelectionActionPosition({
      left: Math.max(12, rect.left + rect.width / 2),
      top: Math.max(12, rect.top - 10),
    });
  }, []);

  const attachCurrentSelectionToPrompt = () => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection) return;
    const selected = editorContent.slice(selection.start, selection.end).trim();
    if (!selected) return;
    updatePromptPills((current) => upsertPromptContextPill(current, createSelectionPromptPill(selected, selection)));
    setSelectionActionPosition(null);
  };

  useEffect(() => {
    document.documentElement.classList.toggle("darkTheme", theme === "dark");
    try {
      window.localStorage.setItem(THEME_STORAGE_KEY, theme);
    } catch {
      // localStorage can be unavailable in restricted webviews; theme still applies for this run.
    }
  }, [theme]);

  useEffect(() => {
    try {
      window.localStorage.setItem(FONT_SIZE_STORAGE_KEY, fontSizeMode);
    } catch {
      // localStorage can be unavailable in restricted webviews; font size still applies for this run.
    }
  }, [fontSizeMode]);

  useEffect(() => {
    void initWorkspace()
      .then((response) => {
        setWorkspace(response);
        setWorkspaceError("");
      })
      .catch((error) => {
        setWorkspace(null);
        setWorkspaceError(error instanceof Error ? error.message : String(error));
      });
  }, []);

  useEffect(() => {
    void loadMemoryTree();
  }, [loadMemoryTree]);

  useEffect(() => {
    if (!memoryOpen) return;
    void loadMemoryTree();
  }, [loadMemoryTree, memoryOpen]);

  useEffect(() => {
    if (!knowledgeGraphOpen) return;
    void loadKnowledgeGraph();
  }, [knowledgeGraphOpen, loadKnowledgeGraph, workspace?.knowledgeFiles.length]);

  useEffect(() => {
    void loadModelAccounts();
  }, [loadModelAccounts]);

  useEffect(() => {
    void invoke<CreativeSkillSources>("wridian_get_creative_skill_sources")
      .then(setCreativeSkillSources)
      .catch(() => setCreativeSkillSources(BUILTIN_CREATIVE_SKILL_SOURCES));
  }, []);

  useEffect(() => {
    void getProjectState()
      .then((state) => {
        setProjectState(state);
        void chatManager.switchProjectChat(state.activeProjectId ?? "");
      })
      .catch((error) => setProjectError(error instanceof Error ? error.message : String(error)));
  }, [chatManager.switchProjectChat, workspace?.files.length, workspace?.filesRootPath]);

  useEffect(() => {
    const requestSeq = relevantNotesRequestSeqRef.current + 1;
    relevantNotesRequestSeqRef.current = requestSeq;
    const content = editorContent.trim();
    const selectedWorkNode = selectedPath ? findFileNodeByPath(workspace?.files ?? [], selectedPath) : undefined;
    if (!selectedPath || !selectedWorkNode || !content) {
      setRelevantNotes([]);
      setRelevantNotesError("");
      setRelevantNotesLoading(false);
      return;
    }
    setRelevantNotesLoading(true);
    const timer = window.setTimeout(() => {
      void invoke<RelevantNote[]>("wridian_find_relevant_notes", {
        input: {
          sourcePath: selectedPath,
          content,
          library: "works",
          limit: 8,
        },
      })
        .then((notes) => {
          if (relevantNotesRequestSeqRef.current !== requestSeq) return;
          setRelevantNotes(notes);
          setRelevantNotesError("");
        })
        .catch((error) => {
          if (relevantNotesRequestSeqRef.current !== requestSeq) return;
          setRelevantNotes([]);
          setRelevantNotesError(error instanceof Error ? error.message : String(error));
        })
        .finally(() => {
          if (relevantNotesRequestSeqRef.current === requestSeq) {
            setRelevantNotesLoading(false);
          }
        });
    }, 650);
    return () => {
      window.clearTimeout(timer);
    };
  }, [
    editorContent,
    projectState.activeProjectId,
    selectedPath,
    workspace?.files,
    workspace?.files.length,
    workspace?.filesRootPath,
    workspace?.knowledgeFiles.length,
    workspace?.knowledgeRootPath,
  ]);

  const files = workspace?.files ?? [];
  const knowledgeFiles = workspace?.knowledgeFiles ?? [];
  const visibleFiles = libraryTab === "works" ? files : knowledgeFiles;
  const selectedWorkNode = selectedPath ? findFileNodeByPath(files, selectedPath) : undefined;
  const selectedKnowledgeNode = selectedPath ? findFileNodeByPath(knowledgeFiles, selectedPath) : undefined;
  const selectedFileNode = selectedWorkNode ?? selectedKnowledgeNode;
  const selectedFileLibrary: WorkspaceLibrary | null = selectedWorkNode ? "works" : selectedKnowledgeNode ? "knowledge" : null;
  const selectedTreePath = selectedPath || filePreview?.path || "";
  const activeLibraryConfigured = libraryTab === "knowledge"
    ? Boolean(workspace?.knowledgeRootConfigured)
    : Boolean(workspace?.workRootConfigured);
  const isRealFile = Boolean(selectedPath);
  const hasEditorSurface = Boolean(selectedPath || filePreview);
  const dirty = isRealFile && !loadingPath && editorContent !== lastSavedContent;
  const bridgeCandidates = useMemo<BridgeCandidate[]>(() => {
    if (!selectedPath || !selectedFileLibrary || !selectedFileNode || !isMarkdownWorkspaceFile(selectedPath)) {
      return [];
    }
    const candidates: BridgeCandidate[] = [];
    if (selectedFileLibrary === "works") {
      const knowledgePill = promptPills.find((pill) => {
        if (!pill.sourcePath) return false;
        return Boolean(findFileNodeByPath(knowledgeFiles, pill.sourcePath));
      });
      if (!knowledgePill?.sourcePath) return candidates;
      const sourceNode = findFileNodeByPath(knowledgeFiles, knowledgePill.sourcePath);
      if (!sourceNode?.relativePath || !isMarkdownWorkspaceFile(sourceNode.path)) return candidates;
      const base = {
        sourceLibrary: "knowledge" as const,
        sourcePath: sourceNode.path,
        sourceRelativePath: sourceNode.relativePath,
        sourceTitle: knowledgePill.label || sourceNode.name,
        targetLibrary: "works" as const,
        targetPath: selectedPath,
      };
      candidates.push(
        { ...base, action: "referencesKnowledge", label: "引用知识" },
        { ...base, action: "adoptsKnowledge", label: "采纳为设定" },
        { ...base, action: "derivedFromKnowledge", label: "改写为规则" },
      );
    }
    if (selectedFileLibrary === "knowledge") {
      const worksPill = promptPills.find((pill) => {
        if (!pill.sourcePath) return false;
        return Boolean(findFileNodeByPath(files, pill.sourcePath));
      });
      if (worksPill?.sourcePath) {
        const sourceNode = findFileNodeByPath(files, worksPill.sourcePath);
        if (sourceNode?.relativePath && isMarkdownWorkspaceFile(sourceNode.path)) {
          const base = {
            sourceLibrary: "works" as const,
            sourcePath: sourceNode.path,
            sourceRelativePath: sourceNode.relativePath,
            sourceTitle: worksPill.label || sourceNode.name,
            targetLibrary: "knowledge" as const,
            targetPath: selectedPath,
          };
          candidates.push(
            { ...base, action: "abstractedFromDraft", label: "从作品抽象" },
            { ...base, action: "excerptedFromProject", label: "摘录到项目" },
          );
        }
      }
      const memoryPill = promptPills.find((pill) => {
        if (!pill.sourcePath) return false;
        return Boolean(findMemoryNodeByPath(memoryTreeState.roots, pill.sourcePath));
      });
      if (memoryPill?.sourcePath) {
        const sourceNode = findMemoryNodeByPath(memoryTreeState.roots, memoryPill.sourcePath);
        const sourceRelativePath = memoryRelativePath(memoryPill.sourcePath);
        if (sourceNode?.path && sourceRelativePath && isMarkdownWorkspaceFile(sourceNode.path)) {
          candidates.push({
            action: "distilledFromMemory",
            label: "从记忆蒸馏",
            sourceLibrary: "creative_memory",
            sourcePath: sourceNode.path,
            sourceRelativePath,
            sourceTitle: memoryPill.label || sourceNode.label,
            targetLibrary: "knowledge",
            targetPath: selectedPath,
          });
        }
      }
    }
    return candidates;
  }, [files, knowledgeFiles, memoryTreeState.roots, promptPills, selectedFileLibrary, selectedFileNode, selectedPath]);

  const saveCurrentFile = useCallback(async () => {
    if (!isRealFile || loadingPath || !dirty) return true;
    const pathToSave = selectedPath;
    const contentToSave = editorContent;
    setSaveStatus("saving");
    setSaveError("");
    try {
      await saveWorkFile(pathToSave, contentToSave);
      setPromptFileContentCache((current) => ({ ...current, [pathToSave]: contentToSave }));
      if (selectedPath === pathToSave && editorContent === contentToSave) {
        setLastSavedContent(contentToSave);
      }
      setSaveStatus("saved");
      return true;
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
      return false;
    }
  }, [dirty, editorContent, isRealFile, loadingPath, selectedPath]);

  const applyBridgeRelation = useCallback(async (candidate: BridgeCandidate) => {
    if (bridgeApplying) return;
    setBridgeApplying(true);
    setBridgeStatus("");
    const saved = await saveCurrentFile();
    if (!saved) {
      setBridgeStatus("当前文件保存失败，未写入桥接关系。");
      setBridgeApplying(false);
      return;
    }
    try {
      const response = await invoke<BridgeRelationResponse>("wridian_apply_bridge_relation", {
        input: {
          action: candidate.action,
          targetLibrary: candidate.targetLibrary,
          targetPath: candidate.targetPath,
          sourceLibrary: candidate.sourceLibrary,
          sourceRelativePath: candidate.sourceRelativePath,
          sourceTitle: candidate.sourceTitle,
        },
      });
      const refreshed = await openWorkFile(candidate.targetPath);
      setEditorContent(refreshed.content);
      setLastSavedContent(refreshed.content);
      setSaveStatus("saved");
      setBridgeStatus(response.message);
      if (knowledgeGraphOpen) {
        void loadKnowledgeGraph();
      }
      void refreshWorkspaceState();
    } catch (error) {
      setBridgeStatus(error instanceof Error ? error.message : String(error));
    } finally {
      setBridgeApplying(false);
    }
  }, [
    bridgeApplying,
    knowledgeGraphOpen,
    loadKnowledgeGraph,
    refreshWorkspaceState,
    saveCurrentFile,
  ]);

  useEffect(() => {
    if (!isRealFile || loadingPath) return;
    if (!dirty) {
      setSaveStatus("saved");
      return;
    }
    setSaveStatus("dirty");
    const timer = window.setTimeout(() => {
      void saveCurrentFile();
    }, 1000);
    return () => window.clearTimeout(timer);
  }, [dirty, isRealFile, loadingPath, saveCurrentFile]);

  const switchProject = async (id: string) => {
    try {
      const nextState = await selectProject(id || null);
      setProjectState(nextState);
      await chatManager.switchProjectChat(nextState.activeProjectId ?? "");
      setProjectError("");
    } catch (error) {
      setProjectError(error instanceof Error ? error.message : String(error));
    }
  };

  const switchModel = async (id: string) => {
    const localLabel = configuredModels.find((model) => model.id === id)?.label ?? "未配置模型";
    const previousSelectedModelId = selectedModelId;
    const previousActiveModelLabel = activeModelLabel;
    setSelectedModelId(id);
    setActiveModelLabel(localLabel);
    try {
      const status = await invoke<ModelAccountsStatus>("wridian_select_active_model", { input: { modelId: id } });
      const activeId = status.activeModelId ?? id;
      setConfiguredModels(status.configuredModels);
      setSelectedModelId(activeId);
      setActiveModelLabel(status.activeModelLabel ?? status.configuredModels.find((model) => model.id === activeId)?.label ?? localLabel);
    } catch (error) {
      setSelectedModelId(previousSelectedModelId);
      setActiveModelLabel(previousActiveModelLabel);
      chatManager.setError(error instanceof Error ? error.message : String(error));
    }
  };

  const refreshWorkspace = (response: WorkspaceInfo) => {
    setWorkspace(response);
    setWorkspaceError("");
  };

  const workspaceRootPath = libraryTab === "knowledge"
    ? activeLibraryConfigured ? workspace?.knowledgeRootPath || "" : ""
    : activeLibraryConfigured ? workspace?.filesRootPath || workspace?.activeWorkRoot || "" : "";
  const currentDirectoryLabel = workspaceRootPath ? baseName(workspaceRootPath) : "";

  const runWorkspaceAction = async (action: () => Promise<WorkspaceInfo>) => {
    setWorkspaceError("");
    try {
      refreshWorkspace(await action());
    } catch (error) {
      setWorkspaceError(error instanceof Error ? error.message : String(error));
    }
  };

  const chooseLibraryRoot = async (tab = libraryTab) => {
    setWorkspaceError("");
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({
        directory: true,
        multiple: false,
        title: tab === "knowledge" ? "选择知识库文件夹" : "选择作品库文件夹",
      });
      if (!selected || Array.isArray(selected)) return;
      refreshWorkspace(await setLibraryRoot(selected, tab));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspaceError(message.includes("not allowed") || message.includes("Tauri") ? "请在 Wridian 桌面端选择本地文件夹。" : message);
    }
  };

  const createFile = async (parentPath = workspaceRootPath) => {
    if (!parentPath) return;
    const name = window.prompt("新建文件", "未命名.md");
    if (!name) return;
    await runWorkspaceAction(() => createWorkFile(parentPath, name));
  };

  const createFolder = async (parentPath = workspaceRootPath) => {
    if (!parentPath) return;
    const name = window.prompt("新建文件夹", "新建文件夹");
    if (!name) return;
    await runWorkspaceAction(() => createWorkFolder(parentPath, name));
  };

  const duplicateNode = async (node: WorkFileNode) => {
    await runWorkspaceAction(() => duplicateWorkNode(node.path));
  };

  const renameNode = async (node: WorkFileNode) => {
    const name = window.prompt("重命名", node.name);
    if (!name || name === node.name) return;
    await runWorkspaceAction(() => renameWorkNode(node.path, name));
  };

  const trashNode = async (node: WorkFileNode) => {
    await runWorkspaceAction(() => trashWorkNode(node.path));
  };

  const addNodeToPrompt = (node: WorkFileNode) => {
    if (node.folder) return;
    void addFileToPrompt(node.name, node.path, node.relativePath);
  };

  const addFileToPrompt = async (name: string, path: string, relativePath = "") => {
    if (!isTextContextFile(path)) {
      updatePromptPills((current) => upsertPromptContextPill(current, createFilePromptPill(name, path, relativePath)));
      return;
    }
    try {
      const cached = promptFileContentCache[path];
      const content = cached ?? (isEditableWorkspaceFile(path)
        ? (await openWorkFile(path)).content
        : (await previewWorkFile(path)).content ?? "");
      setPromptFileContentCache((current) => ({ ...current, [path]: content }));
      updatePromptPills((current) => upsertPromptContextPill(
        current,
        relativePath
          ? createReferencedFileContentPromptPill(name, path, relativePath, content)
          : createFileContentPromptPill(name, path, content),
      ));
    } catch (error) {
      updatePromptPills((current) => upsertPromptContextPill(current, createFilePromptPill(name, path, relativePath)));
      chatManager.setError(`文件内容读取失败，已改为文件路径引用：${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const openFileContextMenu = (node: WorkFileNode, x: number, y: number) => {
    setFileMenu({ node, x, y });
  };

  useEffect(() => {
    if (!fileMenu) return;
    const close = () => setFileMenu(null);
    window.addEventListener("click", close);
    window.addEventListener("keydown", close);
    return () => {
      window.removeEventListener("click", close);
      window.removeEventListener("keydown", close);
    };
  }, [fileMenu]);

  const openFilePath = async (
    requestedPath: string,
    fallbackName = "",
    options: { refreshBeforeOpen?: boolean; targetLibrary?: WorkspaceLibrary } = {},
  ) => {
    const requestSeq = openFileRequestSeqRef.current + 1;
    openFileRequestSeqRef.current = requestSeq;
    const saved = await saveCurrentFile();
    if (!saved || openFileRequestSeqRef.current !== requestSeq) return;

    if (options.targetLibrary) {
      setLibraryTab(options.targetLibrary);
    }
    if (options.refreshBeforeOpen) {
      try {
        await refreshWorkspaceState();
      } catch (error) {
        if (openFileRequestSeqRef.current !== requestSeq) return;
        setWorkspaceError(error instanceof Error ? error.message : String(error));
        return;
      }
    }
    if (openFileRequestSeqRef.current !== requestSeq) return;

    setLoadingPath(requestedPath);
    setSaveError("");
    setSaveStatus("idle");
    if (!isEditableWorkspaceFile(requestedPath)) {
      try {
        const response = await previewWorkFile(requestedPath);
        if (openFileRequestSeqRef.current !== requestSeq) return;
        const name = response.name || fallbackName || requestedPath.split(/[\\/]/).pop() || "未命名文件";
        const previewType = response.previewType === "image" || response.previewType === "pdf" || response.previewType === "text"
          ? response.previewType
          : filePreviewType(response.path);
        let assetUrl = "";
        let previewError = "";
        if (previewType === "image" || previewType === "pdf") {
          try {
            const asset = await previewWorkAsset(response.path);
            assetUrl = asset.url;
          } catch (error) {
            previewError = error instanceof Error ? error.message : String(error);
          }
        }
        if (openFileRequestSeqRef.current !== requestSeq) return;
        setSelectedPath("");
        setFilePreview({
          assetUrl,
          content: response.content ?? "",
          extension: fileExtension(response.path).toUpperCase(),
          name,
          path: response.path,
          previewError,
          type: previewType,
        });
        setEditorTitle(name);
        setEditorContent("");
        setLastSavedContent("");
        updatePromptPills([]);
        setPendingEdits([]);
        setSelectionActionPosition(null);
        setSaveStatus("saved");
      } catch (error) {
        if (openFileRequestSeqRef.current !== requestSeq) return;
        setSaveStatus("error");
        setSaveError(error instanceof Error ? error.message : String(error));
      } finally {
        if (openFileRequestSeqRef.current === requestSeq) {
          setLoadingPath((current) => (current === requestedPath ? "" : current));
        }
      }
      return;
    }
    setFilePreview(null);
    try {
      const response = await openWorkFile(requestedPath);
      if (openFileRequestSeqRef.current !== requestSeq) return;
      setSelectedPath(response.path);
      setEditorTitle(response.name);
      setEditorContent(response.content);
      setLastSavedContent(response.content);
      setPromptFileContentCache((current) => ({ ...current, [response.path]: response.content }));
      draftSelectionRef.current = { start: response.content.length, end: response.content.length };
      setSelectionActionPosition(null);
      updatePromptPills([]);
      setPendingEdits([]);
      setSaveStatus("saved");
      const project = projectState.projects.find((item) => response.path.startsWith(item.id));
      if (project && project.id !== projectState.activeProjectId) {
        void switchProject(project.id);
      } else if (!project && projectState.activeProjectId) {
        void switchProject("");
      }
    } catch (error) {
      if (openFileRequestSeqRef.current !== requestSeq) return;
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      if (openFileRequestSeqRef.current === requestSeq) {
        setLoadingPath((current) => (current === requestedPath ? "" : current));
      }
    }
  };

  const openFile = async (node: WorkFileNode) => {
    if (node.folder) return;
    await openFilePath(node.path, node.name, { targetLibrary: node.library });
  };

  const openKnowledgeGraphFile = (path: string) => {
    setKnowledgeGraphOpen(false);
    void openFilePath(path, "", { refreshBeforeOpen: true, targetLibrary: "knowledge" });
  };

  const addRelevantNoteToPrompt = (note: RelevantNote) => {
    void addFileToPrompt(note.title, note.path, note.relativePath ?? "");
  };

  const openRelevantNote = (note: RelevantNote) => {
    void openFilePath(note.path, note.title, { targetLibrary: "works" });
  };

  const handleDraftKeyDown = (event: ReactKeyboardEvent<HTMLElement>) => {
    if ((event.metaKey || event.ctrlKey) && event.key.toLowerCase() === "s") {
      event.preventDefault();
      void saveCurrentFile();
    }
  };

  const applyTextToDraft = useCallback((text: string, selection: TextSelection) => {
    const start = Math.max(0, Math.min(selection.start, editorContent.length));
    const end = Math.max(start, Math.min(selection.end, editorContent.length));
    const nextContent = `${editorContent.slice(0, start)}${text}${editorContent.slice(end)}`;
    const nextCursor = start + text.length;
    setEditorContent(nextContent);
    draftSelectionRef.current = { start: nextCursor, end: nextCursor };
    setSelectionActionPosition(null);
    window.requestAnimationFrame(() => {
      setContentEditableCaret(draftEditorRef.current, nextCursor);
    });
  }, [editorContent]);

  const copyText = async (text: string) => {
    const reply = text.trim();
    if (!reply) return;
    try {
      await navigator.clipboard.writeText(reply);
    } catch (error) {
      setSaveError(`复制失败：${error instanceof Error ? error.message : String(error)}`);
    }
  };

  const updateChatMessageText = (message: ChatMessage, text: string) => {
    const selection = draftSelectionRef.current;
    const selectedText = editorContent.slice(selection.start, selection.end).trim();
    const updated = chatManager.updateMessageText(message.id, text, {
      content: editorContent,
      selectedText,
      sourcePath: selectedPath,
      title: editorTitle,
    });
    if (!updated) {
      chatManager.setError("消息内容不能为空。");
    } else {
      chatManager.setError("");
    }
  };

  const retryLastUserMessage = (message: ChatMessage) => {
    const contextPills = restorePromptPillsFromMessage(message);
    updatePromptPills(contextPills);
    void sendPrompt({ contextPills, text: message.text, selectedText: message.selectedText });
  };

  const acceptEdit = (id: string) => {
    const edit = pendingEdits.find((item) => item.id === id && item.status === "pending");
    if (!edit) return;
    const guardReport = createDraftReplaceGuardReport(editorContent, pendingEdits.filter((item) => item.status === "pending"));
    const match = guardReport.matches.find((item) => item.edit.id === id);
    if (!match) {
      const skipped = guardReport.skipped.find((item) => item.edit.id === id);
      chatManager.setError(skipped ? describeDraftReplaceSkip(skipped.reason) : "这处修改无法安全定位。");
      return;
    }
    chatManager.setError("");
    applyTextToDraft(edit.replacement, { start: match.index, end: match.index + edit.target.length });
    setPendingEdits((edits) => edits.map((item) => (item.id === id ? { ...item, status: "accepted" } : item)));
  };

  const rejectEdit = (id: string) => {
    setPendingEdits((edits) => edits.map((item) => (item.id === id ? { ...item, status: "rejected" } : item)));
  };

  const acceptAllEdits = () => {
    const pending = pendingEdits.filter((edit) => edit.status === "pending");
    const guardReport = createDraftReplaceGuardReport(editorContent, pending);
    const matches = guardReport.matches;

    if (!matches.length) {
      chatManager.setError("没有可以安全确认的修改。");
      return;
    }

    const appliedIds = new Set(matches.map((match) => match.edit.id));
    const nextContent = [...matches].sort((left, right) => right.index - left.index).reduce((content, match) => {
      const start = match.index;
      const end = start + match.edit.target.length;
      return `${content.slice(0, start)}${match.edit.replacement}${content.slice(end)}`;
    }, editorContent);

    setEditorContent(nextContent);
    setPendingEdits((edits) => edits.map((edit) => (appliedIds.has(edit.id) ? { ...edit, status: "accepted" } : edit)));
    chatManager.setError(guardReport.skipped.length ? `${guardReport.skipped.length} 处修改需要重新定位。` : "");
    draftSelectionRef.current = { start: 0, end: 0 };
    setSelectionActionPosition(null);

  };

  const rejectAllEdits = () => {
    setPendingEdits((edits) => edits.map((edit) => (edit.status === "pending" ? { ...edit, status: "rejected" } : edit)));
  };

  const pendingDraftEdits = useMemo(() => pendingEdits.filter((edit) => edit.status === "pending"), [pendingEdits]);
  const draftReplaceGuardReport = useMemo(
    () => createDraftReplaceGuardReport(editorContent, pendingDraftEdits),
    [editorContent, pendingDraftEdits],
  );
  const blockedDraftEditCount = draftReplaceGuardReport.skipped.length;
  const knowledgeSuggestionIndex = useMemo(() => buildKnowledgeSuggestionIndex(knowledgeFiles), [knowledgeFiles]);
  const enabledCreativeSkills = useMemo(
    () => CREATIVE_SKILLS.filter((skill) => creativeSkillEnabled[skill.id]),
    [creativeSkillEnabled],
  );
  const promptSuggestions = useMemo(() => buildPromptSuggestions({
    creativeSkills: enabledCreativeSkills,
    creativeSkillSources,
    draftKind,
    knowledgeCards: knowledgeSuggestionIndex.cards,
    knowledgeCategories: knowledgeSuggestionIndex.categories,
    selectedKnowledgeCategoryId,
  }), [
    creativeSkillSources,
    draftKind,
    enabledCreativeSkills,
    knowledgeSuggestionIndex,
    selectedKnowledgeCategoryId,
  ]);

  const statusLabel = useMemo(() => {
    if (saveStatus === "idle") return "读取中";
    if (saveStatus === "dirty") return "未保存";
    if (saveStatus === "saving") return "正在保存";
    if (saveStatus === "error") return "保存失败";
    return "已保存";
  }, [saveStatus]);

  const saveMemoryTreeFile = async (path: string, content: string): Promise<boolean> => {
    setSavingMemoryTree(true);
    try {
      const response = await invoke<MemoryTreeState>("wridian_save_memory_tree_file", {
        input: { path, content },
      });
      setMemoryTreeState(response);
      setMemoryError("");
      return true;
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
      return false;
    } finally {
      setSavingMemoryTree(false);
    }
  };

  const deleteMemoryTreeFile = async (path: string): Promise<boolean> => {
    setSavingMemoryTree(true);
    try {
      const response = await invoke<MemoryTreeState>("wridian_delete_memory_tree_file", {
        input: { path },
      });
      setMemoryTreeState(response);
      setMemoryError("");
      return true;
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
      return false;
    } finally {
      setSavingMemoryTree(false);
    }
  };

  const resizeWorkspacePane = (side: "left" | "right", event: ReactPointerEvent<HTMLDivElement>) => {
    const workspaceNode = workspaceRef.current;
    if (!workspaceNode) return;
    event.preventDefault();

    const startX = event.clientX;
    const startWidths = workspacePaneWidths;
    const workspaceWidth = workspaceNode.getBoundingClientRect().width;
    const resizerSpace = WORKSPACE_RESIZER_WIDTH * WORKSPACE_RESIZER_COUNT;

    const onPointerMove = (moveEvent: PointerEvent) => {
      const deltaX = moveEvent.clientX - startX;
      setWorkspacePaneWidths(() => {
        if (side === "left") {
          const maxLeft = Math.min(
            MAX_LEFT_PANE_WIDTH,
            workspaceWidth - startWidths.right - MIN_WRITING_PANE_WIDTH - resizerSpace,
          );
          return {
            left: clamp(startWidths.left + deltaX, MIN_LEFT_PANE_WIDTH, maxLeft),
            right: startWidths.right,
          };
        }

        const maxRight = Math.min(
          MAX_RIGHT_PANE_WIDTH,
          workspaceWidth - startWidths.left - MIN_WRITING_PANE_WIDTH - resizerSpace,
        );
        return {
          left: startWidths.left,
          right: clamp(startWidths.right - deltaX, MIN_RIGHT_PANE_WIDTH, maxRight),
        };
      });
    };

    const onPointerUp = () => {
      window.removeEventListener("pointermove", onPointerMove);
      window.removeEventListener("pointerup", onPointerUp);
    };

    window.addEventListener("pointermove", onPointerMove);
    window.addEventListener("pointerup", onPointerUp, { once: true });
  };

  const openMemoryFolder = async () => {
    if (!workspace?.runtimePath) {
      setMemoryError("请在 Wridian 桌面端打开记忆文件夹。");
      return;
    }
    try {
      await invoke("wridian_open_local_path", { input: { path: `${workspace.runtimePath}\\memory-tree` } });
      setMemoryError("");
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
    }
  };

  return (
    <div
      className="app-shell"
      style={{ "--ui-font-scale": FONT_SIZE_SCALE[fontSizeMode] } as React.CSSProperties}
    >
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" />
          <span>Wridian</span>
        </div>
        <nav className="top-actions" aria-label="Wridian actions">
          <button type="button" title="创作记忆树" aria-label="创作记忆树" onClick={() => {
            setMemoryOpen(true);
          }}>
            <MemoryTreeIcon />
          </button>
          <button type="button" title="知识图谱" aria-label="知识图谱" onClick={() => setKnowledgeGraphOpen(true)}>
            <KnowledgeGraphIcon />
          </button>
          <button type="button" title="技能管理" aria-label="技能管理" onClick={() => setCreativeSkillsOpen(true)}>
            <LightningIcon />
          </button>
          <button type="button" title="模型配置" aria-label="模型配置" onClick={() => setSettingsOpen(true)}>
            <ModelConfigIcon />
          </button>
          <div className="font-size-control" ref={fontSizeControlRef}>
            <button
              type="button"
              title="字体大小"
              aria-label="字体大小"
              aria-expanded={fontSizeMenuOpen}
              onClick={() => setFontSizeMenuOpen((open) => !open)}
            >
              <FontSizeIcon />
            </button>
            {fontSizeMenuOpen ? (
              <div className="font-size-popover" role="menu" aria-label="字体大小">
                <button type="button" className={fontSizeMode === "default" ? "active" : ""} onClick={() => {
                  setFontSizeMode("default");
                  setFontSizeMenuOpen(false);
                }}>
                  默认
                </button>
                <button type="button" className={fontSizeMode === "large" ? "active" : ""} onClick={() => {
                  setFontSizeMode("large");
                  setFontSizeMenuOpen(false);
                }}>
                  较大
                </button>
                <button type="button" className={fontSizeMode === "max" ? "active" : ""} onClick={() => {
                  setFontSizeMode("max");
                  setFontSizeMenuOpen(false);
                }}>
                  最大
                </button>
              </div>
            ) : null}
          </div>
          <button
            type="button"
            title={theme === "light" ? "深色主题" : "浅色主题"}
            aria-label={theme === "light" ? "深色主题" : "浅色主题"}
            onClick={() => setTheme(theme === "light" ? "dark" : "light")}
          >
            {theme === "light" ? <DarkThemeIcon /> : <LightThemeIcon />}
          </button>
        </nav>
      </header>

      <div
        className="workspace"
        ref={workspaceRef}
        style={{
          "--left-pane-width": `${workspacePaneWidths.left}px`,
          "--right-pane-width": `${workspacePaneWidths.right}px`,
        } as React.CSSProperties}
      >
        <aside className="project-rail" aria-label="作品">
          <div className="rail-topline">
            <div className="library-tabs" role="tablist" aria-label="资料库">
              <button type="button" className={libraryTab === "works" ? "active" : ""} onClick={() => setLibraryTab("works")}>
                作品库
              </button>
              <button type="button" className={libraryTab === "knowledge" ? "active" : ""} onClick={() => setLibraryTab("knowledge")}>
                知识库
              </button>
            </div>
            <div className="file-toolbar" aria-label="文件操作">
              <button type="button" title="新建文件" aria-label="新建文件" onClick={() => void createFile()} disabled={!activeLibraryConfigured}>
                <PencilIcon />
              </button>
              <button type="button" title="新建文件夹" aria-label="新建文件夹" onClick={() => void createFolder()} disabled={!activeLibraryConfigured}>
                <FolderPlusIcon />
              </button>
              <button
                type="button"
                title={libraryFolderTooltip(libraryTab)}
                aria-label={libraryFolderTooltip(libraryTab)}
                onClick={() => void chooseLibraryRoot(libraryTab)}
              >
                <WorkFolderIcon />
              </button>
            </div>
          </div>
          {workspaceError ? <div className="rail-error">{workspaceError}</div> : null}
          <div className="file-tree">
            {visibleFiles.length ? (
              visibleFiles.map((node) => (
                <FileNodeView
                  key={node.path}
                  node={node}
                  depth={0}
                  selectedPath={selectedTreePath}
                  onOpenFile={openFile}
                  onOpenMenu={openFileContextMenu}
                />
              ))
            ) : null}
          </div>

          <div className="rail-bottom">
            {currentDirectoryLabel ? (
              <div className="rail-current-directory" title={workspaceRootPath}>
                当前目录：{currentDirectoryLabel}
              </div>
            ) : null}
            <button type="button" title="系统设置" aria-label="系统设置" onClick={() => setSettingsOpen(true)}>
              <SettingsIcon />
            </button>
          </div>
        </aside>

        <div
          className="workspace-resizer"
          role="separator"
          aria-label="调整作品区宽度"
          aria-orientation="vertical"
          tabIndex={0}
          onPointerDown={(event) => resizeWorkspacePane("left", event)}
        />

        <main className="writing-pane">
          <section className={`paper ${filePreview ? "paper-preview" : hasEditorSurface ? "" : "paper-empty"}`} aria-label="正文编辑区">
            {selectedPath ? (
              <>
                <div className="chapter-heading-row">
                  <h1 className="chapter-heading">{editorTitle}</h1>
                  <div className="chapter-heading-actions">
                    {bridgeCandidates.length ? (
                      <div className="bridge-actions" aria-label="作品知识桥接">
                        {bridgeCandidates.map((candidate) => (
                          <button
                            key={candidate.action}
                            type="button"
                            title={`${candidate.label}：${candidate.sourceTitle}`}
                            disabled={bridgeApplying}
                            onClick={() => void applyBridgeRelation(candidate)}
                          >
                            {candidate.label}
                          </button>
                        ))}
                      </div>
                    ) : null}
                    <div className={`save-state ${saveStatus}`} title={saveError || bridgeStatus || undefined}>
                      {bridgeStatus || statusLabel}
                    </div>
                  </div>
                </div>
                <div className="draft-suggestion-actions" aria-label="待确认修改操作" hidden={!pendingDraftEdits.length}>
                    <span>{pendingDraftEdits.length} 处待确认修改</span>
                    {blockedDraftEditCount ? <span className="draft-guard-note">{blockedDraftEditCount} 处需重新定位</span> : null}
                    <button type="button" onClick={acceptAllEdits}>全部确认</button>
                    <button type="button" className="secondary" onClick={rejectAllEdits}>全部取消</button>
                </div>
                <DraftEditor
                  content={editorContent}
                  edits={pendingDraftEdits}
                  editorRef={draftEditorRef}
                  onAcceptEdit={acceptEdit}
                  onChange={setEditorContent}
                  onKeyDown={handleDraftKeyDown}
                  onRejectEdit={rejectEdit}
                  onSelectionActionDismiss={() => setSelectionActionPosition(null)}
                  onSelectionChange={updateDraftSelection}
                />
                {selectionActionPosition ? (
                  <button
                    type="button"
                    className="selection-chat-action"
                    style={{
                      left: `${selectionActionPosition.left}px`,
                      top: `${selectionActionPosition.top}px`,
                    }}
                    onMouseDown={(event) => event.preventDefault()}
                    onClick={attachCurrentSelectionToPrompt}
                  >
                    添加到对话
                  </button>
                ) : null}
              </>
            ) : filePreview ? (
              <FilePreviewViewer file={filePreview} />
            ) : (
              <div className="empty-editor" aria-label="空文件编辑区">
                <div className="empty-brand" aria-label="Wridian">
                  Wridian
                </div>
                <div className="empty-slogan">让故事有记忆，让知识可调用</div>
                <div className="empty-actions" aria-label="开始使用">
                  <button type="button" className="empty-action primary" onClick={() => void chooseLibraryRoot("works")}>
                    选择作品库
                  </button>
                  <button type="button" className="empty-action" onClick={() => setMemoryOpen(true)}>
                    查看记忆树
                  </button>
                </div>
              </div>
            )}
            {saveError ? <div className="paper-error">{saveError}</div> : null}
          </section>
        </main>

        <div
          className="workspace-resizer"
          role="separator"
          aria-label="调整对话区宽度"
          aria-orientation="vertical"
          tabIndex={0}
          onPointerDown={(event) => resizeWorkspacePane("right", event)}
        />

        <ChatPanel
          configuredModels={configuredModels}
          error={chatManager.error}
          messages={chatManager.messages}
          onCopy={copyText}
          onUpdateMessageText={updateChatMessageText}
          onRetry={retryLastUserMessage}
          pending={chatManager.pending}
          prompt={prompt}
          promptPills={promptPills}
          promptSuggestions={promptSuggestions}
          relevantNotes={relevantNotes}
          relevantNotesError={relevantNotesError}
          relevantNotesLoading={relevantNotesLoading}
          activeModelLabel={activeModelLabel}
          selectedModelId={selectedModelId}
          projectError={projectError}
          projects={projectState.projects}
          selectedProjectId={projectState.activeProjectId ?? ""}
          onSelectModel={(id) => void switchModel(id)}
          onSelectProject={(id) => void switchProject(id)}
          onAddRelevantNote={addRelevantNoteToPrompt}
          onOpenRelevantNote={openRelevantNote}
          onStop={chatManager.stopPrompt}
          onPromptChange={setPrompt}
          onPromptPillsChange={updatePromptPills}
          onImagePaste={(files) => {
            updatePromptPills((current) => files.reduce(
              (next, file) => upsertPromptContextPill(next, createImagePromptPill(file.name || "pasted-image", file.size)),
              current,
            ));
          }}
          onRemovePill={(id) => updatePromptPills((current) => current.filter((pill) => pill.id !== id))}
          onSelectSuggestion={(suggestion) => {
            if (suggestion.kind === "command") {
              if (suggestion.pillKind === "tool") {
                updatePromptPills((current) => upsertPromptContextPill(current, createPromptPillFromSuggestion(suggestion)));
              }
              return;
            }
            if (suggestion.kind !== "context") return;
            if (suggestion.id.startsWith("knowledge-category:")) {
              const categoryId = suggestion.insertText.slice("category:".length);
              setSelectedKnowledgeCategoryId(categoryId);
              setPrompt("@");
              return;
            }
            if (suggestion.pillKind === "memory" && suggestion.insertText.startsWith("path:")) {
              const path = suggestion.insertText.slice("path:".length);
              void addFileToPrompt(suggestion.label, path, suggestion.relativePath ?? "");
              setSelectedKnowledgeCategoryId("");
              return;
            }
            updatePromptPills((current) => upsertPromptContextPill(current, createPromptPillFromSuggestion(suggestion)));
          }}
          onSubmit={() => void sendPrompt()}
        />
      </div>

      {memoryOpen ? (
        <MemoryDrawer
          memoryError={memoryError}
          memoryTree={memoryTreeState}
          onClose={() => setMemoryOpen(false)}
          onDeleteFile={deleteMemoryTreeFile}
          onOpenMemoryFolder={openMemoryFolder}
          onSaveFile={saveMemoryTreeFile}
          projects={projectState.projects}
          saving={savingMemoryTree}
          selectedProjectId={projectState.activeProjectId}
        />
      ) : null}
      {knowledgeGraphOpen ? (
        <KnowledgeGraphDrawer
          graph={knowledgeGraphState}
          graphError={knowledgeGraphError}
          healthResult={knowledgeHealthResult}
          knowledgeRootConfigured={Boolean(workspace?.knowledgeRootConfigured)}
          onClose={() => setKnowledgeGraphOpen(false)}
          onHealthResult={setKnowledgeHealthResult}
          onOpenFile={openKnowledgeGraphFile}
          onRefresh={refreshKnowledgeSurfaces}
        />
      ) : null}
      {creativeSkillsOpen ? (
        <CreativeSkillsDrawer
          enabled={creativeSkillEnabled}
          onClose={() => setCreativeSkillsOpen(false)}
          onToggle={(id) => {
            setCreativeSkillEnabled((current) => ({ ...current, [id]: !current[id] }));
          }}
          skills={CREATIVE_SKILLS}
        />
      ) : null}
      {settingsOpen ? <ModelSettingsDialog onClose={() => setSettingsOpen(false)} onChanged={loadModelAccounts} /> : null}
      {fileMenu ? (
        <FileContextMenuView
          menu={fileMenu}
          onAddToPrompt={addNodeToPrompt}
          onClose={() => setFileMenu(null)}
          onCreateFile={createFile}
          onCreateFolder={createFolder}
          onDuplicate={duplicateNode}
          onRename={renameNode}
          onTrash={trashNode}
        />
      ) : null}
    </div>
  );
}

export default App;
