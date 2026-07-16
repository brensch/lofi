import { Link2, Music2, SlidersHorizontal } from "lucide-react";

import type { NetworkControlKey, NetworkState } from "../types/mesh";

interface ControlPanelProps {
  composition: number;
  instanceCount: number;
  network: NetworkState;
  onComposition: (value: number) => void;
  onNetwork: (key: NetworkControlKey, value: number | boolean) => void;
  onVolume: (value: number) => void;
  sampleRate: number;
  volume: number;
}

export function ControlPanel(props: ControlPanelProps) {
  const { composition, instanceCount, network, onComposition, onNetwork, onVolume, sampleRate, volume } = props;
  return (
    <aside className="control-panel" aria-label="Module settings">
      <div className="panel-heading">
        <p>SETTINGS</p>
        <span>Music and connection</span>
      </div>

      <section className="control-section">
        <div className="section-title"><strong><Music2 size={14} /> Music</strong></div>
        <label className="field">
          <span>Music set</span>
          <select value={composition} onChange={(event) => onComposition(Number(event.target.value))}>
            <option value={2}>Tape 02</option><option value={7}>Tape 07</option>
            <option value={17}>Tape 17</option><option value={42}>Tape 42</option>
          </select>
        </label>
        <div className="monitor-level">
          <Range label="Master volume" value={volume * 100} min={0} max={100} step={1} suffix="%" decimals={0} onChange={(value) => onVolume(value / 100)} />
        </div>
      </section>

      <section className="control-section connection-section">
        <label className="switch-row">
          <span><strong><Link2 size={14} /> Sync all modules</strong><small>Keep every module playing together</small></span>
          <input type="checkbox" checked={network.enabled} onChange={(event) => onNetwork("enabled", event.target.checked)} />
        </label>
      </section>

      <details className="control-testing">
        <summary><span><SlidersHorizontal size={14} /> Network testing</span><small>Advanced</small></summary>
        <div className="testing-content">
          <Range label="Message delay" value={network.latencyMs} min={0} max={30} step={0.5} suffix=" ms" onChange={(value) => onNetwork("latencyMs", value)} />
          <Range label="Delay variation" value={network.jitterMs} min={0} max={15} step={0.5} suffix=" ms" onChange={(value) => onNetwork("jitterMs", value)} />
          <Range label="Messages lost" value={network.lossPercent} min={0} max={50} step={1} suffix="%" decimals={0} onChange={(value) => onNetwork("lossPercent", value)} />
          <dl className="network-stats">
            <Stat label="Sent" value={network.sent} /><Stat label="Arrived" value={network.delivered} />
            <Stat label="Lost" value={network.dropped} /><Stat label="In flight" value={network.queued} />
          </dl>
        </div>
      </details>

      <footer className="runtime-info">
        <span>MODULES <b>{instanceCount}</b></span>
        <span>{sampleRate ? `${Math.round(sampleRate).toLocaleString()} HZ` : "--"}</span>
        <span>BROWSER AUDIO</span>
      </footer>
    </aside>
  );
}

interface RangeProps {
  decimals?: number;
  label: string;
  max: number;
  min: number;
  onChange: (value: number) => void;
  step: number;
  suffix: string;
  value: number;
}

function Range({ decimals = 1, label, max, min, onChange, step, suffix, value }: RangeProps) {
  return (
    <label className="range-field">
      <span>{label}</span><output>{value.toFixed(decimals)}{suffix}</output>
      <input type="range" min={min} max={max} step={step} value={value} onChange={(event) => onChange(Number(event.target.value))} />
    </label>
  );
}

function Stat({ label, value }: { label: string; value: number }) {
  return <div><dt>{label}</dt><dd>{value}</dd></div>;
}
