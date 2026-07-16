import { Clock3, Link2, Music2, SlidersHorizontal } from "lucide-react";

import type { NetworkControlKey, NetworkState } from "../types/mesh";

interface ControlPanelProps {
  beatsToChangeMilli?: number;
  instanceCount: number;
  network: NetworkState;
  onNetwork: (key: NetworkControlKey, value: number | boolean) => void;
  onVolume: (value: number) => void;
  sampleRate: number;
  upcomingChange?: number;
  volume: number;
}

export function ControlPanel(props: ControlPanelProps) {
  const { beatsToChangeMilli, instanceCount, network, onNetwork, onVolume, sampleRate, upcomingChange, volume } = props;
  const change = CHANGE_COPY[upcomingChange ?? 0] ?? DEFAULT_CHANGE;
  const beatsRemaining = beatsToChangeMilli === undefined
    ? undefined
    : Math.max(0, Math.ceil(beatsToChangeMilli / 1_000));
  const changeProgress = beatsToChangeMilli === undefined
    ? 0
    : Math.max(0, Math.min(100, (1 - beatsToChangeMilli / 32_000) * 100));
  return (
    <aside className="control-panel" aria-label="Module settings">
      <div className="panel-heading">
        <p>SETTINGS</p>
        <span>Music and connection</span>
      </div>

      <section className="control-section">
        <div className="section-title"><strong><Music2 size={14} /> Music</strong></div>
        <div className="change-preview" aria-live="polite">
          <span>{change.role}</span>
          <strong>{change.headline}</strong>
          <p>{change.detail}</p>
        </div>
        <div className="change-countdown" role="timer" aria-label={`Next music change in ${beatsRemaining ?? "unknown"} beats`}>
          <span><Clock3 size={14} /> Change boundary</span>
          <output>{beatsRemaining ?? "--"}<small> beats</small></output>
          <small className="change-bars">{formatBeatBoundary(beatsRemaining)}</small>
          <div><i style={{ width: `${changeProgress}%` }} /></div>
        </div>
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

interface ChangeCopy {
  detail: string;
  headline: string;
  role: string;
}

const DEFAULT_CHANGE: ChangeCopy = {
  role: "ARRANGEMENT",
  headline: "Waiting for the next variation",
  detail: "The shared transport will announce the incoming part.",
};

const CHANGE_COPY: Record<number, ChangeCopy> = {
  1: { role: "DRUMS", headline: "Hi-hats double up", detail: "The pocket adds sixteenth-note motion." },
  2: { role: "DRUMS", headline: "Hi-hats thin out", detail: "The pocket opens more space between hits." },
  3: { role: "DRUMS", headline: "Open hats enter", detail: "Offbeats gain a longer, brighter accent." },
  4: { role: "DRUMS", headline: "Ghost snares enter", detail: "Quiet syncopated hits fill the pocket." },
  5: { role: "KICK", headline: "Kick shifts forward", detail: "The downbeat pattern becomes more active." },
  6: { role: "KICK", headline: "Kick turns syncopated", detail: "Extra offbeat kicks reshape the groove." },
  7: { role: "KICK", headline: "Kick drops to half-time", detail: "The pulse clears space around each downbeat." },
  8: { role: "DRUMS", headline: "Drum fill enters", detail: "The final bar adds a synchronized snare fill." },
  9: { role: "GROOVE", headline: "Swing gets deeper", detail: "The offbeats lean further behind the grid." },
  12: { role: "BASS", headline: "Low end gains weight", detail: "The bass layer steps forward in the mix." },
  13: { role: "BASS", headline: "Bass gets more active", detail: "The low part takes a stronger role." },
  14: { role: "CHORDS", headline: "Chords step forward", detail: "The harmony becomes more prominent." },
  16: { role: "CHORDS", headline: "Chords pull back", detail: "The harmony makes room for the other parts." },
  18: { role: "MELODY", headline: "Motif steps forward", detail: "The melodic phrase becomes more prominent." },
  21: { role: "TEXTURE", headline: "Texture steps forward", detail: "The background layer becomes more present." },
};

function formatBeatBoundary(beats?: number) {
  if (beats === undefined) return "Waiting for transport";
  if (beats === 0) return "Changing on this beat";
  const bars = Math.floor(beats / 4);
  const remainder = beats % 4;
  if (bars === 0) return `${remainder} ${remainder === 1 ? "beat" : "beats"}`;
  if (remainder === 0) return `${bars} ${bars === 1 ? "bar" : "bars"}`;
  return `${bars} ${bars === 1 ? "bar" : "bars"} + ${remainder} ${remainder === 1 ? "beat" : "beats"}`;
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
