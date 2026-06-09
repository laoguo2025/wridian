import { type CSSProperties, KeyboardEvent as ReactKeyboardEvent, PointerEvent as ReactPointerEvent, WheelEvent as ReactWheelEvent, useCallback, useEffect, useMemo, useRef, useState } from "react";
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
  type DraftKind,
  type PromptContextPill,
} from "./chat/promptContext";
import {
  createDraftReplaceGuardReport,
  describeDraftReplaceSkip,
} from "./editor/draftReplaceGuard";
import { libraryFolderPath, libraryFolderTooltip } from "./libraryToolbar";
import {
  CREATIVE_SKILLS,
  DEFAULT_CREATIVE_SKILL_STATE,
  type CreativeSkill,
  type CreativeSkillId,
} from "./creativeSkills";
import memoryTreeBase from "./assets/memory-tree-base.png";
import "./App.css";

type Theme = "light" | "dark";
type FontSizeMode = "default" | "large" | "max";
type SaveStatus = "idle" | "dirty" | "saving" | "saved" | "error";

type WorkspaceInfo = {
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

type WorkFileNode = {
  name: string;
  path: string;
  relativePath: string;
  library: "works" | "knowledge";
  folder: boolean;
  children: WorkFileNode[];
};

type OpenFileResponse = {
  path: string;
  name: string;
  content: string;
};

type SaveFileResponse = {
  ok: boolean;
  savedAt: string;
};

type CustomApiSettingsStatus = {
  configured: boolean;
  baseUrl?: string | null;
  model?: string | null;
  maskedKey?: string | null;
};

type TestCustomApiResponse = {
  ok: boolean;
  message: string;
};

type DraftEdit = ChatDraftEdit;

type MemoryTreeNode = {
  id: string;
  kind: string;
  label: string;
  description: string;
  path?: string | null;
  content?: string | null;
  children: MemoryTreeNode[];
};

type MemoryTreeState = {
  roots: MemoryTreeNode[];
};

type MemoryLeafCandidate = {
  id: string;
  branch: string;
  title: string;
  summary: string;
  reason: string;
  status: string;
  sourcePath: string;
  targetPath: string;
};

type KnowledgeGraphNode = {
  id: string;
  label: string;
  kind: "folder" | "card" | string;
  path?: string | null;
  group: string;
  size: number;
};

type KnowledgeGraphEdge = {
  source: string;
  target: string;
  kind: string;
};

type KnowledgeGraphState = {
  nodes: KnowledgeGraphNode[];
  edges: KnowledgeGraphEdge[];
};

type KnowledgeCategory = {
  detail: string;
  id: string;
  title: string;
};

type KnowledgeCardSuggestion = {
  category: string;
  categoryId: string;
  id: string;
  relativePath: string;
  sourcePath: string;
  title: string;
};

type FileContextMenu = {
  node: WorkFileNode;
  x: number;
  y: number;
};

type TextSelection = {
  start: number;
  end: number;
};

const DEFAULT_LEFT_PANE_WIDTH = 218;
const DEFAULT_RIGHT_PANE_WIDTH = 292;
const MIN_LEFT_PANE_WIDTH = 168;
const MAX_LEFT_PANE_WIDTH = 360;
const MIN_RIGHT_PANE_WIDTH = 240;
const MAX_RIGHT_PANE_WIDTH = 420;
const MIN_WRITING_PANE_WIDTH = 360;
const WORKSPACE_RESIZER_WIDTH = 12;
const WORKSPACE_RESIZER_COUNT = 2;
const FONT_SIZE_SCALE: Record<FontSizeMode, number> = {
  default: 1,
  large: 1.12,
  max: 1.25,
};
function App() {
  const [theme, setTheme] = useState<Theme>("light");
  const [fontSizeMode, setFontSizeMode] = useState<FontSizeMode>("default");
  const [fontSizeMenuOpen, setFontSizeMenuOpen] = useState(false);
  const [memoryOpen, setMemoryOpen] = useState(false);
  const [knowledgeGraphOpen, setKnowledgeGraphOpen] = useState(false);
  const [creativeSkillsOpen, setCreativeSkillsOpen] = useState(false);
  const [knowledgeGraphState, setKnowledgeGraphState] = useState<KnowledgeGraphState>({ nodes: [], edges: [] });
  const [knowledgeGraphError, setKnowledgeGraphError] = useState("");
  const [creativeSkillEnabled, setCreativeSkillEnabled] = useState<Record<CreativeSkillId, boolean>>(DEFAULT_CREATIVE_SKILL_STATE);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [workspaceError, setWorkspaceError] = useState("");
  const [prompt, setPrompt] = useState("");
  const [pendingEdits, setPendingEdits] = useState<DraftEdit[]>([]);
  const [promptPills, setPromptPills] = useState<PromptContextPill[]>([]);
  const [promptFileContentCache, setPromptFileContentCache] = useState<Record<string, string>>({});
  const [selectedKnowledgeCategoryId, setSelectedKnowledgeCategoryId] = useState("");
  const [activeModelLabel, setActiveModelLabel] = useState("");
  const [projectState, setProjectState] = useState<ProjectState>({ projects: [] });
  const [projectError, setProjectError] = useState("");
  const [hasDraftSelection, setHasDraftSelection] = useState(false);
  const [selectedPath, setSelectedPath] = useState("");
  const [loadingPath, setLoadingPath] = useState("");
  const [editorTitle, setEditorTitle] = useState("");
  const [editorContent, setEditorContent] = useState("");
  const [lastSavedContent, setLastSavedContent] = useState("");
  const [saveStatus, setSaveStatus] = useState<SaveStatus>("idle");
  const [saveError, setSaveError] = useState("");
  const [memoryError, setMemoryError] = useState("");
  const [memoryTreeState, setMemoryTreeState] = useState<MemoryTreeState>({ roots: [] });
  const [memoryLeafCandidate, setMemoryLeafCandidate] = useState<MemoryLeafCandidate | null>(null);
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
  const chatManager = useChatManager({ onDraftEdits: appendDraftEdits });
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

  const sendPrompt = async (override?: { text: string; selectedText?: string }) => {
    const userInput = (override?.text ?? prompt).trim();
    if (!userInput || chatManager.pending) return;
    if (!override) setPrompt("");
    setMemoryOpen(false);
    const sent = await chatManager.sendPrompt({
      content: editorContent,
      contextPills: override ? [] : promptPills,
      draftKind,
      selectedText: override?.selectedText,
      sourcePath: selectedPath,
      text: userInput,
      title: editorTitle,
    });
    if (sent && !override) {
      setPromptPills([]);
    }
  };

  const updateDraftSelection = useCallback(() => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection) return;
    const { start, end } = selection;
    draftSelectionRef.current = { start, end };
    setHasDraftSelection(end > start);
  }, []);

  const attachCurrentSelectionToPrompt = () => {
    const editor = draftEditorRef.current;
    if (!editor) return;
    const selection = readContentEditableSelection(editor);
    if (!selection) return;
    const selected = editorContent.slice(selection.start, selection.end).trim();
    if (!selected) return;
    setPromptPills((current) => upsertPromptContextPill(current, createSelectionPromptPill(selected, selection)));
    setPrompt((current) => current || "请修改这段。");
  };

  useEffect(() => {
    document.documentElement.classList.toggle("darkTheme", theme === "dark");
  }, [theme]);

  useEffect(() => {
    void invoke<WorkspaceInfo>("wridian_init_workspace")
      .then((response) => {
        setWorkspace(response);
        setWorkspaceError("");
      })
      .catch(() => setWorkspace(null));
  }, []);

  useEffect(() => {
    if (!memoryOpen) return;
    void loadMemoryTree();
  }, [loadMemoryTree, memoryOpen]);

  useEffect(() => {
    if (!knowledgeGraphOpen) return;
    void loadKnowledgeGraph();
  }, [knowledgeGraphOpen, loadKnowledgeGraph, workspace?.knowledgeFiles.length]);

  useEffect(() => {
    void invoke<CustomApiSettingsStatus>("wridian_get_custom_api_settings")
      .then((status) => setActiveModelLabel(status.model ?? "未配置模型"))
      .catch(() => setActiveModelLabel("未配置模型"));
  }, []);

  useEffect(() => {
    void getProjectState()
      .then(setProjectState)
      .catch((error) => setProjectError(error instanceof Error ? error.message : String(error)));
  }, [workspace?.files.length, workspace?.filesRootPath]);

  const files = workspace?.files ?? [];
  const knowledgeFiles = workspace?.knowledgeFiles ?? [];
  const visibleFiles = libraryTab === "works" ? files : knowledgeFiles;
  const activeLibraryConfigured = libraryTab === "knowledge"
    ? Boolean(workspace?.knowledgeRootConfigured)
    : Boolean(workspace?.workRootConfigured);
  const isRealFile = Boolean(selectedPath);
  const dirty = isRealFile && !loadingPath && editorContent !== lastSavedContent;

  const saveCurrentFile = useCallback(async () => {
    if (!isRealFile || loadingPath || !dirty) return;
    const pathToSave = selectedPath;
    const contentToSave = editorContent;
    setSaveStatus("saving");
    setSaveError("");
    try {
      await invoke<SaveFileResponse>("wridian_save_file", {
        input: { path: pathToSave, content: contentToSave },
      });
      if (selectedPath === pathToSave && editorContent === contentToSave) {
        setLastSavedContent(contentToSave);
      }
      setSaveStatus("saved");
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    }
  }, [dirty, editorContent, isRealFile, loadingPath, selectedPath]);

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
      setProjectState(await selectProject(id || null));
      setProjectError("");
    } catch (error) {
      setProjectError(error instanceof Error ? error.message : String(error));
    }
  };

  const openCurrentLibraryFolder = async () => {
    setWorkspaceError("");
    const path = libraryFolderPath(libraryTab, workspace);
    if (!path) {
      await chooseLibraryRoot();
      return;
    }
    try {
      const { openPath } = await import("@tauri-apps/plugin-opener");
      await openPath(path);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspaceError(message.includes("not allowed") || message.includes("Tauri") ? "请在 Wridian 桌面端打开本地文件夹。" : message);
    }
  };

  const refreshWorkspace = (response: WorkspaceInfo) => {
    setWorkspace(response);
    setWorkspaceError("");
  };

  const workspaceRootPath = libraryTab === "knowledge"
    ? activeLibraryConfigured ? workspace?.knowledgeRootPath || "" : ""
    : activeLibraryConfigured ? workspace?.filesRootPath || workspace?.activeWorkRoot || "" : "";

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
      const command = tab === "knowledge" ? "wridian_set_knowledge_root" : "wridian_set_work_root";
      refreshWorkspace(await invoke<WorkspaceInfo>(command, { input: { path: selected } }));
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspaceError(message.includes("not allowed") || message.includes("Tauri") ? "请在 Wridian 桌面端选择本地文件夹。" : message);
    }
  };

  const createFile = async (parentPath = workspaceRootPath) => {
    if (!parentPath) return;
    const name = window.prompt("新建文件", "未命名.md");
    if (!name) return;
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_create_work_file", { input: { parentPath, name } }));
  };

  const createFolder = async (parentPath = workspaceRootPath) => {
    if (!parentPath) return;
    const name = window.prompt("新建文件夹", "新建文件夹");
    if (!name) return;
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_create_work_folder", { input: { parentPath, name } }));
  };

  const duplicateNode = async (node: WorkFileNode) => {
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_duplicate_work_node", { input: { path: node.path } }));
  };

  const renameNode = async (node: WorkFileNode) => {
    const name = window.prompt("重命名", node.name);
    if (!name || name === node.name) return;
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_rename_work_node", { input: { path: node.path, newName: name } }));
  };

  const trashNode = async (node: WorkFileNode) => {
    await runWorkspaceAction(() => invoke<WorkspaceInfo>("wridian_trash_work_node", { input: { path: node.path } }));
  };

  const addNodeToPrompt = (node: WorkFileNode) => {
    if (node.folder) return;
    void addFileToPrompt(node.name, node.path, node.relativePath);
  };

  const addFileToPrompt = async (name: string, path: string, relativePath = "") => {
    try {
      const cached = promptFileContentCache[path];
      const content = cached ?? (await invoke<OpenFileResponse>("wridian_open_file", { input: { path } })).content;
      setPromptFileContentCache((current) => ({ ...current, [path]: content }));
      setPromptPills((current) => upsertPromptContextPill(
        current,
        relativePath
          ? createReferencedFileContentPromptPill(name, path, relativePath, content)
          : createFileContentPromptPill(name, path, content),
      ));
    } catch {
      setPromptPills((current) => upsertPromptContextPill(current, createFilePromptPill(name, path, relativePath)));
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

  const openFilePath = async (requestedPath: string, fallbackName = "") => {
    setLoadingPath(requestedPath);
    setEditorTitle(fallbackName || requestedPath.split(/[\\/]/).pop() || "未命名文件");
    setSaveError("");
    setSaveStatus("idle");
    try {
      const response = await invoke<OpenFileResponse>("wridian_open_file", { input: { path: requestedPath } });
      setSelectedPath(response.path);
      setEditorTitle(response.name);
      setEditorContent(response.content);
      setLastSavedContent(response.content);
      setPromptFileContentCache((current) => ({ ...current, [response.path]: response.content }));
      draftSelectionRef.current = { start: response.content.length, end: response.content.length };
      setHasDraftSelection(false);
      setPromptPills([]);
      setPendingEdits([]);
      setSaveStatus("saved");
      const project = projectState.projects.find((item) => response.path.startsWith(item.id));
      if (project && project.id !== projectState.activeProjectId) {
        void switchProject(project.id);
      } else if (!project && projectState.activeProjectId) {
        void switchProject("");
      }
    } catch (error) {
      setSaveStatus("error");
      setSaveError(error instanceof Error ? error.message : String(error));
    } finally {
      setLoadingPath((current) => (current === requestedPath ? "" : current));
    }
  };

  const openFile = async (node: WorkFileNode) => {
    if (node.folder) return;
    await openFilePath(node.path, node.name);
  };

  const openKnowledgeGraphFile = (path: string) => {
    setKnowledgeGraphOpen(false);
    void openFilePath(path);
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
    setHasDraftSelection(false);
    window.requestAnimationFrame(() => {
      setContentEditableCaret(draftEditorRef.current, nextCursor);
    });
  }, [editorContent]);

  const copyText = async (text: string) => {
    const reply = text.trim();
    if (!reply) return;
    try {
      await navigator.clipboard.writeText(reply);
    } catch {
    }
  };

  const editUserMessage = (message: ChatMessage) => {
    setPrompt(message.text);
    setPromptPills(restorePromptPillsFromMessage(message));
  };

  const retryLastUserMessage = (message: ChatMessage) => {
    setPromptPills(restorePromptPillsFromMessage(message));
    void sendPrompt({ text: message.text, selectedText: message.selectedText });
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
    setHasDraftSelection(false);

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
    draftKind,
    knowledgeCards: knowledgeSuggestionIndex.cards,
    knowledgeCategories: knowledgeSuggestionIndex.categories,
    selectedKnowledgeCategoryId,
  }), [draftKind, enabledCreativeSkills, knowledgeSuggestionIndex, selectedKnowledgeCategoryId]);

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

  const plantMemoryLeaf = async (candidate: MemoryLeafCandidate) => {
    setSavingMemoryTree(true);
    try {
      const response = await invoke<MemoryTreeState>("wridian_plant_memory_leaf", {
        input: {
          branch: candidate.branch,
          reason: candidate.reason,
          sourcePath: candidate.sourcePath,
          summary: candidate.summary,
          title: candidate.title,
        },
      });
      setMemoryTreeState(response);
      setMemoryLeafCandidate(null);
      setMemoryError("");
    } catch (error) {
      setMemoryError(error instanceof Error ? error.message : String(error));
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
      const { openPath } = await import("@tauri-apps/plugin-opener");
      await openPath(`${workspace.runtimePath}\\memory-tree`);
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
                title={activeLibraryConfigured ? libraryFolderTooltip(libraryTab) : libraryTab === "knowledge" ? "选择知识库文件夹" : "选择作品库文件夹"}
                aria-label={activeLibraryConfigured ? libraryFolderTooltip(libraryTab) : libraryTab === "knowledge" ? "选择知识库文件夹" : "选择作品库文件夹"}
                onClick={() => void openCurrentLibraryFolder()}
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
                  selectedPath={selectedPath}
                  onOpenFile={openFile}
                  onOpenMenu={openFileContextMenu}
                />
              ))
            ) : null}
          </div>

          <div className="rail-bottom">
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
          <section className={`paper ${selectedPath ? "" : "paper-empty"}`} aria-label="正文编辑区">
            {selectedPath ? (
              <div className="paper-topline">
                <div className="paper-kicker">{baseName(selectedPath)}</div>
                <div className="paper-actions">
                  <button type="button" className="paper-action" onClick={attachCurrentSelectionToPrompt} disabled={!hasDraftSelection}>
                    添加选区到输入框
                  </button>
                  <div className={`save-state ${saveStatus}`} title={saveError || undefined}>
                    {statusLabel}
                  </div>
                </div>
              </div>
            ) : null}
            {selectedPath ? (
              <>
                <h1 className="chapter-heading">{editorTitle}</h1>
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
                  onSelectionChange={updateDraftSelection}
                />
              </>
            ) : (
              <div className="empty-editor" aria-label="空文件编辑区">文件编辑区</div>
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
          error={chatManager.error}
          messages={chatManager.messages}
          onCopy={copyText}
          onEditUserMessage={editUserMessage}
          onRetry={retryLastUserMessage}
          pending={chatManager.pending}
          prompt={prompt}
          promptPills={promptPills}
          promptSuggestions={promptSuggestions}
          activeModelLabel={activeModelLabel}
          projectError={projectError}
          projects={projectState.projects}
          selectedProjectId={projectState.activeProjectId ?? ""}
          onSelectProject={(id) => void switchProject(id)}
          onPromptChange={setPrompt}
          onPromptPillsChange={setPromptPills}
          onImagePaste={(files) => {
            setPromptPills((current) => files.reduce(
              (next, file) => upsertPromptContextPill(next, createImagePromptPill(file.name || "pasted-image", file.size)),
              current,
            ));
          }}
          onRemovePill={(id) => setPromptPills((current) => current.filter((pill) => pill.id !== id))}
          onSelectSuggestion={(suggestion) => {
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
            setPromptPills((current) => upsertPromptContextPill(current, createPromptPillFromSuggestion(suggestion)));
          }}
          onSubmit={() => void sendPrompt()}
        />
      </div>

      {memoryOpen ? (
        <MemoryDrawer
          memoryError={memoryError}
          candidate={memoryLeafCandidate}
          memoryTree={memoryTreeState}
          onClose={() => setMemoryOpen(false)}
          onOpenMemoryFolder={openMemoryFolder}
          onPlantCandidate={plantMemoryLeaf}
          onRejectCandidate={() => setMemoryLeafCandidate(null)}
          onSaveFile={saveMemoryTreeFile}
          saving={savingMemoryTree}
        />
      ) : null}
      {knowledgeGraphOpen ? (
        <KnowledgeGraphDrawer
          graph={knowledgeGraphState}
          graphError={knowledgeGraphError}
          knowledgeRootConfigured={Boolean(workspace?.knowledgeRootConfigured)}
          onClose={() => setKnowledgeGraphOpen(false)}
          onOpenFile={openKnowledgeGraphFile}
          onRefresh={loadKnowledgeGraph}
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
      {settingsOpen ? <ModelSettingsDialog onClose={() => setSettingsOpen(false)} /> : null}
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

function FileNodeView({
  depth,
  node,
  onOpenFile,
  onOpenMenu,
  selectedPath,
}: {
  depth: number;
  node: WorkFileNode;
  onOpenFile: (node: WorkFileNode) => void;
  onOpenMenu: (node: WorkFileNode, x: number, y: number) => void;
  selectedPath: string;
}) {
  const [expanded, setExpanded] = useState(true);
  const isFolder = node.folder;
  const hasChildren = isFolder && node.children.length > 0;
  const fileExt = isFolder ? "" : fileExtension(node.name);
  const rowClassName = [
    "file-row",
    isFolder ? "folder" : "file",
    node.path === selectedPath ? "active" : "",
    isFolder && expanded ? "expanded" : "",
    isFolder && !expanded ? "collapsed" : "",
    isFolder && !hasChildren ? "empty-folder" : "",
  ]
    .filter(Boolean)
    .join(" ");

  const handleOpen = () => {
    if (isFolder) {
      setExpanded((current) => !current);
      return;
    }

    onOpenFile(node);
  };

  return (
    <div className="file-node">
      <button
        className={rowClassName}
        type="button"
        aria-expanded={isFolder ? expanded : undefined}
        title={node.relativePath || node.name}
        onClick={handleOpen}
        onContextMenu={(event) => {
          event.preventDefault();
          onOpenMenu(node, event.clientX, event.clientY);
        }}
      >
        <span className="tree-toggle" aria-hidden="true" />
        <strong>{isFolder ? node.name : fileTitle(node.name)}</strong>
        {fileExt ? <span className="file-ext">{fileExt}</span> : null}
      </button>
      {hasChildren && expanded ? (
        <div className="file-children">
          {node.children.map((child) => (
            <FileNodeView
              key={child.path}
              node={child}
              depth={depth + 1}
              selectedPath={selectedPath}
              onOpenFile={onOpenFile}
              onOpenMenu={onOpenMenu}
            />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function fileTitle(name: string) {
  const extensionStart = name.lastIndexOf(".");
  if (extensionStart <= 0) return name;
  return name.slice(0, extensionStart);
}

function fileExtension(name: string) {
  const extensionStart = name.lastIndexOf(".");
  if (extensionStart <= 0 || extensionStart === name.length - 1) return "";
  return name.slice(extensionStart + 1);
}

function FileContextMenuView({
  menu,
  onAddToPrompt,
  onClose,
  onCreateFile,
  onCreateFolder,
  onDuplicate,
  onRename,
  onTrash,
}: {
  menu: FileContextMenu;
  onAddToPrompt: (node: WorkFileNode) => void;
  onClose: () => void;
  onCreateFile: (parentPath?: string) => Promise<void>;
  onCreateFolder: (parentPath?: string) => Promise<void>;
  onDuplicate: (node: WorkFileNode) => Promise<void>;
  onRename: (node: WorkFileNode) => Promise<void>;
  onTrash: (node: WorkFileNode) => Promise<void>;
}) {
  const run = (action: () => void | Promise<void>) => {
    onClose();
    void action();
  };

  return (
    <div className="context-menu" style={{ left: menu.x, top: menu.y }} onClick={(event) => event.stopPropagation()}>
      {menu.node.folder ? (
        <>
          <button type="button" onClick={() => run(() => onCreateFile(menu.node.path))}>
            新建文件
          </button>
          <button type="button" onClick={() => run(() => onCreateFolder(menu.node.path))}>
            新建文件夹
          </button>
        </>
      ) : null}
      <button type="button" onClick={() => run(() => onDuplicate(menu.node))}>
        创建副本
      </button>
      <button type="button" onClick={() => run(() => onAddToPrompt(menu.node))}>
        添加到对话输入
      </button>
      <button type="button" onClick={() => run(() => onRename(menu.node))}>
        重命名
      </button>
      <button type="button" className="danger" onClick={() => run(() => onTrash(menu.node))}>
        移到回收站
      </button>
    </div>
  );
}

function baseName(path: string) {
  return path.replace(/[\\/]+$/g, "").split(/[\\/]/).pop() || path;
}

function detectDraftKind(path: string, content: string): DraftKind {
  const lowerPath = path.toLowerCase();
  if (lowerPath.endsWith(".fountain")) return "screenplay";

  const sceneSignals = (content.match(/(^|\n)\s*(INT\.|EXT\.|内景|外景|第[一二三四五六七八九十\d]+[集场])/g) ?? []).length;
  const dialogueSignals = (content.match(/(^|\n)\s*[\u4e00-\u9fa5A-Za-z0-9_]{2,12}[：:]/g) ?? []).length;
  return sceneSignals >= 2 || dialogueSignals >= 4 ? "screenplay" : "prose";
}

function clamp(value: number, min: number, max: number) {
  return Math.min(Math.max(value, min), Math.max(min, max));
}

function PencilIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M10 44H38C39.1046 44 40 43.1046 40 42V14H30V4H10C8.89543 4 8 4.89543 8 6V42C8 43.1046 8.89543 44 10 44Z" />
      <path d="M30 4L40 14" />
      <path d="M24 21V35" />
      <path d="M17 28H24L31 28" />
    </svg>
  );
}

function FolderPlusIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M5 8C5 6.89543 5.89543 6 7 6H19L24 12H41C42.1046 12 43 12.8954 43 14V40C43 41.1046 42.1046 42 41 42H7C5.89543 42 5 41.1046 5 40V8Z" />
      <path d="M18 27H30" />
      <path d="M24 21L24 33" />
    </svg>
  );
}

function WorkFolderIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M43 23V14C43 12.8954 42.1046 12 41 12H24L19 6H7C5.89543 6 5 6.89543 5 8V40C5 41.1046 5.89543 42 7 42H22" />
      <circle cx="35" cy="35" r="4" />
      <path d="M35 28V31" />
      <path d="M35 39V42" />
      <path d="M39.8281 30L37.7068 32.1213" />
      <path d="M31.8281 38L29.7068 40.1213" />
      <path d="M30 30L32.1213 32.1213" />
      <path d="M38 38L40.1213 40.1213" />
      <path d="M28 35H29.5H31" />
      <path d="M39 35H40.5H42" />
    </svg>
  );
}

function MemoryTreeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M13.0448 14C13.5501 8.3935 18.262 4 24 4C29.738 4 34.4499 8.3935 34.9552 14H35C39.9706 14 44 18.0294 44 23C44 27.9706 39.9706 32 35 32H13C8.02944 32 4 27.9706 4 23C4 18.0294 8.02944 14 13 14H13.0448Z" />
      <path d="M24 28L29 23" />
      <path d="M24 25L18 19" />
      <path d="M24 44V18" />
    </svg>
  );
}

function KnowledgeGraphIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M13.5 39.3706C16.3908 41.6439 20.0371 42.9999 24 42.9999C27.9629 42.9999 31.6092 41.6439 34.5 39.3706" />
      <path d="M19 9.74707C12.0513 11.8822 7 18.3511 7 25.9999C7 27.9247 7.31989 29.7748 7.9094 31.4999" />
      <path d="M29 9.74707C35.9487 11.8822 41 18.3511 41 25.9999C41 27.9247 40.6801 29.7748 40.0906 31.4999" />
      <path d="M43 36C43 37.3416 42.4716 38.5597 41.6117 39.4577C40.7015 40.4082 39.4199 41 38 41C35.2386 41 33 38.7614 33 36C33 33.9899 34.1861 32.2569 35.8967 31.4626C36.536 31.1657 37.2487 31 38 31C40.7614 31 43 33.2386 43 36Z" />
      <path d="M15 36C15 37.3416 14.4716 38.5597 13.6117 39.4577C12.7015 40.4082 11.4199 41 10 41C7.23858 41 5 38.7614 5 36C5 33.9899 6.18614 32.2569 7.89667 31.4626C8.53604 31.1657 9.24867 31 10 31C12.7614 31 15 33.2386 15 36Z" />
      <path d="M29 9C29 10.3416 28.4716 11.5597 27.6117 12.4577C26.7015 13.4082 25.4199 14 24 14C21.2386 14 19 11.7614 19 9C19 6.98991 20.1861 5.25686 21.8967 4.4626C22.536 4.16572 23.2487 4 24 4C26.7614 4 29 6.23858 29 9Z" />
    </svg>
  );
}

function LightningIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M27 4L10 27H22L19 44L38 19H25L27 4Z" />
    </svg>
  );
}

function ModelConfigIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M18 23.9372V10C18 6.68629 20.6863 4 24 4C27.3137 4 30 6.68629 30 10V12.0057" />
      <path d="M30 24.0034V37.9999C30 41.3136 27.3137 43.9999 24 43.9999C20.6863 43.9999 18 41.3136 18 37.9999V35.9699" />
      <path d="M24 30H9.98415C6.67919 30 4 27.3137 4 24C4 20.6863 6.67919 18 9.98415 18H11.9886" />
      <path d="M24 18H37.9888C41.3087 18 44 20.6863 44 24C44 27.3137 41.3087 30 37.9888 30H36.0663" />
    </svg>
  );
}

function FontSizeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M4 8H32" />
      <path d="M28 21H44" />
      <path d="M18 42L18 8" />
      <path d="M36 42L36 21" />
    </svg>
  );
}

function LightThemeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M24 37C31.1797 37 37 31.1797 37 24C37 16.8203 31.1797 11 24 11C16.8203 11 11 16.8203 11 24C11 31.1797 16.8203 37 24 37Z" />
      <circle cx="24" cy="3.5" r="2.5" />
      <circle cx="38.5" cy="9.5" r="2.5" />
      <circle cx="44.5" cy="24" r="2.5" />
      <circle cx="38.5" cy="38.5" r="2.5" />
      <circle cx="24" cy="44.5" r="2.5" />
      <circle cx="9.5" cy="38.5" r="2.5" />
      <circle cx="3.5" cy="24" r="2.5" />
      <circle cx="9.5" cy="9.5" r="2.5" />
    </svg>
  );
}

function DarkThemeIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M28.0527 4.41085C22.5828 5.83695 18.5455 10.8106 18.5455 16.7273C18.5455 23.7564 24.2436 29.4545 31.2727 29.4545C37.1894 29.4545 42.1631 25.4172 43.5891 19.9473C43.8585 21.256 44 22.6115 44 24C44 35.0457 35.0457 44 24 44C12.9543 44 4 35.0457 4 24C4 12.9543 12.9543 4 24 4C25.3885 4 26.744 4.14149 28.0527 4.41085Z" />
    </svg>
  );
}

function SettingsIcon() {
  return (
    <svg aria-hidden="true" viewBox="0 0 48 48">
      <path d="M18.2838 43.1713C14.9327 42.1736 11.9498 40.3213 9.58787 37.867C10.469 36.8227 11 35.4734 11 34.0001C11 30.6864 8.31371 28.0001 5 28.0001C4.79955 28.0001 4.60139 28.01 4.40599 28.0292C4.13979 26.7277 4 25.3803 4 24.0001C4 21.9095 4.32077 19.8938 4.91579 17.9995C4.94381 17.9999 4.97188 18.0001 5 18.0001C8.31371 18.0001 11 15.3138 11 12.0001C11 11.0488 10.7786 10.1493 10.3846 9.35011C12.6975 7.1995 15.5205 5.59002 18.6521 4.72314C19.6444 6.66819 21.6667 8.00013 24 8.00013C26.3333 8.00013 28.3556 6.66819 29.3479 4.72314C32.4795 5.59002 35.3025 7.1995 37.6154 9.35011C37.2214 10.1493 37 11.0488 37 12.0001C37 15.3138 39.6863 18.0001 43 18.0001C43.0281 18.0001 43.0562 17.9999 43.0842 17.9995C43.6792 19.8938 44 21.9095 44 24.0001C44 25.3803 43.8602 26.7277 43.594 28.0292C43.3986 28.01 43.2005 28.0001 43 28.0001C39.6863 28.0001 37 30.6864 37 34.0001C37 35.4734 37.531 36.8227 38.4121 37.867C36.0502 40.3213 33.0673 42.1736 29.7162 43.1713C28.9428 40.752 26.676 39.0001 24 39.0001C21.324 39.0001 19.0572 40.752 18.2838 43.1713Z" />
      <path d="M24 31C27.866 31 31 27.866 31 24C31 20.134 27.866 17 24 17C20.134 17 17 20.134 17 24C17 27.866 20.134 31 24 31Z" />
    </svg>
  );
}

