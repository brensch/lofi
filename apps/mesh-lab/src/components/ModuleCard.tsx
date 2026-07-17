import { Headphones, Radio, Trash2, VolumeX } from "lucide-react";

import type { MeshNode, NodeControlKey } from "../types/mesh";

const ROLE_NAMES = ["Kick", "Drums", "Bass", "Chords", "Melody"];

interface ModuleCardProps {
  canRemove: boolean;
  node: MeshNode;
  onRemove: (id: number) => void;
  onUpdate: (id: number, key: NodeControlKey, value: number | boolean) => void;
}

export function ModuleCard({ canRemove, node, onRemove, onUpdate }: ModuleCardProps) {
  const activeSounds = ROLE_NAMES.filter((_, index) => (node.roleMask & (1 << index)) !== 0);
  const sounds = activeSounds.length === ROLE_NAMES.length ? "Full mix" : activeSounds.join(" + ");
  const panLabel = Math.abs(node.pan) < 0.04
    ? "C"
    : node.pan < 0 ? `L${Math.round(-node.pan * 100)}` : `R${Math.round(node.pan * 100)}`;
  const timing = node.isRoot
    ? "Reference"
    : !node.synced || node.dispersionUs >= 2_000_000_000
      ? "Connecting"
      : node.dispersionUs < 1_000 ? "Tight" : node.dispersionUs < 5_000 ? "Good" : "Loose";

  return (
    <article className={`module-card ${node.synced ? "synced" : ""}`}>
      <header className="module-card-header">
        <div><span className="module-led" /><strong>MODULE {String(node.id).padStart(2, "0")}</strong></div>
        <button className="icon-button" type="button" title="Remove module" aria-label={`Remove module ${node.id}`} disabled={!canRemove} onClick={() => onRemove(node.id)}>
          <Trash2 size={15} aria-hidden="true" />
        </button>
      </header>

      <div className="module-display">
        <Metric label="SOUND" value={sounds || ROLE_NAMES[node.role] || "--"} title={activeSounds.length ? activeSounds.join(", ") : "This module's parts in the music"} />
        <Metric label="GROUP" value={`#${node.rootId}`} title="Synced modules show the same group number" />
        <Metric label="LINKS" value={String(node.peers)} title="Other modules this one can reach" />
        <Metric label="SYNC" value={timing} title="How closely this module follows the group timing" />
        <div className="beat-track"><i style={{ width: `${node.beatPhase / 10}%` }} /></div>
      </div>

      <div className="module-controls">
        <label className="connection-switch"><span><Radio size={12} /> Sync with other modules</span>
          <input type="checkbox" checked={node.radio} onChange={(event) => onUpdate(node.id, "radio", event.target.checked)} />
        </label>
        <ModuleRange label="Pan" output={panLabel} min={-1} max={1} step={0.01} value={node.pan} onChange={(value) => onUpdate(node.id, "pan", value)} />
        <ModuleRange label="Level" output={`${Math.round(node.gain * 100)}%`} min={0} max={1.2} step={0.01} value={node.gain} onChange={(value) => onUpdate(node.id, "gain", value)} />
        <details className="module-testing">
          <summary>Timing test</summary>
          <ModuleRange label="Clock drift" output={`${Math.round(node.driftPpm)} ppm`} min={-200} max={200} step={1} value={node.driftPpm} onChange={(value) => onUpdate(node.id, "driftPpm", value)} />
        </details>
      </div>

      <footer className="module-actions">
        <button className={node.mute ? "active" : ""} type="button" onClick={() => onUpdate(node.id, "mute", !node.mute)}>
          <VolumeX size={13} aria-hidden="true" /> Mute
        </button>
        <button className={node.solo ? "active" : ""} type="button" onClick={() => onUpdate(node.id, "solo", !node.solo)}>
          <Headphones size={13} aria-hidden="true" /> Solo
        </button>
        <span className="sync-state">{node.isRoot ? "LEADER" : node.synced ? "IN SYNC" : "CONNECTING"}</span>
      </footer>
    </article>
  );
}

function Metric({ label, title, value }: { label: string; title: string; value: string }) {
  return <div title={title}><span>{label}</span><strong>{value}</strong></div>;
}

interface ModuleRangeProps {
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  output: string;
  step: number;
  value: number;
}

function ModuleRange({ label, max, min, onChange, output, step, value }: ModuleRangeProps) {
  return (
    <label className="wide-control">
      <span>{label}</span><output>{output}</output>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(event) => onChange(Number(event.target.value))} />
    </label>
  );
}
