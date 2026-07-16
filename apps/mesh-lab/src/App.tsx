import { useState } from "react";

import { AppHeader } from "./components/AppHeader";
import { ControlPanel } from "./components/ControlPanel";
import { ModuleGrid } from "./components/ModuleGrid";
import { Waveform } from "./components/Waveform";
import { useMeshAudio } from "./hooks/useMeshAudio";
import { MAX_NODES } from "./types/mesh";

const SESSION_SEED = 2;
const INITIAL_VOLUME = 0.7;

export function App() {
  const [volume, setVolume] = useState(INITIAL_VOLUME);
  const mesh = useMeshAudio(SESSION_SEED, INITIAL_VOLUME);
  const running = mesh.runtime === "running";

  const changeVolume = (value: number) => {
    setVolume(value);
    mesh.setVolume(value);
  };

  const leader = mesh.telemetry.nodes.find((node) => node.isRoot) ?? mesh.telemetry.nodes[0];

  return (
    <div className="app-shell">
      <AppHeader
        runtime={mesh.runtime}
        canAdd={mesh.telemetry.nodes.length > 0 && mesh.telemetry.nodes.length < MAX_NODES}
        onAdd={mesh.addNode}
        onToggle={mesh.toggle}
      />
      {mesh.error && <div className="error-banner" role="alert">{mesh.error}</div>}
      <main className="workspace">
        <ControlPanel
          beatsToChangeMilli={leader?.beatsToChangeMilli}
          upcomingChange={leader?.upcomingChange}
          instanceCount={mesh.telemetry.nodes.length}
          network={mesh.telemetry.network}
          onNetwork={mesh.updateNetwork}
          onVolume={changeVolume}
          sampleRate={mesh.telemetry.sampleRate}
          volume={volume}
        />
        <section className="stage" aria-label="Virtual modules">
          <Waveform analyser={mesh.analyser} running={running} />
          <ModuleGrid nodes={mesh.telemetry.nodes} onRemove={mesh.removeNode} onUpdate={mesh.updateNode} />
        </section>
      </main>
    </div>
  );
}
