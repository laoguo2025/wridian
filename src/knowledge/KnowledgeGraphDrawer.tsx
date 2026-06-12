import {
  type MouseEvent as ReactMouseEvent,
  type PointerEvent as ReactPointerEvent,
  type WheelEvent as ReactWheelEvent,
  useEffect,
  useMemo,
  useRef,
  useState,
} from "react";
import { invoke } from "@tauri-apps/api/core";
import {
  forceCenter,
  forceCollide,
  forceLink,
  forceManyBody,
  forceSimulation,
  type SimulationLinkDatum,
  type SimulationNodeDatum,
} from "d3-force";
import type {
  KnowledgeGraphNode,
  KnowledgeGraphState,
  KnowledgeHealthFixResponse,
  KnowledgeHealthWorkflowResponse,
  KnowledgeSearchHit,
} from "../appTypes";
import { clamp } from "../numberUtils";

export function KnowledgeGraphDrawer({
  graph,
  graphError,
  healthResult,
  knowledgeRootConfigured,
  onClose,
  onOpenFile,
  onRefresh,
  onHealthResult,
}: {
  graph: KnowledgeGraphState;
  graphError: string;
  healthResult: KnowledgeHealthWorkflowResponse | KnowledgeHealthFixResponse | null;
  knowledgeRootConfigured: boolean;
  onClose: () => void;
  onOpenFile: (path: string) => void;
  onRefresh: () => void | Promise<void>;
  onHealthResult: (result: KnowledgeHealthWorkflowResponse | KnowledgeHealthFixResponse | null) => void;
}) {
  const layout = useMemo(() => buildKnowledgeGraphLayout(graph), [graph]);
  const canvasRef = useRef<HTMLCanvasElement | null>(null);
  const graphAnimationFrameRef = useRef<number | null>(null);
  const [stageSize, setStageSize] = useState({ height: 520, width: 780 });
  const defaultCamera = useMemo(() => fitKnowledgeGraphCamera(layout.nodes, stageSize), [layout.nodes, stageSize]);
  const [camera, setCamera] = useState(defaultCamera);
  const [hoveredNode, setHoveredNode] = useState<KnowledgeGraphLayoutNode | null>(null);
  const [graphRenderTick, forceGraphRender] = useState(0);
  const [dragging, setDragging] = useState(false);
  const [opsBusy, setOpsBusy] = useState<"health" | "fix" | "search" | "">("");
  const [opsMessage, setOpsMessage] = useState("");
  const [searchQuery, setSearchQuery] = useState("");
  const [searchHits, setSearchHits] = useState<KnowledgeSearchHit[]>([]);
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
    if (!knowledgeRootConfigured || !graph.nodes.length) return;
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
  }, [graph.nodes.length, graphRenderTick, knowledgeRootConfigured, layout, safeCamera, hoveredNode?.id]);

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
    if (!graph.nodes.length) return;
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
    if (!graph.nodes.length || event.button !== 0) return;
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
      const margin = Math.max(260, dragState.node.collisionRadius * 3);
      dragState.node.x = clamp(
        dragState.startNodeX + graphPoint.x - dragState.startGraphX,
        -margin,
        KNOWLEDGE_GRAPH_WORLD_WIDTH + margin,
      );
      dragState.node.y = clamp(
        dragState.startNodeY + graphPoint.y - dragState.startGraphY,
        -margin,
        KNOWLEDGE_GRAPH_WORLD_HEIGHT + margin,
      );
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
    if (node.kind === "folder" || node.kind === "unresolved" || !node.path) return;
    onOpenFile(node.path);
  };

  const resetGraphView = () => {
    setCamera(fitKnowledgeGraphCamera(layout.nodes, stageSize));
  };

  const runKnowledgeHealthCheck = async () => {
    if (!knowledgeRootConfigured || opsBusy) return;
    setOpsBusy("health");
    setOpsMessage("知识库体检中，报告即将生成");
    try {
      const response = await invoke<KnowledgeHealthWorkflowResponse>("wridian_run_knowledge_health_check");
      onHealthResult(response);
      setOpsMessage(`体检完成：发现 ${response.issues.length} 个需要关注的问题，${response.pendingFixes.length} 项需要你判断。`);
      await onRefresh();
    } catch (error) {
      setOpsMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setOpsBusy("");
    }
  };

  const runLowRiskFixes = async () => {
    if (!knowledgeRootConfigured || opsBusy) return;
    setOpsBusy("fix");
    setOpsMessage("正在执行低风险修复");
    try {
      const response = await invoke<KnowledgeHealthFixResponse>("wridian_fix_knowledge_health_low_risk");
      onHealthResult(response);
      setOpsMessage(`修复完成：已处理 ${response.appliedFixes.length} 项低风险问题，报告已更新。`);
      await onRefresh();
    } catch (error) {
      setOpsMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setOpsBusy("");
    }
  };

  const runKnowledgeSearch = async () => {
    if (!knowledgeRootConfigured || opsBusy) return;
    const query = searchQuery.trim();
    if (!query) {
      setSearchHits([]);
      return;
    }
    setOpsBusy("search");
    setOpsMessage("");
    try {
      const response = await invoke<KnowledgeSearchHit[]>("wridian_search_knowledge_bm25", {
        input: { query, limit: 8 },
      });
      setSearchHits(response);
      setOpsMessage(response.length ? `命中 ${response.length} 条知识卡。` : "没有命中知识卡。");
    } catch (error) {
      setOpsMessage(error instanceof Error ? error.message : String(error));
    } finally {
      setOpsBusy("");
    }
  };
  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer knowledge-graph-drawer" role="dialog" aria-modal="true" aria-label="知识图谱" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">知识图谱</div>
          </div>
          <div className="drawer-header-actions">
            <button type="button" className="small-action" onClick={() => void runKnowledgeHealthCheck()} disabled={!knowledgeRootConfigured || Boolean(opsBusy)}>
              {opsBusy === "health" ? "体检中" : "体检"}
            </button>
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

        {graphError ? <div className="rail-error">{graphError}</div> : null}
        {graph.warnings.length ? (
          <div className="rail-warning">
            {graph.warnings.slice(0, 3).map((warning) => (
              <div key={warning}>{warning}</div>
            ))}
            {graph.warnings.length > 3 ? <div>还有 {graph.warnings.length - 3} 条图谱提示。</div> : null}
          </div>
        ) : null}

        <div className="knowledge-ops-panel">
          <form
            className="knowledge-search-form"
            onSubmit={(event) => {
              event.preventDefault();
              void runKnowledgeSearch();
            }}
          >
            <input
              value={searchQuery}
              onChange={(event) => setSearchQuery(event.target.value)}
              placeholder="搜索知识卡"
              disabled={!knowledgeRootConfigured || Boolean(opsBusy)}
            />
            <button type="submit" className="small-action" disabled={!knowledgeRootConfigured || Boolean(opsBusy)}>
              搜索
            </button>
          </form>
          {opsMessage ? <div className="knowledge-ops-message">{opsMessage}</div> : null}
          <div className="knowledge-ops-help">
            图谱只展示知识卡、分类和引用关系；体检会给出可自动处理、需要判断和仅提醒的问题。
          </div>
          {searchHits.length ? (
            <div className="knowledge-search-results">
              {searchHits.map((hit) => (
                <button key={hit.path} type="button" onClick={() => onOpenFile(hit.path)}>
                  <span className="knowledge-search-result-title">{hit.title}</span>
                  <span className="knowledge-search-result-meta">
                    {hit.relativePath} · {hit.score.toFixed(2)}
                  </span>
                  <span className="knowledge-search-result-snippet">{hit.snippet || hit.reasons.join(" / ")}</span>
                </button>
              ))}
            </div>
          ) : null}
        </div>

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
          {knowledgeRootConfigured && graph.nodes.length ? (
            <div className="knowledge-graph-legend" aria-hidden="true">
              <span><i className="folder" />分类</span>
              <span><i className="card" />知识卡</span>
              <span><i className="unresolved" />断链</span>
              <span><b className="solid" />关系</span>
              <span><b className="dashed" />引用</span>
            </div>
          ) : null}
          {knowledgeRootConfigured && graph.nodes.length ? (
            <div className="knowledge-graph-stats" aria-hidden="true">
              <span>{layout.nodes.length} 节点</span>
              <span>{layout.edges.length} 关系</span>
              {graph.nodes.length > layout.nodes.length ? <span>已显示前 {layout.nodes.length} 个</span> : null}
            </div>
          ) : null}
          {!knowledgeRootConfigured ? (
            <div className="knowledge-graph-empty">先选择知识库文件夹</div>
          ) : graph.nodes.length ? (
            <canvas
              ref={canvasRef}
              className="knowledge-graph-canvas"
              aria-label="知识库动态图谱"
              style={{ cursor: dragging ? "grabbing" : hoveredNode ? "pointer" : "grab" }}
            />
          ) : (
            <div className="knowledge-graph-empty">知识库里还没有 Markdown 知识卡</div>
          )}
          {hoveredNode ? (
            <div className="knowledge-graph-preview">
              <div className="knowledge-graph-preview-title">{hoveredNode.label}</div>
              <div className="knowledge-graph-preview-path">{knowledgeGraphDisplayPath(hoveredNode)}</div>
              {hoveredNode.kind === "folder" ? (
                <div className="knowledge-graph-preview-body">分类文件夹</div>
              ) : hoveredNode.kind === "unresolved" ? (
                <KnowledgeGraphMetadataPreview node={hoveredNode} />
              ) : (
                <KnowledgeGraphMetadataPreview node={hoveredNode} />
              )}
            </div>
          ) : null}
          {opsBusy === "health" || opsBusy === "fix" ? (
            <div className="knowledge-health-scan-overlay" aria-live="polite">
              <div className="knowledge-health-scan-radar" />
              <div>{opsBusy === "fix" ? "知识库修复中，报告即将更新" : "知识库体检中，报告即将生成"}</div>
            </div>
          ) : null}
          {healthResult && !opsBusy ? (
            <KnowledgeHealthResultPanel
              result={healthResult}
              onFix={() => void runLowRiskFixes()}
              onOpenReport={() => onOpenFile(healthResult.reportPath)}
            />
          ) : null}
        </div>
      </aside>
    </div>
  );
}

