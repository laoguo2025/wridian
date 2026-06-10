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
import { libraryFolderPath, libraryFolderTooltip } from "./libraryToolbar";
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
  renameWorkNode,
  saveWorkFile,
  setLibraryRoot,
  trashWorkNode,
} from "./workspace/workspaceClient";
import type {
  KnowledgeGraphState,
  MemoryTreeState,
  CustomApiSettingsStatus,
  WorkFileNode,
  WorkspaceInfo,
} from "./appTypes";
import "./App.css";

type Theme = "light" | "dark";
type FontSizeMode = "default" | "large" | "max";
type SaveStatus = "idle" | "dirty" | "saving" | "saved" | "error";

type DraftEdit = ChatDraftEdit;

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
  const [knowledgeGraphState, setKnowledgeGraphState] = useState<KnowledgeGraphState>({ nodes: [], edges: [], warnings: [] });
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
  const [savingMemoryTree, setSavingMemoryTree] = useState(false);
  const [fileMenu, setFileMenu] = useState<FileContextMenu | null>(null);
  const [libraryTab, setLibraryTab] = useState<"works" | "knowledge">("works");
  const [knowledgeInboxOnly, setKnowledgeInboxOnly] = useState(false);
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
    void initWorkspace()
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
  const knowledgeInboxFiles = workspace?.knowledgeInboxFiles ?? [];
  const visibleFiles = libraryTab === "works" ? files : knowledgeInboxOnly ? knowledgeInboxFiles : knowledgeFiles;
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
      await saveWorkFile(pathToSave, contentToSave);
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
    try {
      const cached = promptFileContentCache[path];
      const content = cached ?? (await openWorkFile(path)).content;
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
      const response = await openWorkFile(requestedPath);
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
              <button type="button" className={libraryTab === "knowledge" ? "active" : ""} onClick={() => {
                setLibraryTab("knowledge");
                setKnowledgeInboxOnly(false);
              }}>
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
          {libraryTab === "knowledge" ? (
            <div className="knowledge-inbox-bar">
              <button
                type="button"
                className={knowledgeInboxOnly ? "active" : ""}
                onClick={() => setKnowledgeInboxOnly((current) => !current)}
                disabled={!activeLibraryConfigured}
                aria-pressed={knowledgeInboxOnly}
                title="显示待整理的知识文件"
              >
                <span>候选箱</span>
                <strong>{knowledgeInboxFiles.length}</strong>
              </button>
            </div>
          ) : null}

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
            {libraryTab === "knowledge" && knowledgeInboxOnly && !visibleFiles.length ? (
              <div className="file-tree-empty">没有待整理知识文件</div>
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
          memoryTree={memoryTreeState}
          onClose={() => setMemoryOpen(false)}
          onDeleteFile={deleteMemoryTreeFile}
          onOpenMemoryFolder={openMemoryFolder}
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

export default App;
