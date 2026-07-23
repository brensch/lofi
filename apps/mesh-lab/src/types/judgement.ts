export type JudgementVerdict = "dislike" | "like";

export type CandidateEngine = "loops" | "symbolic";

export interface CandidateIdentity {
  bars: 8;
  bpm: number;
  buildRevision: string;
  durationMs: number;
  engine: CandidateEngine;
  nodeCount: 3;
  profileId: string;
  sampleRate: number;
  seed: number;
  sequence: number;
  sourceSlot: number;
  startPhrase: number;
}

export interface JudgementRecord {
  candidate: CandidateIdentity;
  id: string;
  decisionMs: number;
  listenedMs: number;
  note: string;
  replayCount: number;
  schemaVersion: 1;
  sessionId: string;
  submittedAt: string;
  tags: string[];
  verdict: JudgementVerdict;
}

export interface StoredJudgement {
  record: JudgementRecord;
  synced: boolean;
}