function KnowledgeGraphMetadataPreview({ node }: { node: KnowledgeGraphLayoutNode }) {
  const metrics = [
    node.outgoingCount ? `出链 ${node.outgoingCount}` : "",
    node.backlinkCount ? `反链 ${node.backlinkCount}` : "",
    node.unresolvedCount ? `断链 ${node.unresolvedCount}` : "",
  ].filter(Boolean);
  const chips = [
    ...(node.aliases ?? []).map((value) => `别名:${value}`),
    ...(node.tags ?? []).map((value) => `#${value}`),
    ...(node.sourceRefs ?? []).map((value) => `来源:${value}`),
  ];
  const backlinkSources = node.backlinkSources ?? [];
  return (
    <div className="knowledge-graph-preview-meta">
      {metrics.length ? <div className="knowledge-graph-preview-metrics">{metrics.join(" / ")}</div> : null}
      {chips.length ? (
        <div className="knowledge-graph-preview-chips">
          {chips.slice(0, 8).map((chip) => (
            <span key={chip}>{chip}</span>
          ))}
        </div>
      ) : null}
      {backlinkSources.length ? (
        <div className="knowledge-graph-preview-links">被引用：{backlinkSources.join("、")}</div>
      ) : null}
    </div>
  );
}

function knowledgeGraphDisplayPath(node: KnowledgeGraphLayoutNode) {
  return node.relativePath || node.group || node.label;
}