function DraftEditor({
  content,
  editorRef,
  edits,
  onAcceptEdit,
  onChange,
  onKeyDown,
  onRejectEdit,
  onSelectionChange,
}: {
  content: string;
  editorRef: React.RefObject<HTMLDivElement | null>;
  edits: DraftEdit[];
  onAcceptEdit: (id: string) => void;
  onChange: (content: string) => void;
  onKeyDown: (event: ReactKeyboardEvent<HTMLDivElement>) => void;
  onRejectEdit: (id: string) => void;
  onSelectionChange: () => void;
}) {
  const chunks = buildDraftSuggestionChunks(content, edits);

  useEffect(() => {
    const editor = editorRef.current;
    if (!editor || edits.length) return;
    if (editor.innerText !== content) {
      editor.innerText = content;
    }
  }, [content, editorRef, edits.length]);

  return (
    <div
      ref={editorRef}
      className="draft-editor"
      contentEditable={!edits.length}
      role="textbox"
      aria-label="正文"
      spellCheck={false}
      suppressContentEditableWarning
      onInput={(event) => onChange(event.currentTarget.innerText)}
      onKeyDown={onKeyDown}
      onKeyUp={onSelectionChange}
      onMouseUp={onSelectionChange}
    >
      {chunks.map((chunk, index) => {
        if (chunk.kind === "text") {
          return <span key={`text-${index}`}>{chunk.text}</span>;
        }
        return (
          <span className="inline-edit" key={chunk.edit.id}>
            <span className="inline-diff">
              <del>{chunk.edit.target}</del>
              <ins>{chunk.edit.replacement}</ins>
            </span>
            {chunk.edit.rationale ? <small>{chunk.edit.rationale}</small> : null}
            <span className="inline-edit-actions" contentEditable={false}>
              <button type="button" onClick={() => onAcceptEdit(chunk.edit.id)}>确认</button>
              <button type="button" className="secondary" onClick={() => onRejectEdit(chunk.edit.id)}>取消</button>
            </span>
          </span>
        );
      })}
    </div>
  );
}

