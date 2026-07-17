import { Check, Copy, Download, Pause, Play, RotateCcw, UploadCloud, X } from "lucide-react";
import { useCallback, useEffect, useMemo, useRef, useState } from "react";

import { AppNavigation } from "../components/AppNavigation";
import { Waveform } from "../components/Waveform";
import {
  useJudgementAudio,
} from "../hooks/useJudgementAudio";
import { candidateKey, CANDIDATE_BARS, newCandidate } from "../lib/candidateProfiles";
import {
  downloadJudgements,
  judgementSessionId,
  loadJudgements,
  retryPendingJudgements,
  storeJudgement,
} from "../lib/judgements";
import type { CandidateIdentity, JudgementRecord, JudgementVerdict, StoredJudgement } from "../types/judgement";
import { APP_VERSION } from "../version";

const INITIAL_VOLUME = 0.7;
const POSITIVE_TAGS = ["Groove", "Melody", "Drums", "Bass", "Texture", "Evolution", "Mix"];
const PROBLEM_TAGS = [
  "Timing",
  "Playback glitch",
  "Repetitive",
  "Harsh bass",
  "Weak melody",
  "Bad samples",
  "Too sparse",
  "Too busy",
  "Not lo-fi",
  "Mix balance",
];

export function JudgementPage() {
  const initialRecords = useMemo(loadJudgements, []);
  const seenCandidatesRef = useRef(new Set(initialRecords.map((item) => candidateKey({
    seed: item.record.candidate.seed,
    startPhrase: item.record.candidate.startPhrase ?? 0,
  }))));
  const sessionIdRef = useRef(judgementSessionId());
  const completedAtRef = useRef(0);
  const debugStatusTimerRef = useRef(0);
  const [candidate, setCandidate] = useState(() => newCandidate(
    initialRecords.length + 1,
    initialRecords.at(-1)?.record.candidate,
    seenCandidatesRef.current,
  ));
  const [note, setNote] = useState("");
  const [records, setRecords] = useState(initialRecords);
  const [replayCount, setReplayCount] = useState(0);
  const [selectedTags, setSelectedTags] = useState<string[]>([]);
  const [submitting, setSubmitting] = useState(false);
  const [debugStatus, setDebugStatus] = useState("Copy debug");
  const [volume, setVolume] = useState(INITIAL_VOLUME);
  const audio = useJudgementAudio(INITIAL_VOLUME);
  const pendingCount = records.filter((item) => !item.synced).length;
  const likes = records.filter((item) => item.record.verdict === "like").length;
  const progress = Math.min(1, audio.elapsedMs / candidate.durationMs);
  const currentBar = Math.min(CANDIDATE_BARS, Math.floor(progress * CANDIDATE_BARS) + 1);

  useEffect(() => {
    void retryPendingJudgements().then(setRecords);
    return () => window.clearTimeout(debugStatusTimerRef.current);
  }, []);

  useEffect(() => {
    if (audio.runtime === "complete") completedAtRef.current = performance.now();
  }, [audio.runtime]);

  const toggleTag = (tag: string) => {
    setSelectedTags((current) => current.includes(tag)
      ? current.filter((item) => item !== tag)
      : [...current, tag]);
  };

  const replay = async () => {
    setReplayCount((count) => count + 1);
    await audio.start(candidate);
  };

  const submit = useCallback(async (verdict: JudgementVerdict) => {
    if (audio.runtime !== "complete" || submitting) return;
    setSubmitting(true);
    const identity: CandidateIdentity = {
      ...candidate,
      sampleRate: audio.sampleRate,
    };
    const record: JudgementRecord = {
      candidate: identity,
      decisionMs: Math.max(0, Math.round(performance.now() - completedAtRef.current)),
      id: crypto.randomUUID(),
      listenedMs: candidate.durationMs * (replayCount + 1),
      note: note.trim(),
      replayCount,
      schemaVersion: 1,
      sessionId: sessionIdRef.current,
      submittedAt: new Date().toISOString(),
      tags: selectedTags,
      verdict,
    };
    const nextRecords = await storeJudgement(record);
    setRecords(nextRecords);
    setNote("");
    setReplayCount(0);
    setSelectedTags([]);
    const next = newCandidate(candidate.sequence + 1, candidate, seenCandidatesRef.current);
    setCandidate(next);
    setSubmitting(false);
    await audio.start(next);
  }, [audio, candidate, note, replayCount, selectedTags, submitting]);

  useEffect(() => {
    const onKeyDown = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.matches("textarea, input")) return;
      if (event.key === "ArrowLeft") void submit("dislike");
      if (event.key === "ArrowRight") void submit("like");
    };
    window.addEventListener("keydown", onKeyDown);
    return () => window.removeEventListener("keydown", onKeyDown);
  }, [submit]);

  const changeVolume = (nextVolume: number) => {
    setVolume(nextVolume);
    audio.setVolume(nextVolume);
  };

  const copyDebugState = async () => {
    try {
      const snapshot = await audio.debugSnapshot(candidate);
      await copyText(snapshot);
      setDebugStatus("Copied");
    } catch {
      setDebugStatus("Copy failed");
    }
    window.clearTimeout(debugStatusTimerRef.current);
    debugStatusTimerRef.current = window.setTimeout(() => setDebugStatus("Copy debug"), 2_000);
  };

  return (
    <div className="app-shell judgement-shell">
      <header className="topbar judgement-topbar">
        <div className="brand">
          <span className="brand-mark" aria-hidden="true"><i /><i /><i /></span>
          <span className="brand-name"><strong>LOFI MESH</strong><small>v{APP_VERSION}</small></span>
          <AppNavigation active="judge" />
        </div>
        <div className={`transport-status ${audio.runtime === "playing" ? "running" : audio.runtime === "error" ? "error" : ""}`} role="status">
          <span className="status-dot" />
          <span>{runtimeLabel(audio.runtime)}</span>
        </div>
        <div className="header-actions">
          <button className="secondary-button" type="button" onClick={copyDebugState}>
            <Copy size={15} /> {debugStatus}
          </button>
          <button className="secondary-button" type="button" disabled={!records.length} onClick={() => downloadJudgements(records)}>
            <Download size={15} /> Export
          </button>
        </div>
      </header>

      {audio.error && <div className="error-banner" role="alert">{audio.error}</div>}
      <main className="judgement-workspace">
        <aside className="study-sidebar">
          <div className="panel-heading"><p>LISTENING SESSION</p><span>Personal taste dataset</span></div>
          <section className="study-stats" aria-label="Session results">
            <StudyStat label="Judged" value={records.length} />
            <StudyStat label="Liked" value={likes} />
            <StudyStat label="Rate" value={records.length ? `${Math.round((likes / records.length) * 100)}%` : "--"} />
          </section>
          <section className="control-section">
            <label className="range-field">
              <span>Monitor volume</span><output>{Math.round(volume * 100)}%</output>
              <input type="range" min={0} max={100} step={1} value={volume * 100} onChange={(event) => changeVolume(Number(event.target.value) / 100)} />
            </label>
          </section>
          <section className="study-sync">
            <span><UploadCloud size={14} /> Saved to this box</span>
            <strong>{pendingCount ? `${pendingCount} pending` : "Up to date"}</strong>
          </section>
          <RecentJudgements records={records} />
          <footer className="runtime-info">
            <span>BUILD <b>{__BUILD_REVISION__}</b></span>
            <span>{audio.sampleRate ? `${audio.sampleRate.toLocaleString()} HZ` : "--"}</span>
          </footer>
        </aside>

        <section className="judgement-stage" aria-label="Listening candidate">
          <Waveform analyser={audio.analyser} running={audio.runtime === "playing"} />
          <div className="candidate-heading">
            <div><p>CANDIDATE</p><h1>{String(candidate.sequence).padStart(3, "0")}</h1></div>
            <span>{CANDIDATE_BARS} bars · {candidate.bpm} BPM · {candidate.nodeCount} modules</span>
          </div>
          <div className="bar-progress" aria-label={`Bar ${currentBar} of ${CANDIDATE_BARS}`}>
            {Array.from({ length: CANDIDATE_BARS }, (_, index) => {
              const barProgress = Math.max(0, Math.min(1, progress * CANDIDATE_BARS - index));
              return <i key={index}><b style={{ transform: `scaleX(${barProgress})` }} /></i>;
            })}
          </div>

          <div className={`candidate-body ${audio.runtime === "complete" ? "rating" : ""}`}>
            {audio.runtime === "idle" || audio.runtime === "error" ? (
              <button className="start-candidate" type="button" onClick={() => audio.start(candidate)}>
                <Play size={18} fill="currentColor" /> Start sample
              </button>
            ) : audio.runtime === "loading" ? (
              <div className="candidate-wait"><span className="loading-meter" /><strong>Preparing candidate</strong></div>
            ) : audio.runtime === "playing" || audio.runtime === "paused" ? (
              <div className="playback-control">
                <button className="round-control" type="button" title={audio.runtime === "playing" ? "Pause" : "Resume"} aria-label={audio.runtime === "playing" ? "Pause" : "Resume"} onClick={audio.togglePause}>
                  {audio.runtime === "playing" ? <Pause size={25} fill="currentColor" /> : <Play size={25} fill="currentColor" />}
                </button>
                <strong>BAR {currentBar} / {CANDIDATE_BARS}</strong>
                <span>{formatRemaining(candidate.durationMs - audio.elapsedMs)}</span>
              </div>
            ) : (
              <div className="rating-layout">
                <div className="feedback-fields">
                  <FeedbackGroup label="Worked" options={POSITIVE_TAGS} selected={selectedTags} onToggle={toggleTag} />
                  <FeedbackGroup label="Problems" options={PROBLEM_TAGS} selected={selectedTags} onToggle={toggleTag} />
                  <label className="note-field">
                    <span>Notes <small>Optional</small></span>
                    <textarea maxLength={600} rows={3} value={note} onChange={(event) => setNote(event.target.value)} placeholder="Specific moment, instrument, or feeling" />
                  </label>
                </div>
                <div className="verdict-controls">
                  <button className="verdict-button dislike" type="button" title="Dislike" aria-label="Dislike this candidate" disabled={submitting} onClick={() => submit("dislike")}><X size={31} /></button>
                  <button className="replay-button" type="button" title="Replay candidate" aria-label="Replay candidate" disabled={submitting} onClick={replay}><RotateCcw size={18} /></button>
                  <button className="verdict-button like" type="button" title="Like" aria-label="Like this candidate" disabled={submitting} onClick={() => submit("like")}><Check size={31} /></button>
                </div>
              </div>
            )}
          </div>
        </section>
      </main>
    </div>
  );
}