function KnowledgeHealthResultPanel({
  onFix,
  onOpenReport,
  result,
}: {
  onFix: () => void;
  onOpenReport: () => void;
  result: KnowledgeHealthWorkflowResponse | KnowledgeHealthFixResponse;
}) {
  const appliedCount = "appliedFixes" in result ? result.appliedFixes.length : 0;
  const stopStagePointer = (event: ReactPointerEvent | ReactMouseEvent) => {
    event.stopPropagation();
  };
  const issueTagCount = [
    result.summary.unresolvedLinkCount,
    result.summary.orphanFileCount,
    Math.max(0, result.summary.fileCount - result.summary.sourceCoverageCount),
    result.pendingFixes.length,
  ].filter((count) => count > 0).length;
  return (
    <div
      className="knowledge-health-result-panel"
      onClick={stopStagePointer}
      onPointerDown={stopStagePointer}
      onPointerMove={stopStagePointer}
      onPointerUp={stopStagePointer}
    >
      <div className="knowledge-health-result-header">
        <div>
          <div className="knowledge-health-result-title">体检完成</div>
          <div className="knowledge-health-result-meta">报告：{result.reportRelativePath}</div>
        </div>
        <strong>{result.score}</strong>
      </div>
      <div className="knowledge-health-result-stats">
        <span>断链 {result.summary.unresolvedLinkCount}</span>
        <span>孤岛 {result.summary.orphanFileCount}</span>
        <span>缺来源 {Math.max(0, result.summary.fileCount - result.summary.sourceCoverageCount)}</span>
        <span>需判断 {result.pendingFixes.length}</span>
      </div>
      <div className="knowledge-health-result-note">
        可自动处理的低风险项会写入报告；需要判断的整理建议不会自动改文件。
        {appliedCount ? ` 本次已处理 ${appliedCount} 项。` : ` 本次发现 ${result.issues.length} 个问题，覆盖 ${issueTagCount} 类。`}
      </div>
      <div className="knowledge-health-result-actions">
        <button type="button" className="small-action" onClick={onOpenReport}>
          打开报告
        </button>
        <button type="button" className="small-action" onClick={onFix} disabled={!result.pendingFixes.length && !result.issues.length}>
          处理低风险项
        </button>
      </div>
    </div>
  );
}