function readContentEditableSelection(root: HTMLElement): TextSelection | null {
  const selection = window.getSelection();
  if (!selection || selection.rangeCount === 0) return null;
  const range = selection.getRangeAt(0);
  if (!root.contains(range.startContainer) || !root.contains(range.endContainer)) return null;
  const beforeStart = range.cloneRange();
  beforeStart.selectNodeContents(root);
  beforeStart.setEnd(range.startContainer, range.startOffset);
  const beforeEnd = range.cloneRange();
  beforeEnd.selectNodeContents(root);
  beforeEnd.setEnd(range.endContainer, range.endOffset);
  const start = beforeStart.toString().length;
  const end = beforeEnd.toString().length;
  return { start: Math.min(start, end), end: Math.max(start, end) };
}

function setContentEditableCaret(root: HTMLElement | null, offset: number) {
  if (!root) return;
  root.focus();
  const walker = document.createTreeWalker(root, NodeFilter.SHOW_TEXT);
  let remaining = offset;
  let node = walker.nextNode();
  while (node) {
    const textLength = node.textContent?.length ?? 0;
    if (remaining <= textLength) {
      const range = document.createRange();
      range.setStart(node, remaining);
      range.collapse(true);
      const selection = window.getSelection();
      selection?.removeAllRanges();
      selection?.addRange(range);
      return;
    }
    remaining -= textLength;
    node = walker.nextNode();
  }
  const range = document.createRange();
  range.selectNodeContents(root);
  range.collapse(false);
  const selection = window.getSelection();
  selection?.removeAllRanges();
  selection?.addRange(range);
}

type DraftSuggestionChunk =
  | { kind: "text"; text: string }
  | { kind: "edit"; edit: DraftEdit };

function buildDraftSuggestionChunks(content: string, edits: DraftEdit[]): DraftSuggestionChunk[] {
  const matches = createDraftReplaceGuardReport(content, edits).matches;
  const chunks: DraftSuggestionChunk[] = [];
  let cursor = 0;

  for (const match of matches) {
    if (match.index < cursor) continue;
    if (match.index > cursor) {
      chunks.push({ kind: "text", text: content.slice(cursor, match.index) });
    }
    chunks.push({ kind: "edit", edit: match.edit });
    cursor = match.index + match.edit.target.length;
  }

  if (cursor < content.length) {
    chunks.push({ kind: "text", text: content.slice(cursor) });
  }
  return chunks.length ? chunks : [{ kind: "text", text: content }];
}

function MemoryDrawer({
  candidate,
  memoryError,
  memoryTree,
  onClose,
  onOpenMemoryFolder,
  onPlantCandidate,
  onRejectCandidate,
  onSaveFile,
  saving,
}: {
  candidate: MemoryLeafCandidate | null;
  memoryError: string;
  memoryTree: MemoryTreeState;
  onClose: () => void;
  onOpenMemoryFolder: () => void;
  onPlantCandidate: (candidate: MemoryLeafCandidate) => void;
  onRejectCandidate: () => void;
  onSaveFile: (path: string, content: string) => Promise<boolean>;
  saving: boolean;
}) {
  const viewModel = useMemo(() => buildMemoryTreeViewModel(memoryTree.roots), [memoryTree.roots]);
  const [selectedPath, setSelectedPath] = useState("");
  const [editorSide, setEditorSide] = useState<"left" | "right">("right");
  const selectedNode = useMemo(() => findMemoryNodeByPath(memoryTree.roots, selectedPath), [memoryTree.roots, selectedPath]);
  const [draft, setDraft] = useState(selectedNode?.content ?? "");
  const [transitionSaving, setTransitionSaving] = useState(false);
  const isBusy = saving || transitionSaving;

  useEffect(() => {
    setDraft(selectedNode?.content ?? "");
  }, [selectedNode?.content, selectedNode?.path]);

  const saveDirtyDraft = async () => {
    if (!selectedNode?.path || draft === (selectedNode.content ?? "")) return true;
    setTransitionSaving(true);
    try {
      return await onSaveFile(selectedNode.path, draft);
    } finally {
      setTransitionSaving(false);
    }
  };

  const selectNode = async (node: MemoryTreeNode | undefined, side: "left" | "right") => {
    if (isBusy) return;
    if (!node?.path || node.content == null) return;
    if (node.path === selectedPath) {
      setEditorSide(side);
      return;
    }
    const saved = await saveDirtyDraft();
    if (!saved) return;
    setEditorSide(side);
    setSelectedPath(node.path);
  };

  const save = async (closeAfterSave = false) => {
    if (isBusy) return false;
    const saved = await saveDirtyDraft();
    if (!saved) return false;
    if (closeAfterSave) {
      setSelectedPath("");
    }
    return true;
  };

  const closeEditorFromBlank = async () => {
    if (!selectedNode?.path) return;
    await save(true);
  };

  const closeDrawer = async () => {
    if (isBusy) return;
    const saved = await save(false);
    if (!saved) return;
    onClose();
  };

  return (
    <div className="drawer-backdrop" onMouseDown={() => void closeDrawer()} role="presentation">
      <aside className="memory-drawer memory-tree-drawer" role="dialog" aria-modal="true" aria-label="创作记忆树" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">创作记忆树</div>
          </div>
          <div className="drawer-header-actions">
            <button type="button" className="small-action" onClick={onOpenMemoryFolder} disabled={isBusy}>
              记忆文件夹
            </button>
            <button type="button" className="icon-button" onClick={() => void closeDrawer()} aria-label="关闭" disabled={isBusy}>
              ×
            </button>
          </div>
        </div>

        {memoryError ? <div className="rail-error">{memoryError}</div> : null}

        <div className="memory-forest-shell" aria-label="创作记忆树仿真视图">
          <div className="memory-forest" aria-label="创作记忆树" onMouseDown={() => void closeEditorFromBlank()}>
            <img className="memory-tree-base" src={memoryTreeBase} alt="" aria-hidden="true" />
            <div className="memory-tree-roots">
              <button
                type="button"
                className={`memory-sense-card ${viewModel.sense?.path === selectedPath ? "active" : ""}`}
                onMouseDown={(event) => event.stopPropagation()}
                onClick={() => void selectNode(viewModel.sense, "left")}
                disabled={isBusy}
              >
                <strong>自我意识</strong>
                <small>SENSE.md</small>
              </button>
              {viewModel.trunk.map((node) => (
                <button
                  type="button"
                  key={node.id}
                  className={`memory-trunk-card ${trunkNodeClass(node.label)} ${node.path === selectedPath ? "active" : ""}`}
                  onMouseDown={(event) => event.stopPropagation()}
                  onClick={() => void selectNode(node, "right")}
                  disabled={isBusy}
                >
                  <strong>{trunkTitleCn(node.label)}</strong>
                  <small>{node.label}</small>
                </button>
              ))}
            </div>
            {viewModel.branches.map((branch, index) => (
              <MemoryBranchArm
                key={branch.key}
                branch={branch}
                disabled={isBusy}
                index={index}
                selectedPath={selectedNode?.path ?? ""}
                onSelect={(node, side) => void selectNode(node, side)}
              />
            ))}
            {candidate ? (
              <section className="memory-node-detail editor-right candidate-panel" onMouseDown={(event) => event.stopPropagation()}>
                <div className="candidate-leaf-orbit" aria-hidden="true">
                  <span />
                </div>
                <div className="memory-tree-editor-header">
                  <div>
                    <h2>{candidate.title}</h2>
                    <p>候选叶子 / {branchLabel(candidate.branch)} / 等待确认</p>
                  </div>
                  <div className="candidate-actions">
                    <button type="button" onClick={() => onPlantCandidate(candidate)} disabled={isBusy}>
                      {isBusy ? "种下中" : "确认种下"}
                    </button>
                    <button type="button" className="secondary" onClick={onRejectCandidate} disabled={isBusy}>
                      放弃
                    </button>
                  </div>
                </div>
                <div className="candidate-body">
                  <p>{candidate.summary}</p>
                  <div>
                    <strong>为什么长出来</strong>
                    <p>{candidate.reason}</p>
                  </div>
                  <div>
                    <strong>将写入</strong>
                    <p>{candidate.targetPath}</p>
                  </div>
                </div>
              </section>
            ) : selectedNode?.path ? (
              <section className={`memory-node-detail editor-${editorSide}`} onMouseDown={(event) => event.stopPropagation()}>
                <div className="memory-tree-editor-header">
                  <div>
                    <h2>{selectedNode.label}</h2>
                    <p>{selectedNode.description}</p>
                  </div>
                  <button type="button" onClick={() => void save()} disabled={isBusy || draft === (selectedNode.content ?? "")}>
                    {isBusy ? "保存中" : "保存"}
                  </button>
                </div>
                <textarea
                  className="memory-tree-textarea"
                  value={draft}
                  onChange={(event) => setDraft(event.currentTarget.value)}
                  spellCheck={false}
                  aria-label={`编辑 ${selectedNode.label}`}
                />
              </section>
            ) : null}
          </div>
        </div>
      </aside>
    </div>
  );
}

