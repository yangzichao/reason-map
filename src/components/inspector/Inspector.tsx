// Right-rail "详情" tab: shows the edge inspector when an edge is selected, otherwise the
// node inspector. Keeps the two focused components separate (SPEC: small files).

import { useStore } from "@/state/store";
import NodeInspector from "./NodeInspector";
import EdgeInspector from "./EdgeInspector";

export default function Inspector() {
  const selectedEdgeId = useStore((s) => s.selectedEdgeId);
  return selectedEdgeId ? <EdgeInspector /> : <NodeInspector />;
}