type KnowledgeGraphLayoutNode = KnowledgeGraphNode & {
  color: string;
  collisionRadius: number;
  depth: number;
  fx?: number | null;
  fy?: number | null;
  radius: number;
  showLabel: boolean;
  vx?: number;
  vy?: number;
  x: number;
  y: number;
} & SimulationNodeDatum;

type KnowledgeGraphLayoutEdge = SimulationLinkDatum<KnowledgeGraphLayoutNode> & {
  kind: string;
  source: KnowledgeGraphLayoutNode;
  target: KnowledgeGraphLayoutNode;
};

type KnowledgeGraphCamera = {
  offsetX: number;
  offsetY: number;
  scale: number;
};

const KNOWLEDGE_GRAPH_WORLD_WIDTH = 1100;
const KNOWLEDGE_GRAPH_WORLD_HEIGHT = 640;
const KNOWLEDGE_GRAPH_CAMERA_MIN_SCALE = 0.05;
const KNOWLEDGE_GRAPH_CAMERA_MAX_SCALE = 5.2;
const MAX_KNOWLEDGE_GRAPH_NODES = 1000;
const MAX_KNOWLEDGE_GRAPH_EDGES = 2400;
const DEFAULT_KNOWLEDGE_GRAPH_FIT_RATIO = 0.86;

