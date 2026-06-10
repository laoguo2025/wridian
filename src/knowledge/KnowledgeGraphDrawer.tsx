import { type PointerEvent as ReactPointerEvent, type WheelEvent as ReactWheelEvent, useEffect, useMemo, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import type { KnowledgeGraphNode, KnowledgeGraphState, OpenFileResponse } from "../appTypes";
import { clamp } from "../numberUtils";

export function KnowledgeGraphDrawer({
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
  const [activeView, setActiveView] = useState<KnowledgeGraphGovernanceView>("all");
  const viewCounts = useMemo(() => buildKnowledgeGraphViewCounts(graph), [graph]);
  const filteredGraph = useMemo(() => filterKnowledgeGraphState(graph, activeView), [activeView, graph]);
  const layout = useMemo(() => buildKnowledgeGraphLayout(filteredGraph), [filteredGraph]);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const graphAnimationFrameRef = useRef<number | null>(null);
  const [stageSize, setStageSize] = useState({ height: 520, width: 780 });
  const defaultCamera = useMemo(() => fitKnowledgeGraphCamera(layout.nodes, stageSize), [layout.nodes, stageSize]);
  const [camera, setCamera] = useState(defaultCamera);
  const [hoveredNode, setHoveredNode] = useState<KnowledgeGraphLayoutNode | null>(null);
  const [nodePreview, setNodePreview] = useState<{ path: string; content: string; error: string } | null>(null);
  const [graphRenderTick, forceGraphRender] = useState(0);
  const [dragging, setDragging] = useState(false);
  const safeCamera = sanitizeKnowledgeGraphCamera(camera, defaultCamera);
  const dragStateRef = useRef<{
    kind: "pan" | "node";
    pointerId: number;
    startClientX: number;
    startClientY: number;
    startGraphX: number;
    startGraphY: number;
    startNodeX?: number;
    startNodeY?: number;
    node?: KnowledgeGraphLayoutNode;
    startOffsetX: number;
    startOffsetY: number;
    moved: boolean;
  } | null>(null);
  const suppressGraphClickRef = useRef(false);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const updateSize = () => {
      const bounds = canvas.getBoundingClientRect();
      setStageSize({
        height: Math.max(1, Math.round(bounds.height)),
        width: Math.max(1, Math.round(bounds.width)),
      });
    };
    updateSize();
    const observer = new ResizeObserver(updateSize);
    observer.observe(canvas);
    return () => observer.disconnect();
  }, [knowledgeRootConfigured, graph.nodes.length]);

  useEffect(() => {
    setCamera(defaultCamera);
  }, [defaultCamera]);

  useEffect(() => {
    if (!knowledgeRootConfigured || !filteredGraph.nodes.length) return;
    let cancelled = false;
    const render = (time: number) => {
      if (cancelled) return;
      drawKnowledgeGraphCanvas(canvasRef.current, layout, safeCamera, hoveredNode?.id ?? null, time);
      graphAnimationFrameRef.current = window.requestAnimationFrame(render);
    };
    graphAnimationFrameRef.current = window.requestAnimationFrame(render);
    return () => {
      cancelled = true;
      if (graphAnimationFrameRef.current != null) {
        window.cancelAnimationFrame(graphAnimationFrameRef.current);
        graphAnimationFrameRef.current = null;
      }
    };
  }, [filteredGraph.nodes.length, graphRenderTick, knowledgeRootConfigured, layout, safeCamera, hoveredNode?.id]);

  useEffect(() => {
    if (!hoveredNode || hoveredNode.kind === "folder" || !hoveredNode.path) {
      setNodePreview(null);
      return;
    }
    const path = hoveredNode.path;
    let cancelled = false;
    const timer = window.setTimeout(() => {
      void invoke<OpenFileResponse>("wridian_open_file", { input: { path } })
        .then((response) => {
          if (!cancelled) setNodePreview({ path, content: response.content.slice(0, 520), error: "" });
        })
        .catch((error) => {
          if (!cancelled) setNodePreview({ path, content: "", error: error instanceof Error ? error.message : String(error) });
        });
    }, 220);
    return () => {
      cancelled = true;
      window.clearTimeout(timer);
    };
  }, [hoveredNode]);

  const clientToCanvasPoint = (event: ReactPointerEvent<HTMLDivElement>) => {
    const bounds = event.currentTarget.getBoundingClientRect();
    return {
      x: event.clientX - bounds.left,
      y: event.clientY - bounds.top,
    };
  };

  const clientToGraph = (event: ReactPointerEvent<HTMLDivElement>, view = safeCamera) => {
    const point = clientToCanvasPoint(event);
    return canvasPointToKnowledgeGraph(point, view);
  };

  const findEventNode = (event: ReactPointerEvent<HTMLDivElement>) => {
    const graphPoint = clientToGraph(event);
    return pickKnowledgeGraphNode(layout.nodes, graphPoint);
  };

  const handleWheel = (event: ReactWheelEvent<HTMLDivElement>) => {
    if (!filteredGraph.nodes.length) return;
    event.preventDefault();
    const bounds = event.currentTarget.getBoundingClientRect();
    const anchor = {
      x: event.clientX - bounds.left,
      y: event.clientY - bounds.top,
    };
    setCamera((current) =>
      zoomKnowledgeGraphCamera(
        sanitizeKnowledgeGraphCamera(current, defaultCamera),
        anchor,
        Math.exp(-event.deltaY * 0.0015),
      ),
    );
  };

  const handleGraphPointerDown = (event: ReactPointerEvent<HTMLDivElement>) => {
    if (!filteredGraph.nodes.length || event.button !== 0) return;
    const graphPoint = clientToGraph(event);
    const node = findEventNode(event);
    event.currentTarget.setPointerCapture(event.pointerId);
    dragStateRef.current = {
      kind: node ? "node" : "pan",
      pointerId: event.pointerId,
      startClientX: event.clientX,
      startClientY: event.clientY,
      startGraphX: graphPoint.x,
      startGraphY: graphPoint.y,
      startNodeX: node?.x,
      startNodeY: node?.y,
      node,
      startOffsetX: safeCamera.offsetX,
      startOffsetY: safeCamera.offsetY,
      moved: false,
    };
    setDragging(true);
  };

  const handleGraphPointerMove = (event: ReactPointerEvent<HTMLDivElement>) => {
    const dragState = dragStateRef.current;
    if (!dragState) {
      setHoveredNode(findEventNode(event) ?? null);
      return;
    }
    if (dragState.pointerId !== event.pointerId) return;
    const deltaX = event.clientX - dragState.startClientX;
    const deltaY = event.clientY - dragState.startClientY;
    if (Math.abs(deltaX) + Math.abs(deltaY) > 4) dragState.moved = true;
    if (dragState.kind === "node" && dragState.node && dragState.startNodeX != null && dragState.startNodeY != null) {
      const graphPoint = clientToGraph(event);
      dragState.node.x = clamp(dragState.startNodeX + graphPoint.x - dragState.startGraphX, dragState.node.collisionRadius, 100 - dragState.node.collisionRadius);
      dragState.node.y = clamp(dragState.startNodeY + graphPoint.y - dragState.startGraphY, dragState.node.collisionRadius + 1.8, 100 - dragState.node.collisionRadius);
      forceGraphRender((tick) => tick + 1);
      return;
    }
    setCamera((current) =>
      sanitizeKnowledgeGraphCamera(
        {
          ...current,
          offsetX: dragState.startOffsetX + deltaX,
          offsetY: dragState.startOffsetY + deltaY,
        },
        defaultCamera,
      ),
    );
  };

  const handleGraphPointerUp = (event: ReactPointerEvent<HTMLDivElement>) => {
    const dragState = dragStateRef.current;
    if (dragState?.pointerId === event.pointerId) {
      if (!dragState.moved && dragState.node) {
        openGraphNode(dragState.node);
      }
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

  const resetGraphView = () => {
    setCamera(fitKnowledgeGraphCamera(layout.nodes, stageSize));
  };
  const activePreview = hoveredNode?.path && nodePreview?.path === hoveredNode.path ? nodePreview : null;
  const activeNeighborhood = useMemo(() => buildKnowledgeGraphNeighborhood(graph, hoveredNode), [graph, hoveredNode]);

  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer knowledge-graph-drawer" role="dialog" aria-modal="true" aria-label="知识图谱" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">知识图谱</div>
            <div className="drawer-subtitle">{graph.relationships.length} 条关系，{graph.nodes.filter((node) => node.id.startsWith("card:")).length} 张卡片</div>
          </div>
          <div className="drawer-header-actions">
            <button type="button" className="small-action" onClick={resetGraphView}>
              重置视图
            </button>
            <button type="button" className="small-action" onClick={onRefresh}>
              刷新
            </button>
            <button type="button" className="icon-button" onClick={onClose} aria-label="关闭">
              ×
            </button>
          </div>
        </div>

        <div className="knowledge-graph-viewbar" role="tablist" aria-label="知识治理视图">
          {KNOWLEDGE_GRAPH_VIEWS.map((view) => (
            <button
              key={view.id}
              type="button"
              className={activeView === view.id ? "active" : ""}
              title={view.description}
              onClick={() => setActiveView(view.id)}
            >
              <span>{view.label}</span>
              <small>{viewCounts[view.id]}</small>
            </button>
          ))}
        </div>

        <div className="knowledge-graph-health-strip">
          <span>质量闸门：素材出处、关联索引、采纳沉淀、版本进化</span>
          <span>体检动作：补出处、补关联、合并、归档、重写</span>
        </div>

        {graphError ? <div className="rail-error">{graphError}</div> : null}
        {graph.warnings.length ? (
          <div className="rail-warning">
            {graph.warnings.slice(0, 3).map((warning) => (
              <div key={warning}>{warning}</div>
            ))}
            {graph.warnings.length > 3 ? <div>还有 {graph.warnings.length - 3} 条图谱提示。</div> : null}
          </div>
        ) : null}

        <div
          className={dragging ? "knowledge-graph-stage dragging" : "knowledge-graph-stage"}
          aria-label="知识库动态图谱"
          onPointerDown={handleGraphPointerDown}
          onPointerMove={handleGraphPointerMove}
          onPointerUp={handleGraphPointerUp}
          onPointerCancel={handleGraphPointerUp}
          onWheel={handleWheel}
          onMouseLeave={() => setHoveredNode(null)}
        >
          {!knowledgeRootConfigured ? (
            <div className="knowledge-graph-empty">先选择知识库文件夹</div>
          ) : filteredGraph.nodes.length ? (
            <canvas ref={canvasRef} className="knowledge-graph-canvas" aria-label="知识库动态图谱" />
          ) : (
            <div className="knowledge-graph-empty">{activeView === "all" ? "知识库里还没有 Markdown 知识卡" : "当前治理视图没有命中项"}</div>
          )}
          {hoveredNode ? (
            <div className="knowledge-graph-preview">
              <div className="knowledge-graph-preview-title">{hoveredNode.typeIcon ? `${hoveredNode.typeIcon} ` : ""}{hoveredNode.label}</div>
              <div className="knowledge-graph-preview-path">{hoveredNode.path ?? hoveredNode.group}</div>
              {hoveredNode.kind === "folder" ? (
                <div className="knowledge-graph-preview-body">分类文件夹</div>
              ) : activePreview?.content ? (
                <>
                  <div className="knowledge-graph-preview-meta">
                    <span>{knowledgeGraphNodeKindLabel(hoveredNode.kind)}</span>
                    <span>入链 {hoveredNode.inboundCount}</span>
                    <span>出链 {hoveredNode.outboundCount}</span>
                    {hoveredNode.usedByWorks.length ? <span>作品引用 {hoveredNode.usedByWorks.length}</span> : null}
                    {hoveredNode.reviewStatus ? <span>体检 {hoveredNode.reviewStatus}</span> : null}
                    {hoveredNode.hasConflict ? <span>有冲突</span> : null}
                    {hoveredNode.hasUncertainty ? <span>待核查</span> : null}
                  </div>
                  {hoveredNode.defaultFields.length ? (
                    <div className="knowledge-graph-preview-fields">{hoveredNode.defaultFields.slice(0, 5).map(knowledgeGraphRelationLabel).join(" / ")}</div>
                  ) : null}
                  {hoveredNode.referencedBy.length ? (
                    <div className="knowledge-graph-preview-fields">被知识卡引用：{hoveredNode.referencedBy.slice(0, 4).join(" / ")}</div>
                  ) : null}
                  {hoveredNode.usedByWorks.length ? (
                    <div className="knowledge-graph-preview-fields">被作品引用：{hoveredNode.usedByWorks.slice(0, 4).join(" / ")}</div>
                  ) : null}
                  <KnowledgeGraphNeighborhoodPanel neighborhood={activeNeighborhood} />
                  <div className="knowledge-graph-preview-body">{activePreview.content}</div>
                </>
              ) : activePreview?.error ? (
                <>
                  <KnowledgeGraphNeighborhoodPanel neighborhood={activeNeighborhood} />
                  <div className="knowledge-graph-preview-body">{activePreview.error}</div>
                </>
              ) : (
                <>
                  <KnowledgeGraphNeighborhoodPanel neighborhood={activeNeighborhood} />
                  <div className="knowledge-graph-preview-body">读取中...</div>
                </>
              )}
            </div>
          ) : null}
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

type KnowledgeGraphNeighborhoodItem = {
  bidirectional: boolean;
  id: string;
  label: string;
  path?: string | null;
  relationLabel: string;
};

type KnowledgeGraphNeighborhoodGroup = {
  count: number;
  label: string;
};

type KnowledgeGraphNeighborhood = {
  groups: KnowledgeGraphNeighborhoodGroup[];
  inbound: KnowledgeGraphNeighborhoodItem[];
  outbound: KnowledgeGraphNeighborhoodItem[];
};

function KnowledgeGraphNeighborhoodPanel({ neighborhood }: { neighborhood: KnowledgeGraphNeighborhood }) {
  const hasLinks = neighborhood.outbound.length > 0 || neighborhood.inbound.length > 0;
  if (!hasLinks) {
    return (
      <div className="knowledge-graph-neighborhood">
        <div className="knowledge-graph-neighborhood-title">关系邻域</div>
        <div className="knowledge-graph-neighborhood-empty">暂无出链或反链</div>
      </div>
    );
  }
  return (
    <div className="knowledge-graph-neighborhood">
      <div className="knowledge-graph-neighborhood-title">关系邻域</div>
      {neighborhood.groups.length ? (
        <div className="knowledge-graph-neighborhood-groups">
          {neighborhood.groups.slice(0, 5).map((group) => (
            <span key={group.label}>{group.label} {group.count}</span>
          ))}
        </div>
      ) : null}
      <KnowledgeGraphNeighborhoodList title="指向" items={neighborhood.outbound} />
      <KnowledgeGraphNeighborhoodList title="反链" items={neighborhood.inbound} />
    </div>
  );
}

function KnowledgeGraphNeighborhoodList({ items, title }: { items: KnowledgeGraphNeighborhoodItem[]; title: string }) {
  if (!items.length) return null;
  return (
    <div className="knowledge-graph-neighborhood-list">
      <div className="knowledge-graph-neighborhood-list-title">{title}</div>
      {items.slice(0, 4).map((item) => (
        <div className="knowledge-graph-neighborhood-row" key={`${title}:${item.id}:${item.relationLabel}`}>
          <span>{item.relationLabel}{item.bidirectional ? "·双向" : ""}</span>
          <strong>{item.label}</strong>
        </div>
      ))}
      {items.length > 4 ? <div className="knowledge-graph-neighborhood-more">还有 {items.length - 4} 个节点</div> : null}
    </div>
  );
}

type KnowledgeGraphGovernanceView =
  | "all"
  | "noSource"
  | "unreferenced"
  | "adoptedOpen"
  | "islands"
  | "duplicateTitles"
  | "reviewNeeded"
  | "staleHighReference";

const KNOWLEDGE_GRAPH_VIEWS: { id: KnowledgeGraphGovernanceView; label: string; description: string }[] = [
  { id: "all", label: "全部", description: "查看当前知识库所有 Markdown 节点和关系。" },
  { id: "noSource", label: "缺素材出处", description: "优先补素材出处；S 级知识卡必须能回到原始材料或拆解报告。" },
  { id: "unreferenced", label: "关联索引空", description: "检查是否缺少关联索引，或是否应降级、合并、归档。" },
  { id: "adoptedOpen", label: "采纳未沉淀", description: "已被作品采纳，但尚未改写沉淀为作品设定或规则。" },
  { id: "islands", label: "孤岛待归档", description: "没有入链也没有出链，优先判断补关联还是归档。" },
  { id: "duplicateTitles", label: "重复待合并", description: "标题重复，进入合并、改名或区分概念候选。" },
  { id: "reviewNeeded", label: "待核查冲突", description: "展示 zhishiku-skill 标记的冲突、不确定性或待核查知识卡。" },
  { id: "staleHighReference", label: "高频老化", description: "被高频引用但长期未进化，优先复查或重写。" },
];

type KnowledgeGraphCamera = {
  offsetX: number;
  offsetY: number;
  scale: number;
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
      color: knowledgeGraphTypedNodeColor(node, depth),
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

function filterKnowledgeGraphState(graph: KnowledgeGraphState, view: KnowledgeGraphGovernanceView): KnowledgeGraphState {
  if (view === "all") return graph;
  const nodes = graph.nodes.filter((node) => nodeMatchesKnowledgeGraphView(node, view));
  const nodeIds = new Set(nodes.map((node) => node.id));
  return {
    ...graph,
    nodes,
    edges: graph.edges.filter((edge) => nodeIds.has(edge.source) && nodeIds.has(edge.target)),
  };
}

function buildKnowledgeGraphViewCounts(graph: KnowledgeGraphState): Record<KnowledgeGraphGovernanceView, number> {
  return KNOWLEDGE_GRAPH_VIEWS.reduce((counts, view) => {
    counts[view.id] = view.id === "all"
      ? graph.nodes.filter((node) => node.id.startsWith("card:")).length
      : graph.nodes.filter((node) => nodeMatchesKnowledgeGraphView(node, view.id)).length;
    return counts;
  }, {} as Record<KnowledgeGraphGovernanceView, number>);
}

function nodeMatchesKnowledgeGraphView(node: KnowledgeGraphNode, view: KnowledgeGraphGovernanceView) {
  if (!node.id.startsWith("card:") || node.kind === "type-definition") return false;
  if (view === "noSource") return isSourceRequiredKnowledgeNode(node) && !node.hasSource;
  if (view === "unreferenced") return node.outboundCount === 0;
  if (view === "adoptedOpen") return node.adoptedButNotDistilled;
  if (view === "islands") return node.inboundCount === 0 && node.outboundCount === 0;
  if (view === "duplicateTitles") return node.duplicateTitle || node.duplicateConcept;
  if (view === "reviewNeeded") return node.hasConflict || node.hasUncertainty || knowledgeGraphReviewStatusNeedsAttention(node.reviewStatus);
  if (view === "staleHighReference") return node.staleHighReference;
  return true;
}

function knowledgeGraphReviewStatusNeedsAttention(status?: string | null) {
  if (!status) return false;
  return /待|需|冲突|不确定|核查|复审|过期|老化|review|conflict|uncertain|stale/i.test(status);
}

function isSourceRequiredKnowledgeNode(node: KnowledgeGraphNode) {
  const kind = node.kind.toLowerCase();
  return kind === "knowledge_card" || kind === "knowledge-card" || kind === "method" || kind === "skill_output" || kind === "skill-output";
}

function buildKnowledgeGraphNeighborhood(graph: KnowledgeGraphState, node: KnowledgeGraphNode | null): KnowledgeGraphNeighborhood {
  if (!node || node.kind === "folder" || !node.id.startsWith("card:")) {
    return { groups: [], inbound: [], outbound: [] };
  }
  const nodesById = new Map(graph.nodes.map((candidate) => [candidate.id, candidate]));
  const bidirectionalKeys = new Set(
    graph.relationships
      .filter((relation) => relation.bidirectional)
      .flatMap((relation) => {
        const source = `card:${relation.sourceFile}`;
        const target = `card:${relation.targetFile}`;
        const kind = `frontmatter:${relation.fieldName}`;
        return [`${source}->${target}->${kind}`, `${target}->${source}->${kind}`];
      }),
  );
  const itemForEdge = (edge: { source: string; target: string; kind: string }, direction: "inbound" | "outbound"): KnowledgeGraphNeighborhoodItem | null => {
    const neighborId = direction === "outbound" ? edge.target : edge.source;
    const neighbor = nodesById.get(neighborId);
    if (!neighbor || neighbor.kind === "folder") return null;
    return {
      bidirectional: bidirectionalKeys.has(`${edge.source}->${edge.target}->${edge.kind}`),
      id: neighbor.id,
      label: neighbor.label,
      path: neighbor.path,
      relationLabel: knowledgeGraphRelationLabel(edge.kind),
    };
  };
  const relationEdges = graph.edges.filter((edge) => edge.kind !== "contains" && (edge.source === node.id || edge.target === node.id));
  const outbound = relationEdges
    .filter((edge) => edge.source === node.id)
    .map((edge) => itemForEdge(edge, "outbound"))
    .filter((item): item is KnowledgeGraphNeighborhoodItem => Boolean(item))
    .sort(knowledgeGraphNeighborhoodItemSort);
  const inbound = relationEdges
    .filter((edge) => edge.target === node.id)
    .map((edge) => itemForEdge(edge, "inbound"))
    .filter((item): item is KnowledgeGraphNeighborhoodItem => Boolean(item))
    .sort(knowledgeGraphNeighborhoodItemSort);
  const groupCounts = new Map<string, number>();
  for (const item of [...outbound, ...inbound]) {
    groupCounts.set(item.relationLabel, (groupCounts.get(item.relationLabel) ?? 0) + 1);
  }
  const groups = [...groupCounts.entries()]
    .map(([label, count]) => ({ count, label }))
    .sort((left, right) => right.count - left.count || left.label.localeCompare(right.label, "zh-Hans-CN"));
  return { groups, inbound, outbound };
}

function knowledgeGraphNeighborhoodItemSort(left: KnowledgeGraphNeighborhoodItem, right: KnowledgeGraphNeighborhoodItem) {
  return left.relationLabel.localeCompare(right.relationLabel, "zh-Hans-CN") || left.label.localeCompare(right.label, "zh-Hans-CN");
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

function knowledgeGraphTypedNodeColor(node: KnowledgeGraphNode, depth: number) {
  if (node.typeColor && /^#[0-9a-f]{3}([0-9a-f]{3})?$/i.test(node.typeColor)) return node.typeColor;
  if (node.kind === "folder") return knowledgeGraphNodeColor(depth);
  const normalizedKind = node.kind.toLowerCase();
  if (normalizedKind.includes("source")) return "#5f8f7b";
  if (normalizedKind.includes("skill")) return "#c69348";
  if (normalizedKind.includes("concept")) return "#5f79b8";
  if (normalizedKind.includes("entity")) return "#6f7fb7";
  if (normalizedKind.includes("method") || normalizedKind.includes("knowledge")) return "#dc7d57";
  return knowledgeGraphNodeColor(depth);
}

const DEFAULT_KNOWLEDGE_GRAPH_FIT_RATIO = 0.64;

function fitKnowledgeGraphCamera(nodes: KnowledgeGraphLayoutNode[], size = { height: 520, width: 780 }): KnowledgeGraphCamera {
  const width = Math.max(1, size.width);
  const height = Math.max(1, size.height);
  if (!nodes.length) return { offsetX: width / 2, offsetY: height / 2, scale: 1 };
  let minX = Infinity;
  let minY = Infinity;
  let maxX = -Infinity;
  let maxY = -Infinity;
  for (const node of nodes) {
    const padding = node.collisionRadius + 2.5;
    minX = Math.min(minX, node.x - padding);
    minY = Math.min(minY, node.y - padding);
    maxX = Math.max(maxX, node.x + padding);
    maxY = Math.max(maxY, node.y + padding);
  }
  if (!Number.isFinite(minX)) return { offsetX: width / 2, offsetY: height / 2, scale: 1 };
  const graphWidth = Math.max(1, maxX - minX);
  const graphHeight = Math.max(1, maxY - minY);
  const fittedScale = Math.min((width - 52) / graphWidth, (height - 52) / graphHeight);
  const scale = clamp(fittedScale * DEFAULT_KNOWLEDGE_GRAPH_FIT_RATIO, 2.4, 7.8);
  const centerX = (minX + maxX) / 2;
  const centerY = (minY + maxY) / 2;
  return {
    offsetX: width / 2 - centerX * scale,
    offsetY: height / 2 - centerY * scale,
    scale,
  };
}

function sanitizeKnowledgeGraphCamera(
  camera: KnowledgeGraphCamera,
  fallback: KnowledgeGraphCamera = { offsetX: 390, offsetY: 260, scale: 6 },
): KnowledgeGraphCamera {
  const fallbackScale = Number.isFinite(fallback.scale) && fallback.scale > 0 ? fallback.scale : 6;
  const fallbackOffsetX = Number.isFinite(fallback.offsetX) ? fallback.offsetX : 390;
  const fallbackOffsetY = Number.isFinite(fallback.offsetY) ? fallback.offsetY : 260;
  const scale = Number.isFinite(camera.scale) && camera.scale > 0 ? clamp(camera.scale, 2.4, 32) : fallbackScale;
  const offsetX = Number.isFinite(camera.offsetX) ? clamp(camera.offsetX, -6000, 6000) : fallbackOffsetX;
  const offsetY = Number.isFinite(camera.offsetY) ? clamp(camera.offsetY, -6000, 6000) : fallbackOffsetY;
  return { offsetX, offsetY, scale };
}

function zoomKnowledgeGraphCamera(
  camera: KnowledgeGraphCamera,
  anchor: { x: number; y: number },
  factor: number,
): KnowledgeGraphCamera {
  const base = sanitizeKnowledgeGraphCamera(camera);
  const safeFactor = Number.isFinite(factor) && factor > 0 ? clamp(factor, 0.25, 4) : 1;
  const anchorX = Number.isFinite(anchor.x) ? anchor.x : 390;
  const anchorY = Number.isFinite(anchor.y) ? anchor.y : 260;
  const graphPoint = canvasPointToKnowledgeGraph({ x: anchorX, y: anchorY }, base);
  const scale = clamp(base.scale * safeFactor, 2.4, 32);
  return sanitizeKnowledgeGraphCamera({
    offsetX: anchorX - graphPoint.x * scale,
    offsetY: anchorY - graphPoint.y * scale,
    scale,
  });
}

function canvasPointToKnowledgeGraph(point: { x: number; y: number }, camera: KnowledgeGraphCamera) {
  const safe = sanitizeKnowledgeGraphCamera(camera);
  return {
    x: (point.x - safe.offsetX) / safe.scale,
    y: (point.y - safe.offsetY) / safe.scale,
  };
}

function knowledgeGraphToCanvasPoint(node: { x: number; y: number }, camera: KnowledgeGraphCamera) {
  return {
    x: node.x * camera.scale + camera.offsetX,
    y: node.y * camera.scale + camera.offsetY,
  };
}

function pickKnowledgeGraphNode(nodes: KnowledgeGraphLayoutNode[], point: { x: number; y: number }) {
  let best: KnowledgeGraphLayoutNode | undefined;
  let bestDistance = Infinity;
  for (const node of nodes) {
    const radius = node.radius + 1.8;
    const dx = node.x - point.x;
    const dy = node.y - point.y;
    const distance = dx * dx + dy * dy;
    if (distance <= radius * radius && distance < bestDistance) {
      best = node;
      bestDistance = distance;
    }
  }
  return best;
}

function drawKnowledgeGraphCanvas(
  canvas: HTMLCanvasElement | null,
  layout: ReturnType<typeof buildKnowledgeGraphLayout>,
  camera: KnowledgeGraphCamera,
  hoveredNodeId: string | null,
  time = 0,
) {
  if (!canvas) return;
  const bounds = canvas.getBoundingClientRect();
  const width = Math.max(1, Math.round(bounds.width));
  const height = Math.max(1, Math.round(bounds.height));
  const ratio = window.devicePixelRatio || 1;
  const pixelWidth = Math.max(1, Math.round(width * ratio));
  const pixelHeight = Math.max(1, Math.round(height * ratio));
  if (canvas.width !== pixelWidth || canvas.height !== pixelHeight) {
    canvas.width = pixelWidth;
    canvas.height = pixelHeight;
  }
  const context = canvas.getContext("2d");
  if (!context) return;
  context.setTransform(ratio, 0, 0, ratio, 0, 0);
  context.clearRect(0, 0, width, height);
  const safeCamera = sanitizeKnowledgeGraphCamera(camera);
  const motion = time / 1000;
  const edgeDashOffset = -(motion * 18) % 18;

  context.save();
  context.lineCap = "round";
  context.lineJoin = "round";
  for (const edge of layout.edges) {
    const source = knowledgeGraphToCanvasPoint(edge.source, safeCamera);
    const target = knowledgeGraphToCanvasPoint(edge.target, safeCamera);
    const relationKind = knowledgeGraphRelationKind(edge.kind);
    context.beginPath();
    context.moveTo(source.x, source.y);
    context.lineTo(target.x, target.y);
    context.strokeStyle = knowledgeGraphEdgeColor(relationKind);
    context.lineWidth = knowledgeGraphEdgeWidth(relationKind);
    context.setLineDash(knowledgeGraphEdgeDash(relationKind));
    context.lineDashOffset = relationKind === "frontmatter" ? 0 : edgeDashOffset;
    context.stroke();
    if (relationKind === "frontmatter") {
      drawKnowledgeGraphEdgeLabel(context, source, target, knowledgeGraphRelationLabel(edge.kind));
    }
  }
  context.setLineDash([]);
  context.lineDashOffset = 0;

  for (const [index, node] of layout.nodes.entries()) {
    const point = knowledgeGraphToCanvasPoint(node, safeCamera);
    const pulseWave = Math.sin(motion * 2.05 + index * 0.42);
    const pulse = 0.5 + pulseWave * 0.5;
    const baseRadius = Math.max(3.4, node.radius * safeCamera.scale);
    const radius = baseRadius * (1 + pulseWave * (node.kind === "card" ? 0.075 : 0.055));
    const hovered = hoveredNodeId === node.id;
    if (node.kind !== "card" || hovered) {
      const haloRadius = radius * (hovered ? 2.05 : 1.42 + pulse * 0.42);
      const haloAlpha = hovered ? 0.34 : 0.13 + pulse * 0.18;
      context.beginPath();
      context.arc(point.x, point.y, haloRadius, 0, Math.PI * 2);
      context.fillStyle = hexToRgba(node.color, haloAlpha);
      context.fill();
    }
    context.beginPath();
    context.arc(point.x, point.y, radius * (hovered ? 1.12 : 1), 0, Math.PI * 2);
    context.fillStyle = node.color;
    context.fill();
    context.lineWidth = hovered ? 1.8 : 0.9 + pulse * 0.55;
    context.strokeStyle = hovered ? "rgba(248, 245, 238, 0.92)" : "rgba(248, 245, 238, 0.68)";
    context.stroke();
    if (node.typeIcon && radius >= 7) {
      context.font = `${clamp(radius * 0.92, 8, 15)}px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif`;
      context.textAlign = "center";
      context.textBaseline = "middle";
      context.fillStyle = "rgba(248, 245, 238, 0.92)";
      context.fillText(node.typeIcon.slice(0, 2), point.x, point.y + 0.5, radius * 1.5);
    }
  }

  context.textAlign = "center";
  context.textBaseline = "top";
  for (const node of layout.nodes) {
    if (!node.showLabel && hoveredNodeId !== node.id) continue;
    const point = knowledgeGraphToCanvasPoint(node, safeCamera);
    const radius = Math.max(3.4, node.radius * safeCamera.scale);
    const labelFontSize = knowledgeGraphLabelFontSize(radius, hoveredNodeId === node.id);
    const labelTop = point.y + radius + Math.max(4, labelFontSize * 0.42);
    const label = ellipsizeCanvasLabel(node.label, hoveredNodeId === node.id ? 22 : 14);
    context.font = `${labelFontSize}px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif`;
    context.lineWidth = Math.max(2, labelFontSize * 0.28);
    context.strokeStyle = "rgba(31, 29, 26, 0.72)";
    context.strokeText(label, point.x, labelTop);
    context.fillStyle = "rgba(236, 229, 219, 0.86)";
    context.fillText(label, point.x, labelTop);
  }
  context.restore();
}

function knowledgeGraphLabelFontSize(nodePixelRadius: number, hovered: boolean) {
  const base = nodePixelRadius * 0.72 + 3.2;
  return clamp(hovered ? base + 1.2 : base, 7, 13.5);
}

function hexToRgba(hex: string, alpha: number) {
  const normalized = hex.replace("#", "");
  const value = Number.parseInt(normalized, 16);
  if (!Number.isFinite(value)) return `rgba(220, 125, 87, ${alpha})`;
  const red = (value >> 16) & 255;
  const green = (value >> 8) & 255;
  const blue = value & 255;
  return `rgba(${red}, ${green}, ${blue}, ${alpha})`;
}

function ellipsizeCanvasLabel(label: string, maxLength: number) {
  if (label.length <= maxLength) return label;
  return `${label.slice(0, Math.max(1, maxLength - 1))}…`;
}

function knowledgeGraphRelationKind(kind: string) {
  if (kind.startsWith("frontmatter:")) return "frontmatter";
  if (kind === "wikilink") return "wikilink";
  return "contains";
}

function knowledgeGraphRelationLabel(kind: string) {
  const normalized = kind.replace(/^frontmatter:/, "").trim();
  const labels: Record<string, string> = {
    abstracted_from_draft: "从稿件抽象",
    adopts_knowledge: "已采纳",
    appears_in: "出现于",
    belongs_to: "归属",
    contains: "包含",
    derived_from_knowledge: "已沉淀",
    distilled_from_memory: "从记忆蒸馏",
    derived_from: "提炼自",
    evidence: "依据材料",
    excerpted_from_project: "从作品摘录",
    extracts_to: "提炼为",
    quotes: "引用摘录",
    source: "素材来源",
    conflicts_with: "冲突对象",
    冲突对象: "冲突对象",
    冲突卡片: "冲突对象",
    uncertainty: "不确定性",
    不确定性: "不确定性",
    references_knowledge: "引用知识",
    related_to: "关联",
    source_ref: "素材出处",
    source_refs: "素材出处",
    supports: "支撑",
    used_by_projects: "被作品使用",
    uses_elements: "使用元素",
    wikilink: "正文链接",
  };
  const fallback = /[\u4e00-\u9fff]/.test(normalized) ? normalized : "扩展关系";
  return (labels[normalized] ?? fallback).slice(0, 22);
}

function knowledgeGraphNodeKindLabel(kind: string) {
  const normalized = kind.toLowerCase();
  const labels: Record<string, string> = {
    analysis: "拆解报告",
    card: "知识卡",
    concept: "概念",
    entity: "实体",
    folder: "分类",
    knowledge_card: "知识卡",
    "knowledge-card": "知识卡",
    knowledge_concept: "概念卡",
    knowledge_entity: "实体卡",
    knowledge_source: "来源资料",
    method: "方法卡",
    note: "普通笔记",
    skill: "技能产物",
    skill_output: "技能产物",
    "skill-output": "技能产物",
    source: "来源资料",
    "type-definition": "类型定义",
  };
  return labels[normalized] ?? kind;
}

function knowledgeGraphEdgeColor(kind: string) {
  if (kind === "frontmatter") return "rgba(220, 125, 87, 0.9)";
  if (kind === "wikilink") return "rgba(220, 125, 87, 0.72)";
  return "rgba(138, 129, 118, 0.64)";
}

function knowledgeGraphEdgeWidth(kind: string) {
  if (kind === "frontmatter") return 1.85;
  if (kind === "wikilink") return 1.35;
  return 1.1;
}

function knowledgeGraphEdgeDash(kind: string) {
  if (kind === "frontmatter") return [];
  if (kind === "wikilink") return [5, 4];
  return [4, 5];
}

function drawKnowledgeGraphEdgeLabel(
  context: CanvasRenderingContext2D,
  source: { x: number; y: number },
  target: { x: number; y: number },
  label: string,
) {
  if (!label) return;
  const midX = (source.x + target.x) / 2;
  const midY = (source.y + target.y) / 2;
  context.save();
  context.font = "10px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif";
  const width = Math.min(122, Math.max(34, context.measureText(label).width + 10));
  context.fillStyle = "rgba(36, 32, 28, 0.76)";
  context.strokeStyle = "rgba(220, 125, 87, 0.5)";
  context.lineWidth = 1;
  roundRectPath(context, midX - width / 2, midY - 8, width, 16, 6);
  context.fill();
  context.stroke();
  context.textAlign = "center";
  context.textBaseline = "middle";
  context.fillStyle = "rgba(248, 238, 229, 0.88)";
  context.fillText(label, midX, midY + 0.5, width - 8);
  context.restore();
}

function roundRectPath(context: CanvasRenderingContext2D, x: number, y: number, width: number, height: number, radius: number) {
  const safeRadius = Math.min(radius, width / 2, height / 2);
  context.beginPath();
  context.moveTo(x + safeRadius, y);
  context.lineTo(x + width - safeRadius, y);
  context.quadraticCurveTo(x + width, y, x + width, y + safeRadius);
  context.lineTo(x + width, y + height - safeRadius);
  context.quadraticCurveTo(x + width, y + height, x + width - safeRadius, y + height);
  context.lineTo(x + safeRadius, y + height);
  context.quadraticCurveTo(x, y + height, x, y + height - safeRadius);
  context.lineTo(x, y + safeRadius);
  context.quadraticCurveTo(x, y, x + safeRadius, y);
}

function stableNumber(value: string) {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash);
}
