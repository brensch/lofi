import { Boxes, Pause, Play, Plus } from "lucide-react";

import type { RuntimeState } from "../hooks/useMeshAudio";

interface AppHeaderProps {
  canAdd: boolean;
  onAdd: () => void;
  onToggle: () => void;
  runtime: RuntimeState;
}

const STATUS_LABEL: Record<RuntimeState, string> = {
  error: "Could not start",
  offline: "Ready to start",
  paused: "Music paused",
  running: "Playing",
  starting: "Starting modules",
};

export function AppHeader({ canAdd, onAdd, onToggle, runtime }: AppHeaderProps) {
  const running = runtime === "running";
  return (
    <header className="topbar">
      <div className="brand">
        <span className="brand-mark" aria-hidden="true"><i /><i /><i /></span>
        <strong>LOFI MESH</strong>
        <span>Mesh lab</span>
      </div>

      <div className={`transport-status ${running ? "running" : runtime === "error" ? "error" : ""}`} role="status">
        <span className="status-dot" />
        <span>{STATUS_LABEL[runtime]}</span>
      </div>

      <div className="header-actions">
        <button className="secondary-button" type="button" onClick={onAdd} disabled={!canAdd}>
          <Plus size={16} aria-hidden="true" /> Add module
        </button>
        <button className="primary-button" type="button" onClick={onToggle} disabled={runtime === "starting"}>
          {running ? <Pause size={16} aria-hidden="true" /> : runtime === "offline" ? <Boxes size={16} aria-hidden="true" /> : <Play size={16} aria-hidden="true" />}
          {running ? "Pause" : runtime === "offline" ? "Start" : "Resume"}
        </button>
      </div>
    </header>
  );
}