function buildKnowledgeGraphLayout(graph: KnowledgeGraphState) {
  const limited = graph.nodes.slice(0, MAX_KNOWLEDGE_GRAPH_NODES);
  const depthBuckets = new Map<number, KnowledgeGraphNode[]>();
  for (const node of limited) {
    const depth = knowledgeGraphNodeDepth(node);
    const bucket = depthBuckets.get(depth);
    if (bucket) {
      bucket.push(node);
    } else {
      depthBuckets.set(depth, [node]);
    }
  }
  const siblingPositionById = new Map<string, { index: number; count: number }>();
  for (const siblings of depthBuckets.values()) {
    siblings.forEach((node, index) => {
      siblingPositionById.set(node.id, { index, count: Math.max(1, siblings.length) });
    });
  }
  const nodes = limited.map((node, index): KnowledgeGraphLayoutNode => {
    const depth = knowledgeGraphNodeDepth(node);
    const siblingPosition = siblingPositionById.get(node.id) ?? { index: 0, count: 1 };
    const groupHash = stableNumber(`${node.group}:${node.id}`);
    const depthRing = 120 + Math.min(depth, 7) * 58 + (node.kind === "folder" ? 0 : 28);
    const angle = ((siblingPosition.index / siblingPosition.count) * 360 + (depth % 2) * 23 + (groupHash % 22)) * (Math.PI / 180);
    const radius = knowledgeGraphNodeRadius(node);
    const showLabel = node.kind === "folder" || index < 80 || (node.backlinkCount ?? 0) + (node.outgoingCount ?? 0) >= 4;
    const labelRadius = showLabel ? Math.min(92, Math.max(22, node.label.length * 4.2)) : 0;
    return {
      ...node,
      collisionRadius: radius + labelRadius * 0.42 + 8,
      color: knowledgeGraphTypedNodeColor(node, depth),
      depth,
      radius,
      showLabel,
      vx: 0,
      vy: 0,
      x: KNOWLEDGE_GRAPH_WORLD_WIDTH / 2 + Math.cos(angle) * depthRing,
      y: KNOWLEDGE_GRAPH_WORLD_HEIGHT / 2 + Math.sin(angle) * depthRing,
    };
  });
  const byId = new Map(nodes.map((node) => [node.id, node]));
  const edges = graph.edges
    .map((edge) => ({
      kind: edge.kind,
      source: byId.get(edge.source),
      target: byId.get(edge.target),
    }))
    .filter((edge): edge is KnowledgeGraphLayoutEdge => Boolean(edge.source && edge.target))
    .slice(0, MAX_KNOWLEDGE_GRAPH_EDGES);
  relaxKnowledgeGraphLayout(nodes, edges);
  return { edges, nodes };
}

function knowledgeGraphNodeRadius(node: KnowledgeGraphNode) {
  if (node.kind === "folder") return Math.min(22, 13 + node.size * 0.18);
  if (node.kind === "unresolved") return 8.8;
  const relationScore = (node.backlinkCount ?? 0) + (node.outgoingCount ?? 0) + (node.sourceRefs?.length ?? 0);
  return Math.min(13.4, 6.6 + Math.sqrt(Math.max(1, node.size + relationScore)) * 0.58);
}

function knowledgeGraphEdgeDistance(edge: KnowledgeGraphLayoutEdge) {
  const relationKind = knowledgeGraphRelationKind(edge.kind);
  if (relationKind === "frontmatter") return 126;
  if (relationKind === "unresolved") return 112;
  if (relationKind === "embed") return 104;
  if (relationKind === "wikilink") return 92;
  return 78 + Math.min(42, edge.target.depth * 7);
}

function knowledgeGraphEdgeStrength(edge: KnowledgeGraphLayoutEdge) {
  const relationKind = knowledgeGraphRelationKind(edge.kind);
  if (relationKind === "frontmatter") return 0.42;
  if (relationKind === "embed") return 0.3;
  if (relationKind === "wikilink") return 0.24;
  if (relationKind === "unresolved") return 0.18;
  return 0.32;
}

