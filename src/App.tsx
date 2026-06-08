import { KeyboardEvent, useEffect, useMemo, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type Theme = "light" | "dark";

type WorkspaceInfo = {
  vaultPath: string;
  runtimePath: string;
  activeWorkRoot?: string | null;
  files: WorkFileNode[];
};

type WorkFileNode = {
  name: string;
  path: string;
  folder: boolean;
  children: WorkFileNode[];
};

const fallbackFiles: WorkFileNode[] = [
  {
    name: "雾城手记",
    path: "demo://雾城手记",
    folder: true,
    children: [
      { name: "outline.md", path: "demo://outline.md", folder: false, children: [] },
      {
        name: "chapters",
        path: "demo://chapters",
        folder: true,
        children: [
          { name: "01.md", path: "demo://chapters/01.md", folder: false, children: [] },
          { name: "02.md", path: "demo://chapters/02.md", folder: false, children: [] },
          { name: "03.md", path: "demo://chapters/03.md", folder: false, children: [] },
        ],
      },
      {
        name: "characters",
        path: "demo://characters",
        folder: true,
        children: [{ name: "女主.md", path: "demo://characters/女主.md", folder: false, children: [] }],
      },
      { name: "world.md", path: "demo://world.md", folder: false, children: [] },
    ],
  },
];

const memoryItems = [
  "女主知道父亲失踪真相，但还没有证据。",
  "雨夜场景不能提前暴露凶手。",
  "第三章要强化她进门前的主动选择。",
];

const modelAccounts = [
  "Anthropic",
  "OpenAI / GPT",
  "Gemini",
  "Qwen",
  "DeepSeek",
  "Kimi",
  "Mimo",
  "字节 / Doubao",
  "自定义 API",
];

function App() {
  const [theme, setTheme] = useState<Theme>("light");
  const [memoryOpen, setMemoryOpen] = useState(false);
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [workspace, setWorkspace] = useState<WorkspaceInfo | null>(null);
  const [workspaceError, setWorkspaceError] = useState("");
  const [prompt, setPrompt] = useState("");

  const sendPrompt = () => {
    setPrompt("");
    setMemoryOpen(true);
  };

  const handlePromptKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key !== "Enter" || event.shiftKey) return;
    event.preventDefault();
    sendPrompt();
  };

  useEffect(() => {
    document.documentElement.classList.toggle("darkTheme", theme === "dark");
  }, [theme]);

  useEffect(() => {
    void invoke<WorkspaceInfo>("wridian_init_workspace")
      .then(setWorkspace)
      .catch(() => setWorkspace(null));
  }, []);

  const files = workspace?.files?.length ? workspace.files : fallbackFiles;

  const openWorkFolder = async () => {
    setWorkspaceError("");
    try {
      const { open } = await import("@tauri-apps/plugin-dialog");
      const selected = await open({ directory: true, multiple: false });
      if (typeof selected !== "string") return;
      const response = await invoke<WorkspaceInfo>("wridian_set_work_root", { input: { path: selected } });
      setWorkspace(response);
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error);
      setWorkspaceError(message.includes("not allowed") || message.includes("Tauri") ? "请在 Wridian 桌面端选择本地文件夹。" : message);
    }
  };

  const draftText = useMemo(
    () => [
      "她推开门的时候，雨水顺着袖口往下滴。",
      "",
      "屋里没有开灯，只有楼道里的光斜斜切进来。她没有立刻喊人。那一秒的停顿不像恐惧，更像确认。",
      "",
      "她已经知道里面会有什么。",
    ].join("\n"),
    [],
  );

  return (
    <div className="app-shell">
      <header className="topbar">
        <div className="brand">
          <span className="brand-mark" />
          <span>Wridian</span>
        </div>
        <nav className="top-actions" aria-label="Wridian actions">
          <button type="button" onClick={() => setMemoryOpen(true)}>
            记忆
          </button>
          <button type="button" onClick={() => setSettingsOpen(true)}>
            模型
          </button>
          <button type="button" onClick={() => setTheme(theme === "light" ? "dark" : "light")}>
            {theme === "light" ? "深色" : "浅色"}
          </button>
        </nav>
      </header>

      <div className="workspace">
        <aside className="project-rail" aria-label="作品">
          <div className="rail-topline">
            <div className="rail-section-title">作品</div>
            <button className="open-folder" type="button" onClick={() => void openWorkFolder()}>
              打开文件夹
            </button>
          </div>
          <button className="work-title active" type="button" title={workspace?.activeWorkRoot || workspace?.vaultPath || undefined}>
            <span>{workspace?.activeWorkRoot ? baseName(workspace.activeWorkRoot) : "Wridian Vault"}</span>
            <small>{workspace?.activeWorkRoot || "默认本地写作目录"}</small>
          </button>

          {workspaceError ? <div className="rail-error">{workspaceError}</div> : null}

          <div className="file-tree">
            {files.map((node) => (
              <FileNodeView key={node.path} node={node} depth={0} />
            ))}
          </div>

          <div className="rail-divider" />
          <button className="new-work" type="button">+ 新建</button>
        </aside>

        <main className="writing-pane">
          <section className="paper" aria-label="正文编辑区">
            <div className="paper-kicker">雾城手记</div>
            <input className="chapter-heading" defaultValue="第三章：雨夜" aria-label="章节标题" />
            <textarea className="draft-editor" defaultValue={draftText} aria-label="正文" spellCheck={false} />
          </section>

          <form
            className="prompt-bar"
            onSubmit={(event) => {
              event.preventDefault();
              sendPrompt();
            }}
          >
            <textarea
              value={prompt}
              onChange={(event) => setPrompt(event.currentTarget.value)}
              onKeyDown={handlePromptKeyDown}
              placeholder="Enter 发送，Shift + Enter 换行"
              aria-label="共创输入"
            />
            <button type="submit" aria-label="发送">↑</button>
          </form>
        </main>
      </div>

      {memoryOpen ? <MemoryDrawer onClose={() => setMemoryOpen(false)} workspace={workspace} /> : null}
      {settingsOpen ? <ModelSettingsDialog onClose={() => setSettingsOpen(false)} /> : null}
    </div>
  );
}

