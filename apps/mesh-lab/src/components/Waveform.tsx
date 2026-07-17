import { useEffect, useRef } from "react";

const VISUAL_REFRESH_MS = 100;

interface WaveformProps {
  analyser?: AnalyserNode;
  running: boolean;
}

export function Waveform({ analyser, running }: WaveformProps) {
  const canvasRef = useRef<HTMLCanvasElement>(null);

  useEffect(() => {
    const canvas = canvasRef.current;
    if (!canvas) return;
    const drawing = canvas.getContext("2d");
    if (!drawing) return;
    const samples = analyser ? new Uint8Array(analyser.fftSize) : undefined;
    let timerId = 0;

    const draw = () => {
      const ratio = Math.min(window.devicePixelRatio || 1, 1.5);
      const width = Math.max(1, Math.floor(canvas.clientWidth * ratio));
      const height = Math.max(1, Math.floor(canvas.clientHeight * ratio));
      if (canvas.width !== width || canvas.height !== height) { canvas.width = width; canvas.height = height; }
      drawing.fillStyle = "#151a18";
      drawing.fillRect(0, 0, width, height);
      drawing.strokeStyle = "#28312e";
      drawing.lineWidth = 1;
      for (let line = 1; line < 4; line += 1) {
        const y = (height * line) / 4;
        drawing.beginPath(); drawing.moveTo(0, y); drawing.lineTo(width, y); drawing.stroke();
      }
      if (analyser && samples && running) {
        analyser.getByteTimeDomainData(samples);
        drawing.strokeStyle = "#65d99a";
        drawing.lineWidth = Math.max(1.5, ratio);
        drawing.beginPath();
        for (let index = 0; index < samples.length; index += 1) {
          const sample = (samples[index] - 128) / 128;
          const x = (index / (samples.length - 1)) * width;
          const y = (0.5 - sample * 0.43) * height;
          if (index === 0) drawing.moveTo(x, y); else drawing.lineTo(x, y);
        }
        drawing.stroke();
      }
      if (running) timerId = window.setTimeout(draw, VISUAL_REFRESH_MS);
    };
    draw();
    return () => window.clearTimeout(timerId);
  }, [analyser, running]);

  return <div className="scope" aria-label="Mixed output waveform"><canvas ref={canvasRef} /><span className="scope-label">MIX OUT</span></div>;
}