function relaxKnowledgeGraphLayout(
  nodes: KnowledgeGraphLayoutNode[],
  edges: KnowledgeGraphLayoutEdge[],
) {
  if (!nodes.length) return;
  const simulation = forceSimulation<KnowledgeGraphLayoutNode>(nodes)
    .force(
      "charge",
      forceManyBody<KnowledgeGraphLayoutNode>()
        .strength((node) => (node.kind === "folder" ? -420 : node.kind === "unresolved" ? -170 : -135))
        .distanceMax(520),
    )
    .force(
      "link",
      forceLink<KnowledgeGraphLayoutNode, KnowledgeGraphLayoutEdge>(edges)
        .id((node) => node.id)
        .distance((edge) => knowledgeGraphEdgeDistance(edge))
        .strength((edge) => knowledgeGraphEdgeStrength(edge)),
    )
    .force("center", forceCenter(KNOWLEDGE_GRAPH_WORLD_WIDTH / 2, KNOWLEDGE_GRAPH_WORLD_HEIGHT / 2).strength(0.045))
    .force("collide", forceCollide<KnowledgeGraphLayoutNode>().radius((node) => node.collisionRadius).iterations(2))
    .stop();
  const ticks = nodes.length > 650 ? 150 : nodes.length > 320 ? 190 : 230;
  simulation.tick(ticks);
  simulation.stop();
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
  if (node.kind === "folder") return knowledgeGraphNodeColor(depth);
  if (node.kind === "unresolved") return "#9d3d35";
  const normalizedKind = node.kind.toLowerCase();
  if (normalizedKind.includes("source")) return "#5f8f7b";
  if (normalizedKind.includes("analysis") || normalizedKind.includes("report")) return "#8d7cc3";
  if (normalizedKind.includes("skill")) return "#c69348";
  if (normalizedKind.includes("method") || normalizedKind.includes("knowledge")) return "#dc7d57";
  if (normalizedKind.includes("concept") || normalizedKind.includes("entity")) return "#5f79b8";
  return knowledgeGraphNodeColor(depth);
}

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
  const scale = clamp(fittedScale * DEFAULT_KNOWLEDGE_GRAPH_FIT_RATIO, KNOWLEDGE_GRAPH_CAMERA_MIN_SCALE, 1.8);
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
  fallback: KnowledgeGraphCamera = { offsetX: 390, offsetY: 260, scale: 0.72 },
): KnowledgeGraphCamera {
  const fallbackScale = Number.isFinite(fallback.scale) && fallback.scale > 0 ? fallback.scale : 0.72;
  const fallbackOffsetX = Number.isFinite(fallback.offsetX) ? fallback.offsetX : 390;
  const fallbackOffsetY = Number.isFinite(fallback.offsetY) ? fallback.offsetY : 260;
  const scale = Number.isFinite(camera.scale) && camera.scale > 0 ? clamp(camera.scale, KNOWLEDGE_GRAPH_CAMERA_MIN_SCALE, KNOWLEDGE_GRAPH_CAMERA_MAX_SCALE) : fallbackScale;
  const offsetX = Number.isFinite(camera.offsetX) ? clamp(camera.offsetX, -12000, 12000) : fallbackOffsetX;
  const offsetY = Number.isFinite(camera.offsetY) ? clamp(camera.offsetY, -12000, 12000) : fallbackOffsetY;
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
  const scale = clamp(base.scale * safeFactor, KNOWLEDGE_GRAPH_CAMERA_MIN_SCALE, KNOWLEDGE_GRAPH_CAMERA_MAX_SCALE);
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
    const radius = node.radius + 8;
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
    context.globalAlpha = knowledgeGraphEdgeAlpha(relationKind);
    context.lineDashOffset = relationKind === "frontmatter" ? 0 : edgeDashOffset;
    context.stroke();
    context.globalAlpha = 1;
    if (relationKind === "frontmatter") {
      drawKnowledgeGraphEdgeLabel(context, source, target, knowledgeGraphRelationLabel(edge.kind));
    }
  }
  context.setLineDash([]);
  context.lineDashOffset = 0;

  for (const [index, node] of layout.nodes.entries()) {
    const point = knowledgeGraphToCanvasPoint(node, safeCamera);
    const pulse = 0.5 + Math.sin(motion * 1.65 + index * 0.42) * 0.5;
    const radius = Math.max(node.kind === "folder" ? 5.2 : 3.1, node.radius * safeCamera.scale) + pulse * (node.kind === "folder" ? 0.5 : 0.35);
    const hovered = hoveredNodeId === node.id;
    if (node.kind === "folder" || hovered) {
      context.beginPath();
      context.arc(point.x, point.y, radius + (hovered ? 9 : 6 + pulse * 1.6), 0, Math.PI * 2);
      context.fillStyle = hexToRgba(node.color, hovered ? 0.2 : 0.09 + pulse * 0.04);
      context.fill();
    }
    context.beginPath();
    context.arc(point.x, point.y, radius + (hovered ? 1.4 : 0), 0, Math.PI * 2);
    context.fillStyle = node.kind === "folder" ? hexToRgba(node.color, 0.94) : hexToRgba(node.color, 0.78);
    context.fill();
    context.lineWidth = hovered ? 1.9 : node.kind === "folder" ? 1.4 : 0.8;
    context.strokeStyle = hovered ? "rgba(248, 245, 238, 0.92)" : node.kind === "folder" ? "rgba(248, 245, 238, 0.78)" : "rgba(248, 245, 238, 0.46)";
    context.stroke();
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
    context.lineWidth = Math.max(1, labelFontSize * 0.14);
    context.strokeStyle = "rgba(249, 246, 239, 0.72)";
    context.strokeText(label, point.x, labelTop);
    context.fillStyle = node.kind === "folder" ? "rgba(62, 55, 48, 0.9)" : "rgba(96, 86, 76, 0.78)";
    context.fillText(label, point.x, labelTop);
  }
  context.restore();
}