type MemoryBranchView = {
  key: string;
  labelCn: string;
  label: string;
  leaves: MemoryTreeNode[];
  rule?: MemoryTreeNode;
};

const MEMORY_BRANCH_LAYOUT = [
  { key: "user", label: "USER.md", labelCn: "用户画像" },
  { key: "relationship", label: "RELATIONSHIP.md", labelCn: "关系准则" },
  { key: "journey", label: "JOURNEY.md", labelCn: "创作旅程" },
  { key: "drama", label: "DRAMA.md", labelCn: "剧本记忆" },
  { key: "novel", label: "NOVEL.md", labelCn: "小说记忆" },
  { key: "knowledge", label: "KNOWLEDGE.md", labelCn: "知识调用" },
  { key: "skill", label: "SKILL.md", labelCn: "技能方法" },
  { key: "awareness", label: "AWARENESS.md", labelCn: "复盘反思" },
] as const;

function buildMemoryTreeViewModel(roots: MemoryTreeNode[]) {
  const rootLayer = roots.find((node) => node.id === "totem");
  const branchLayer = roots.find((node) => node.id === "branches");
  const leafLayer = roots.find((node) => node.id === "leaves");
  const trunk = rootLayer?.children ?? [];
  const sense = branchLayer?.children.find((node) => node.label.toLowerCase().startsWith("sense"));
  const branches = MEMORY_BRANCH_LAYOUT.map(({ key, label, labelCn }) => {
    const rule = branchLayer?.children.find((node) => node.label.toLowerCase().startsWith(key));
    const leafRoot = leafLayer?.children.find((node) => node.label === key);
    return {
      key,
      label,
      labelCn,
      leaves: flattenMemoryLeaves(leafRoot),
      rule,
    };
  });
  return { branches, sense, trunk };
}

function flattenMemoryLeaves(node: MemoryTreeNode | undefined): MemoryTreeNode[] {
  if (!node) return [];
  const leaves: MemoryTreeNode[] = [];
  const visit = (item: MemoryTreeNode) => {
    if (item.content != null && item.path) {
      if (!item.label.toLowerCase().startsWith("legacy-")) {
        leaves.push(item);
      }
      return;
    }
    item.children.forEach(visit);
  };
  node.children.forEach(visit);
  return leaves;
}

function branchLabel(branch: string) {
  switch (branch) {
    case "sense":
      return "自我意识";
    case "user":
      return "用户画像";
    case "relationship":
      return "关系";
    case "journey":
      return "创作里程碑";
    case "drama":
      return "剧本";
    case "novel":
      return "小说";
    case "knowledge":
      return "知识";
    case "skill":
      return "技能";
    case "awareness":
      return "反思";
    default:
      return "记忆";
  }
}

function MemoryBranchArm({
  branch,
  disabled,
  onSelect,
  selectedPath,
}: {
  branch: MemoryBranchView;
  disabled: boolean;
  index: number;
  onSelect: (node: MemoryTreeNode, side: "left" | "right") => void;
  selectedPath: string;
}) {
  const side = ["user", "relationship", "drama", "knowledge"].includes(branch.key) ? "left" : "right";
  const editorSide = ["journey", "novel", "skill", "sense"].includes(branch.key) ? "left" : "right";
  const active = branch.rule?.path === selectedPath;
  const leafCount = branch.leaves.length;
  const leafSlots = Math.min(18, Math.max(1, leafCount));
  return (
    <div className={`memory-branch-arm ${side} branch-${branch.key} ${active ? "active" : ""}`}>
      <button
        type="button"
        className={`memory-branch-card ${active ? "active" : ""}`}
        onMouseDown={(event) => event.stopPropagation()}
        onClick={() => branch.rule ? onSelect(branch.rule, editorSide) : undefined}
        disabled={disabled}
      >
        <strong>{branch.labelCn}</strong>
        <small>{branch.label}</small>
      </button>
      <div className="memory-leaf-dots" aria-label={`${branch.labelCn}叶子`}>
        {branch.leaves.map((leaf, leafIndex) => (
          <button
            type="button"
            key={leaf.id}
            className={`memory-leaf-dot ${leaf.path === selectedPath ? "active" : ""}`}
            style={{
              "--leaf-angle": `${-120 + (leafIndex % leafSlots) * (240 / Math.max(1, leafSlots - 1))}deg`,
              "--leaf-radius": `${34 + Math.floor(leafIndex / 18) * 14 + (leafIndex % 3) * 8}px`,
            } as React.CSSProperties}
            title={leaf.label}
            aria-label={`打开记忆叶子 ${leaf.label}`}
            onMouseDown={(event) => event.stopPropagation()}
            onClick={() => onSelect(leaf, editorSide)}
            disabled={disabled}
          />
        ))}
      </div>
    </div>
  );
}

function trunkTitleCn(label: string) {
  if (label === "SOUL.md") return "图腾";
  if (label === "AGENTS.md") return "树根";
  if (label === "MEMORY.md") return "主干";
  return "根文件";
}

function trunkNodeClass(label: string) {
  if (label === "SOUL.md") return "totem";
  if (label === "AGENTS.md") return "root";
  if (label === "MEMORY.md") return "trunk";
  return "file";
}

