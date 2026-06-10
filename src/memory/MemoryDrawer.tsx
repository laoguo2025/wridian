import { type CSSProperties, useEffect, useMemo, useState } from "react";
import type { MemoryTreeNode, MemoryTreeState } from "../appTypes";
import memoryTreeBase from "../assets/memory-tree-base.png";

export function MemoryDrawer({
  memoryError,
  memoryTree,
  onClose,
  onDeleteFile,
  onOpenMemoryFolder,
  onSaveFile,
  saving,
}: {
  memoryError: string;
  memoryTree: MemoryTreeState;
  onClose: () => void;
  onDeleteFile: (path: string) => Promise<boolean>;
  onOpenMemoryFolder: () => void;
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

  const deleteSelected = async () => {
    if (isBusy || !selectedNode?.path) return;
    const confirmed = window.confirm("删除这片记忆叶子？此操作只删除记忆树 leaves 下的普通 Markdown 叶子文件。");
    if (!confirmed) return;
    setTransitionSaving(true);
    try {
      const deleted = await onDeleteFile(selectedNode.path);
      if (deleted) {
        setSelectedPath("");
      }
    } finally {
      setTransitionSaving(false);
    }
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
            {selectedNode?.path ? (
              <section className={`memory-node-detail editor-${editorSide}`} onMouseDown={(event) => event.stopPropagation()}>
                <div className="memory-tree-editor-header">
                  <div>
                    <h2>{selectedNode.label}</h2>
                    <p>{selectedNode.description}</p>
                  </div>
                  <div className="memory-tree-editor-actions">
                    <button type="button" className="delete-memory" onClick={() => void deleteSelected()} disabled={isBusy}>
                      删除
                    </button>
                    <button type="button" onClick={() => void save()} disabled={isBusy || draft === (selectedNode.content ?? "")}>
                      {isBusy ? "保存中" : "保存"}
                    </button>
                  </div>
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
            } as CSSProperties}
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