function knowledgeGraphLabelFontSize(nodePixelRadius: number, hovered: boolean) {
  const base = nodePixelRadius * 0.58 + 2.6;
  return clamp(hovered ? base + 0.8 : base, 6.5, 10.8);
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
  if (kind.includes("unresolved")) return "unresolved";
  if (kind === "embed" || kind.startsWith("embed:")) return "embed";
  if (kind.startsWith("frontmatter:")) return "frontmatter";
  if (kind === "wikilink") return "wikilink";
  return "contains";
}

function knowledgeGraphRelationLabel(kind: string) {
  return kind.replace(/^frontmatter:/, "").replace(/:unresolved$/, "").replace(/_/g, " ").slice(0, 22);
}

function knowledgeGraphEdgeColor(kind: string) {
  if (kind === "unresolved") return "rgba(157, 61, 53, 0.72)";
  if (kind === "embed") return "rgba(95, 143, 123, 0.68)";
  if (kind === "frontmatter") return "rgba(160, 91, 66, 0.82)";
  if (kind === "wikilink") return "rgba(116, 107, 94, 0.58)";
  return "rgba(116, 107, 94, 0.34)";
}

function knowledgeGraphEdgeWidth(kind: string) {
  if (kind === "unresolved") return 1.55;
  if (kind === "embed") return 1.45;
  if (kind === "frontmatter") return 2.15;
  if (kind === "wikilink") return 1.2;
  return 0.9;
}

function knowledgeGraphEdgeAlpha(kind: string) {
  if (kind === "unresolved") return 0.86;
  if (kind === "embed") return 0.74;
  if (kind === "frontmatter") return 0.92;
  if (kind === "wikilink") return 0.72;
  return 0.54;
}

function knowledgeGraphEdgeDash(kind: string) {
  if (kind === "unresolved") return [2, 5];
  if (kind === "embed") return [7, 3, 2, 3];
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