function findMemoryNodeByPath(nodes: MemoryTreeNode[], path: string): MemoryTreeNode | undefined {
  for (const node of nodes) {
    if (node.path === path) return node;
    const child = findMemoryNodeByPath(node.children, path);
    if (child) return child;
  }
  return undefined;
}

function KnowledgeGraphDrawer({
  graph,
  graphError,
  knowledgeRootConfigured,
  onClose,
  onOpenFile,
  onRefresh,
}: {
  graph: KnowledgeGraphState;
  graphError: string;
  knowledgeRootConfigured: boolean;
  onClose: () => void;
  onOpenFile: (path: string) => void;
  onRefresh: () => void;
}) {
  const layout = useMemo(() => buildKnowledgeGraphLayout(graph), [graph]);
  const [viewport, setViewport] = useState({ scale: 1, x: 0, y: 0 });
  const [dragging, setDragging] = useState(false);
  const dragStateRef = useRef<{
    pointerId: number;
    startClientX: number;
    startClientY: number;
    startX: number;
    startY: number;
    moved: boolean;
  } | null>(null);
  const suppressGraphClickRef = useRef(false);

  const handleWheel = (event: ReactWheelEvent<HTMLDivElement>) => {
    if (!graph.nodes.length) return;
    event.preventDefault();
    setViewport((current) => ({
      ...current,
      scale: clamp(current.scale * (event.deltaY > 0 ? 0.9 : 1.1), 0.6, 3.2),
    }));
  };

  const handleGraphPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!graph.nodes.length || event.button !== 0) return;
    event.currentTarget.setPointerCapture(event.pointerId);
    dragStateRef.current = {
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startX: viewport.x,
      startY: viewport.y,
      moved: false,
    };
    setDragging(true);
  };

  const handleGraphPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const dragState = dragStateRef.current;
    if (!dragState || dragState.pointerId !== event.pointerId) return;
    const deltaX = event.clientX - dragState.startClientX;
    const deltaY = event.clientY - dragState.startClientY;
    const bounds = event.currentTarget.getBoundingClientRect();
    const svgDeltaX = (deltaX / Math.max(1, bounds.width)) * 100;
    const svgDeltaY = (deltaY / Math.max(1, bounds.height)) * 100;
    if (Math.abs(deltaX) + Math.abs(deltaY) > 4) dragState.moved = true;
    setViewport((current) => ({
      ...current,
      x: dragState.startX + svgDeltaX,
      y: dragState.startY + svgDeltaY,
    }));
  };

  const handleGraphPointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    const dragState = dragStateRef.current;
    if (dragState?.pointerId === event.pointerId) {
      if (dragState.moved) {
        suppressGraphClickRef.current = true;
        window.setTimeout(() => {
          suppressGraphClickRef.current = false;
        }, 120);
      }
      dragStateRef.current = null;
      setDragging(false);
    }
    if (event.currentTarget.hasPointerCapture(event.pointerId)) {
      event.currentTarget.releasePointerCapture(event.pointerId);
    }
  };

  const openGraphNode = (node: KnowledgeGraphLayoutNode) => {
    if (suppressGraphClickRef.current) {
      suppressGraphClickRef.current = false;
      return;
    }
    if (node.kind === "folder" || !node.path) return;
    onOpenFile(node.path);
  };

  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer knowledge-graph-drawer" role="dialog" aria-modal="true" aria-label="知识图谱" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">知识图谱</div>
          </div>
          <div className="drawer-header-actions">
            <button type="button" className="small-action" onClick={onRefresh}>
              刷新
            </button>
            <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
              ×
            </button>
          </div>
        </div>

        {graphError ? <div className="rail-error">{graphError}</div> : null}

        <div
          className={dragging ? "knowledge-graph-stage dragging" : "knowledge-graph-stage"}
          aria-label="知识库动态图谱"
          onPointerDown={handleGraphPointerDown}
          onPointerMove={handleGraphPointerMove}
          onPointerUp={handleGraphPointerUp}
          onPointerCancel={handleGraphPointerUp}
          onWheel={handleWheel}
        >
          {!knowledgeRootConfigured ? (
            <div className="knowledge-graph-empty">先选择知识库文件夹</div>
          ) : graph.nodes.length ? (
            <svg className="knowledge-graph-canvas" viewBox="0 0 100 100" role="img" aria-label="知识库动态图谱">
              <g className="graph-viewport" transform={`translate(${viewport.x} ${viewport.y}) translate(50 50) scale(${viewport.scale}) translate(-50 -50)`}>
                <g className="graph-motion">
                  {layout.edges.map((edge) => (
                    <line
                      key={`${edge.source.id}-${edge.target.id}-${edge.kind}`}
                      x1={edge.source.x}
                      y1={edge.source.y}
                      x2={edge.target.x}
                      y2={edge.target.y}
                      className={`graph-edge edge-${edge.kind}`}
                    />
                  ))}
                  {layout.nodes.map((node) => (
                    <g
                      key={node.id}
                      className={`graph-node node-${node.kind}`}
                      onClick={() => openGraphNode(node)}
                      style={{ "--node-fill": node.color } as CSSProperties}
                    >
                      <title>{node.path ?? node.label}</title>
                      <circle cx={node.x} cy={node.y} r={node.radius} />
                      {node.showLabel ? (
                        <text x={node.x} y={node.y + node.radius + 1.8}>{node.label}</text>
                      ) : null}
                    </g>
                  ))}
                </g>
              </g>
            </svg>
          ) : (
            <div className="knowledge-graph-empty">知识库里还没有 Markdown 知识卡</div>
          )}
        </div>
      </aside>
    </div>
  );
}

function CreativeSkillsDrawer({
  enabled,
  onClose,
  onToggle,
  skills,
}: {
  enabled: Record<CreativeSkillId, boolean>;
  onClose: () => void;
  onToggle: (id: CreativeSkillId) => void;
  skills: CreativeSkill[];
}) {
  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer creative-skills-drawer" role="dialog" aria-modal="true" aria-label="技能管理" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">技能管理</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <div className="creative-skills-list">
          {skills.map((skill) => (
            <div className="creative-skill-row" key={skill.id}>
              <div className="creative-skill-main">
                <div className="creative-skill-title">{skill.title}</div>
                <div className="creative-skill-meta">{skill.status}</div>
              </div>
              <button
                type="button"
                className={enabled[skill.id] ? "skill-toggle active" : "skill-toggle"}
                aria-pressed={enabled[skill.id]}
                onClick={() => onToggle(skill.id)}
              >
                <span />
              </button>
            </div>
          ))}
        </div>
      </aside>
    </div>
  );
}

type KnowledgeGraphLayoutNode = KnowledgeGraphNode & {
  color: string;
  collisionRadius: number;
  depth: number;
  radius: number;
  showLabel: boolean;
  x: number;
  y: number;
};

function buildKnowledgeGraphLayout(graph: KnowledgeGraphState) {
  const limited = graph.nodes.slice(0, 180);
  const nodes = limited.map((node, index): KnowledgeGraphLayoutNode => {
    const depth = knowledgeGraphNodeDepth(node);
    const depthSiblings = limited.filter((candidate) => knowledgeGraphNodeDepth(candidate) === depth);
    const siblingIndex = depthSiblings.findIndex((candidate) => candidate.id === node.id);
    const siblingCount = Math.max(1, depthSiblings.length);
    const groupHash = stableNumber(`${node.group}:${node.id}`);
    const depthRing = 9 + Math.min(depth, 5) * 8.4 + (node.kind === "folder" ? 0 : 4.8);
    const angle = ((siblingIndex / siblingCount) * 360 + (depth % 2) * 23 + (groupHash % 18)) * (Math.PI / 180);
    const radius = node.kind === "folder" ? 1.28 + Math.min(0.38, node.size / 42) : 0.78 + Math.min(0.32, node.size / 42);
    const showLabel = node.kind === "folder" || index < 38;
    const labelRadius = showLabel ? Math.min(10.5, Math.max(3.8, node.label.length * 0.46)) : 0;
    return {
      ...node,
      collisionRadius: radius + labelRadius + 1.25,
      color: knowledgeGraphNodeColor(depth),
      depth,
      radius,
      showLabel,
      x: clamp(50 + Math.cos(angle) * depthRing, 8, 92),
      y: clamp(50 + Math.sin(angle) * depthRing, 9, 91),
    };
  });
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const edges = graph.edges
    .map((edge) => ({
      kind: edge.kind,
      source: byId.get(edge.source),
      target: byId.get(edge.target),
    }))
    .filter((edge): edge is { kind: string; source: KnowledgeGraphLayoutNode; target: KnowledgeGraphLayoutNode } => Boolean(edge.source && edge.target))
    .slice(0, 260);
  relaxKnowledgeGraphLayout(nodes, edges);
  return { edges, nodes };
}

