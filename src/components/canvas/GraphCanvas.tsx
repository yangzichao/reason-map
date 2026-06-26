// The argument canvas (React Flow). Local node state gives smooth dragging; positions
// persist on drag stop. Focus-mode dims everything outside the selection's neighborhood.
// Connecting two nodes creates a support edge (SPEC §7.4). Double-click empty space (or the
// ＋ button) adds a claim; Delete/Backspace removes the selection.

import { useCallback, useEffect, useMemo, useRef } from "react";
import {
  ReactFlow,
  Background,
  Controls,
  MiniMap,
  useNodesState,
  useEdgesState,
  type Edge,
  type Node,
  type Connection,
  type ReactFlowInstance,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import ClaimNodeView, { type ClaimNodeData } from "./ClaimNode";
import { neighborhood } from "./layout";
import { EDGE_META, STATUS_META } from "@/types/domain";
import { useStore } from "@/state/store";

const nodeTypes = { claim: ClaimNodeView };

export default function GraphCanvas() {
  const graph = useStore((s) => s.graph);
  const criticality = useStore((s) => s.criticality);
  const circular = useStore((s) => s.circular);
  const selectedNodeIds = useStore((s) => s.selectedNodeIds);
  const selectedEdgeId = useStore((s) => s.selectedEdgeId);
  const focusMode = useStore((s) => s.focusMode);
  const selectNode = useStore((s) => s.selectNode);
  const selectEdge = useStore((s) => s.selectEdge);
  const clearSelection = useStore((s) => s.clearSelection);
  const addEdge = useStore((s) => s.addEdge);
  const addNode = useStore((s) => s.addNode);
  const removeNode = useStore((s) => s.removeNode);
  const removeEdge = useStore((s) => s.removeEdge);
  const persistNodePosition = useStore((s) => s.persistNodePosition);

  const instanceRef = useRef<ReactFlowInstance<Node<ClaimNodeData>, Edge> | null>(null);

  const [rfNodes, setRfNodes, onNodesChange] = useNodesState<Node<ClaimNodeData>>([]);
  const [rfEdges, setRfEdges, onEdgesChange] = useEdgesState<Edge>([]);

  // Add a claim at a flow position (or near the viewport center when none is given),
  // then select it so the user can immediately type.
  const addClaimAt = useCallback(
    async (flow?: { x: number; y: number }) => {
      const inst = instanceRef.current;
      const pos = flow ?? (inst ? inst.screenToFlowPosition({ x: window.innerWidth / 2, y: 260 }) : { x: 240, y: 160 });
      const id = await addNode("新命题", "open", pos.x, pos.y);
      if (id) selectNode(id, false);
    },
    [addNode, selectNode],
  );

  const onPaneDoubleClick = useCallback(
    (e: React.MouseEvent) => {
      const target = e.target as HTMLElement;
      if (!target.classList.contains("react-flow__pane")) return;
      const inst = instanceRef.current;
      if (!inst) return;
      void addClaimAt(inst.screenToFlowPosition({ x: e.clientX, y: e.clientY }));
    },
    [addClaimAt],
  );

  const circularSet = useMemo(() => new Set(circular), [circular]);
  const visible = useMemo(() => {
    if (!focusMode || selectedNodeIds.length === 0 || !graph) return null;
    return neighborhood(selectedNodeIds, graph.edges);
  }, [focusMode, selectedNodeIds, graph]);

  // Rebuild nodes from the source of truth whenever the graph/derived data/selection change.
  useEffect(() => {
    if (!graph) {
      setRfNodes([]);
      return;
    }
    setRfNodes(
      graph.nodes.map((n) => ({
        id: n.id,
        type: "claim",
        position: { x: n.x, y: n.y },
        selected: selectedNodeIds.includes(n.id),
        data: {
          node: n,
          criticality: criticality[n.id],
          circular: circularSet.has(n.id),
          dimmed: visible ? !visible.has(n.id) : false,
        },
      })),
    );
  }, [graph, criticality, circularSet, selectedNodeIds, visible, setRfNodes]);

  useEffect(() => {
    if (!graph) {
      setRfEdges([]);
      return;
    }
    setRfEdges(
      graph.edges.map((e) => {
        const meta = EDGE_META[e.edgeType];
        const dim = visible ? !(visible.has(e.fromNode) && visible.has(e.toNode)) : false;
        return {
          id: e.id,
          source: e.fromNode,
          target: e.toNode,
          label: meta.label,
          animated: e.edgeType === "rebut",
          selected: e.id === selectedEdgeId,
          style: {
            stroke: meta.color,
            strokeWidth: e.id === selectedEdgeId ? 3 : 1.5,
            opacity: dim ? 0.12 : 1,
            strokeDasharray: e.edgeType === "rebut" ? "6 4" : undefined,
          },
          labelStyle: { fill: meta.color, fontSize: 11, opacity: dim ? 0.2 : 1 },
          labelBgStyle: { fill: "#f6f8f1", opacity: dim ? 0.2 : 0.92 },
        } as Edge;
      }),
    );
  }, [graph, selectedEdgeId, visible, setRfEdges]);

  if (!graph) return <div className="canvas-empty">加载中…</div>;

  return (
    <div className="canvas-wrap" onDoubleClick={onPaneDoubleClick}>
      <ReactFlow
        nodes={rfNodes}
        edges={rfEdges}
        nodeTypes={nodeTypes}
        onInit={(inst) => (instanceRef.current = inst)}
        onNodesChange={onNodesChange}
        onEdgesChange={onEdgesChange}
        onNodeDragStop={(_, n) => void persistNodePosition(n.id, n.position.x, n.position.y)}
        onConnect={(c: Connection) => {
          if (c.source && c.target) void addEdge(c.source, c.target);
        }}
        onNodeClick={(e, n) => selectNode(n.id, e.shiftKey)}
        onEdgeClick={(_, ed) => selectEdge(ed.id)}
        onPaneClick={() => clearSelection()}
        onNodesDelete={(ns) => ns.forEach((n) => void removeNode(n.id))}
        onEdgesDelete={(es) => es.forEach((ed) => void removeEdge(ed.id))}
        deleteKeyCode={["Delete", "Backspace"]}
        fitView
        proOptions={{ hideAttribution: true }}
        minZoom={0.2}
        maxZoom={2}
      >
        <Background color="#cdd4c2" gap={20} />
        <Controls showInteractive={false} />
        <MiniMap
          pannable
          zoomable
          nodeColor={(n) => {
            const d = n.data as ClaimNodeData | undefined;
            return d ? STATUS_META[d.node.status].color : "#b3bca6";
          }}
          maskColor="rgba(35,42,34,0.08)"
        />
      </ReactFlow>
      <button className="canvas-add-node" title="添加命题(或双击空白处)" onClick={() => void addClaimAt()}>
        ＋ 命题
      </button>
    </div>
  );
}
