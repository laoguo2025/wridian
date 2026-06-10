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
  const layout = useMemo(() => buildKnowledgeGraphLayout(graph), [graph]);
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

  return (
    <div className="drawer-backdrop" onMouseDown={onClose} role="presentation">
      <aside className="memory-drawer knowledge-graph-drawer" role="dialog" aria-modal="true" aria-label="知识图谱" onMouseDown={(event) => event.stopPropagation()}>
        <div className="drawer-header">
          <div>
            <div className="drawer-title">知识图谱</div>
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
          ) : graph.nodes.length ? (
            <canvas ref={canvasRef} className="knowledge-graph-canvas" aria-label="知识库动态图谱" />
          ) : (
            <div className="knowledge-graph-empty">知识库里还没有 Markdown 知识卡</div>
          )}
          {hoveredNode ? (
            <div className="knowledge-graph-preview">
              <div className="knowledge-graph-preview-title">{hoveredNode.label}</div>
              <div className="knowledge-graph-preview-path">{hoveredNode.path ?? hoveredNode.group}</div>
              {hoveredNode.kind === "folder" ? (
                <div className="knowledge-graph-preview-body">分类文件夹</div>
              ) : activePreview?.content ? (
                <div className="knowledge-graph-preview-body">{activePreview.content}</div>
              ) : activePreview?.error ? (
                <div className="knowledge-graph-preview-body">{activePreview.error}</div>
              ) : (
                <div className="knowledge-graph-preview-body">读取中...</div>
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
  const scale = clamp(Math.min((width - 52) / graphWidth, (height - 52) / graphHeight), 3.2, 12);
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
    context.beginPath();
    context.moveTo(source.x, source.y);
    context.lineTo(target.x, target.y);
    context.strokeStyle = edge.kind === "wikilink" ? "rgba(220, 125, 87, 0.78)" : "rgba(138, 129, 118, 0.72)";
    context.lineWidth = edge.kind === "wikilink" ? 1.45 : 1.2;
    context.setLineDash(edge.kind === "wikilink" ? [5, 4] : [4, 4]);
    context.lineDashOffset = edgeDashOffset;
    context.stroke();
  }
  context.setLineDash([]);
  context.lineDashOffset = 0;

  for (const [index, node] of layout.nodes.entries()) {
    const point = knowledgeGraphToCanvasPoint(node, safeCamera);
    const pulse = 0.5 + Math.sin(motion * 1.65 + index * 0.42) * 0.5;
    const radius = Math.max(3.4, node.radius * safeCamera.scale) + pulse * 0.7;
    const hovered = hoveredNodeId === node.id;
    if (node.kind !== "card" || hovered) {
      context.beginPath();
      context.arc(point.x, point.y, radius + (hovered ? 9 : 5 + pulse * 2), 0, Math.PI * 2);
      context.fillStyle = hexToRgba(node.color, hovered ? 0.24 : 0.1 + pulse * 0.05);
      context.fill();
    }
    context.beginPath();
    context.arc(point.x, point.y, radius + (hovered ? 1.4 : 0), 0, Math.PI * 2);
    context.fillStyle = node.color;
    context.fill();
    context.lineWidth = hovered ? 1.8 : 0.9 + pulse * 0.35;
    context.strokeStyle = hovered ? "rgba(248, 245, 238, 0.92)" : "rgba(248, 245, 238, 0.68)";
    context.stroke();
  }

  context.textAlign = "center";
  context.textBaseline = "top";
  context.font = "11px system-ui, -apple-system, BlinkMacSystemFont, Segoe UI, sans-serif";
  for (const node of layout.nodes) {
    if (!node.showLabel && hoveredNodeId !== node.id) continue;
    const point = knowledgeGraphToCanvasPoint(node, safeCamera);
    const radius = Math.max(3.4, node.radius * safeCamera.scale);
    const label = ellipsizeCanvasLabel(node.label, hoveredNodeId === node.id ? 22 : 14);
    context.lineWidth = 3;
    context.strokeStyle = "rgba(31, 29, 26, 0.72)";
    context.strokeText(label, point.x, point.y + radius + 5);
    context.fillStyle = "rgba(236, 229, 219, 0.86)";
    context.fillText(label, point.x, point.y + radius + 5);
  }
  context.restore();
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

function stableNumber(value: string) {
  let hash = 2166136261;
  for (const character of value) {
    hash ^= character.charCodeAt(0);
    hash = Math.imul(hash, 16777619);
  }
  return Math.abs(hash);
}