function relaxKnowledgeGraphLayout(
  nodes: KnowledgeGraphLayoutNode[],
  edges: { kind: string; source: KnowledgeGraphLayoutNode; target: KnowledgeGraphLayoutNode }[],
) {
  const centerX = 50;
  const centerY = 50;
  for (let iteration = 0; iteration < 90; iteration += 1) {
    for (const edge of edges) {
      const targetDistance = edge.kind === "contains" ? 12 + edge.target.depth * 1.6 : 18;
      const dx = edge.target.x - edge.source.x;
      const dy = edge.target.y - edge.source.y;
      const distance = Math.max(0.01, Math.hypot(dx, dy));
      const pull = (distance - targetDistance) * (edge.kind === "contains" ? 0.012 : 0.006);
      const moveX = (dx / distance) * pull;
      const moveY = (dy / distance) * pull;
      edge.source.x += moveX;
      edge.source.y += moveY;
      edge.target.x -= moveX;
      edge.target.y -= moveY;
    }

    for (let index = 0; index < nodes.length; index += 1) {
      const node = nodes[index];
      for (let otherIndex = index + 1; otherIndex < nodes.length; otherIndex += 1) {
        const other = nodes[otherIndex];
        let dx = other.x - node.x;
        let dy = other.y - node.y;
        let distance = Math.hypot(dx, dy);
        if (distance < 0.01) {
          const angle = ((stableNumber(`${node.id}:${other.id}`) % 360) * Math.PI) / 180;
          dx = Math.cos(angle) * 0.01;
          dy = Math.sin(angle) * 0.01;
          distance = 0.01;
        }
        const minimumDistance = node.collisionRadius + other.collisionRadius;
        if (distance >= minimumDistance) continue;
        const push = (minimumDistance - distance) * 0.48;
        const pushX = (dx / distance) * push;
        const pushY = (dy / distance) * push;
        node.x -= pushX;
        node.y -= pushY;
        other.x += pushX;
        other.y += pushY;
      }
    }

    for (const node of nodes) {
      const returnForce = node.kind === "folder" ? 0.006 : 0.003;
      node.x += (centerX - node.x) * returnForce;
      node.y += (centerY - node.y) * returnForce;
      node.x = clamp(node.x, node.collisionRadius, 100 - node.collisionRadius);
      node.y = clamp(node.y, node.collisionRadius + 1.8, 100 - node.collisionRadius);
    }
  }
}

function knowledgeGraphNodeDepth(node: KnowledgeGraphNode) {
  const source = node.id.replace(/^(folder|card):/, "");
  return Math.min(6, source.split(/[\\/]/).filter(Boolean).length);
}

function knowledgeGraphNodeColor(depth: number) {
  const colors = ["#b85d3f", "#c96b49", "#dc7d57", "#e49472", "#eeaa8a", "#f1bca3", "#d1714e"];
  return colors[Math.min(colors.length - 1, Math.max(0, depth))];
}

function stableNumber(value: string) {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash);
}

function buildKnowledgeSuggestionIndex(nodes: WorkFileNode[]) {
  const categories = new Map<string, KnowledgeCategory>();
  const cards: KnowledgeCardSuggestion[] = [];
  const visit = (node: WorkFileNode, categoryId = "") => {
    if (node.folder) {
      const id = node.relativePath || node.path;
      categories.set(id, {
        detail: `${countMarkdownCards(node)} 张知识卡`,
        id,
        title: node.name,
      });
      node.children.forEach((child) => visit(child, id));
      return;
    }
    if (!/\.(md|markdown)$/i.test(node.name)) return;
    const fallbackCategory = categoryId || "__root__";
    if (!categoryId && !categories.has(fallbackCategory)) {
      categories.set(fallbackCategory, {
        detail: "知识库根目录",
        id: fallbackCategory,
        title: "根目录",
      });
    }
    cards.push({
      category: categories.get(fallbackCategory)?.title ?? "知识卡",
      categoryId: fallbackCategory,
      id: node.path,
      relativePath: node.relativePath,
      sourcePath: node.path,
      title: node.name.replace(/\.(md|markdown)$/i, ""),
    });
  };
  nodes.forEach((node) => visit(node));
  return {
    cards,
    categories: [...categories.values()].filter((category) =>
      cards.some((card) => card.categoryId === category.id),
    ),
  };
}

function countMarkdownCards(node: WorkFileNode): number {
  if (!node.folder) return /\.(md|markdown)$/i.test(node.name) ? 1 : 0;
  return node.children.reduce((total, child) => total + countMarkdownCards(child), 0);
}

function ModelSettingsDialog({ onClose }: { onClose: () => void }) {
  const [baseUrl, setBaseUrl] = useState("");
  const [apiKey, setApiKey] = useState("");
  const [model, setModel] = useState("");
  const [maskedKey, setMaskedKey] = useState("");
  const [configured, setConfigured] = useState(false);
  const [busy, setBusy] = useState(false);
  const [message, setMessage] = useState("");

  useEffect(() => {
    void invoke<CustomApiSettingsStatus>("wridian_get_custom_api_settings")
      .then((status) => {
        setConfigured(status.configured);
        setBaseUrl(status.baseUrl ?? "");
        setModel(status.model ?? "");
        setMaskedKey(status.maskedKey ?? "");
      })
      .catch((error) => setMessage(error instanceof Error ? error.message : "请在 Wridian 桌面端配置模型账户。"));
  }, []);

  const saveSettings = async () => {
    setBusy(true);
    setMessage("");
    try {
      const status = await invoke<CustomApiSettingsStatus>("wridian_save_custom_api_settings", {
        input: { baseUrl, apiKey, model },
      });
      setConfigured(status.configured);
      setBaseUrl(status.baseUrl ?? "");
      setModel(status.model ?? "");
      setMaskedKey(status.maskedKey ?? "");
      setApiKey("");
      setMessage("已保存。");
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  const testSettings = async () => {
    setBusy(true);
    setMessage("");
    try {
      const response = await invoke<TestCustomApiResponse>("wridian_test_custom_api");
      setMessage(response.message || (response.ok ? "连接成功。" : "连接失败。"));
    } catch (error) {
      setMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="modal-backdrop" onMouseDown={onClose} role="presentation">
      <section className="settings-dialog" role="dialog" aria-modal="true" aria-label="模型账户" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">模型账户</div>
            <div className="drawer-subtitle">先接入一个 OpenAI-compatible 第三方 API。</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
            ×
          </button>
        </div>

        <div className="settings-form">
          <label>
            <span>Base URL</span>
            <input value={baseUrl} onChange={(event) => setBaseUrl(event.currentTarget.value)} placeholder="https://api.example.com/v1" />
          </label>
          <label>
            <span>API Key</span>
            <input
              value={apiKey}
              onChange={(event) => setApiKey(event.currentTarget.value)}
              placeholder={configured && maskedKey ? `已保存：${maskedKey}` : "sk-..."}
              type="password"
            />
          </label>
          <label>
            <span>Model</span>
            <input value={model} onChange={(event) => setModel(event.currentTarget.value)} placeholder="gpt-4o-mini" />
          </label>
        </div>

        {message ? <div className="settings-message">{message}</div> : null}

        <div className="settings-actions">
          <button type="button" className="secondary-action" onClick={() => void testSettings()} disabled={busy || !configured}>
            测试连接
          </button>
          <button type="button" className="primary-action" onClick={() => void saveSettings()} disabled={busy}>
            保存
          </button>
        </div>
      </section>
    </div>
  );
}

export default App;