async function copyText(text: string) {
  if (navigator.clipboard?.writeText) {
    await navigator.clipboard.writeText(text);
    return;
  }
  const field = document.createElement("textarea");
  field.value = text;
  field.style.position = "fixed";
  field.style.opacity = "0";
  document.body.append(field);
  field.select();
  const copied = document.execCommand("copy");
  field.remove();
  if (!copied) throw new Error("Clipboard is unavailable");
}

function runtimeLabel(runtime: ReturnType<typeof useJudgementAudio>["runtime"]) {
  if (runtime === "playing") return "Candidate playing";
  if (runtime === "paused") return "Candidate paused";
  if (runtime === "complete") return "Ready to judge";
  if (runtime === "loading") return "Preparing audio";
  if (runtime === "error") return "Could not start";
  return "Ready to listen";
}

function formatRemaining(remainingMs: number) {
  const seconds = Math.max(0, Math.ceil(remainingMs / 1_000));
  return `0:${String(seconds).padStart(2, "0")}`;
}

function StudyStat({ label, value }: { label: string; value: number | string }) {
  return <div><span>{label}</span><strong>{value}</strong></div>;
}

function FeedbackGroup(props: { label: string; onToggle: (tag: string) => void; options: string[]; selected: string[] }) {
  return (
    <fieldset className="feedback-group">
      <legend>{props.label}</legend>
      <div>{props.options.map((option) => (
        <label key={option} className={props.selected.includes(option) ? "selected" : ""}>
          <input type="checkbox" checked={props.selected.includes(option)} onChange={() => props.onToggle(option)} />
          <span>{option}</span>
        </label>
      ))}</div>
    </fieldset>
  );
}

function RecentJudgements({ records }: { records: StoredJudgement[] }) {
  const recent = records.slice(-6).reverse();
  return (
    <section className="recent-judgements">
      <header><span>Recent</span><strong>{records.length}</strong></header>
      {recent.length ? recent.map(({ record, synced }) => (
        <div key={record.id}>
          <span className={record.verdict}>{record.verdict === "like" ? <Check size={12} /> : <X size={12} />}</span>
          <strong>Candidate {String(record.candidate.sequence).padStart(3, "0")}</strong>
          <small>{record.tags.slice(0, 2).join(" · ") || "No tags"}</small>
          <i className={synced ? "synced" : ""} title={synced ? "Saved to box" : "Pending upload"} />
        </div>
      )) : <p>No judgements yet</p>}
    </section>
  );
}