function FileNodeView({ depth, node }: { depth: number; node: WorkFileNode }) {
  return (
    <div className="file-node">
      <button
        className={node.folder ? "file-row folder" : node.name === "03.md" ? "file-row active" : "file-row"}
        type="button"
        style={{ paddingLeft: `${8 + depth * 12}px` }}
        title={node.path}
      >
        <span>{node.folder ? "▾" : ""}</span>
        <strong>{node.name}</strong>
      </button>
      {node.folder && node.children.length ? (
        <div className="file-children">
          {node.children.map((child) => (
            <FileNodeView key={child.path} node={child} depth={depth + 1} />
          ))}
        </div>
      ) : null}
    </div>
  );
}

function baseName(path: string) {
  return path.replace(/[\\/]+$/g, "").split(/[\\/]/).pop() || path;
}

function MemoryDrawer({ onClose, workspace }: { onClose: () => void; workspace: WorkspaceInfo | null }) {
  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer" role="dialog" aria-modal="true" aria-label="记忆" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">记忆</div>
            <div className="drawer-subtitle">当前作品：雾城手记</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">×</button>
        </div>

        <section className="memory-card">
          <h2>当前现场</h2>
          <p>第三章：雨夜</p>
          <p>上次讨论：女主进门前的动机。</p>
        </section>

        <section className="memory-card">
          <h2>相关记忆</h2>
          <ul>
            {memoryItems.map((item) => (
              <li key={item}>{item}</li>
            ))}
          </ul>
        </section>

        <section className="memory-card pending">
          <h2>待确认</h2>
          <p>第三章需要让女主动机提前出现。</p>
          <div className="drawer-actions">
            <button type="button">记住</button>
            <button type="button" className="secondary">忽略</button>
          </div>
        </section>

        <footer className="drawer-footer">
          {workspace?.vaultPath ? `Vault: ${workspace.vaultPath}` : "本地 Vault 初始化中"}
        </footer>
      </aside>
    </div>
  );
}

function ModelSettingsDialog({ onClose }: { onClose: () => void }) {
  return (
    <div className="modal-backdrop" onMouseDown={onClose} role="presentation">
      <section className="settings-dialog" role="dialog" aria-modal="true" aria-label="模型账户" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">模型账户</div>
            <div className="drawer-subtitle">开源版只保存用户自己的 API 账号。</div>
          </div>
          <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">×</button>
        </div>

        <div className="provider-list">
          {modelAccounts.map((provider) => (
            <button key={provider} className="provider-row" type="button">
              <span>{provider}</span>
              <small>未配置</small>
            </button>
          ))}
        </div>
      </section>
    </div>
  );
}

export default App;
