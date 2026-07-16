import { Cpu } from "lucide-react";

import type { MeshNode, NodeControlKey } from "../types/mesh";
import { ModuleCard } from "./ModuleCard";

interface ModuleGridProps {
  nodes: MeshNode[];
  onRemove: (id: number) => void;
  onUpdate: (id: number, key: NodeControlKey, value: number | boolean) => void;
}

export function ModuleGrid({ nodes, onRemove, onUpdate }: ModuleGridProps) {
  return (
    <>
      <div className="module-heading">
        <div><p>YOUR SETUP</p><h1>Modules</h1></div>
        <span>{nodes.length === 1 ? "1 module" : `${nodes.length} modules`}</span>
      </div>
      <div className="module-grid">
        {nodes.length === 0 ? (
          <div className="empty-state">
            <Cpu size={24} aria-hidden="true" />
            <strong>No modules running</strong>
            <span>Press Start to hear the set.</span>
          </div>
        ) : nodes.map((node) => (
          <ModuleCard key={node.id} node={node} canRemove={nodes.length > 1} onRemove={onRemove} onUpdate={onUpdate} />
        ))}
      </div>
    </>
  );
}
