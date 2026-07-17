import type { JudgementRecord, StoredJudgement } from "../types/judgement";

const STORAGE_KEY = "lofi-mesh:judgements:v1";
const SESSION_KEY = "lofi-mesh:judgement-session:v1";

export function judgementSessionId() {
  const existing = sessionStorage.getItem(SESSION_KEY);
  if (existing) return existing;
  const sessionId = crypto.randomUUID();
  sessionStorage.setItem(SESSION_KEY, sessionId);
  return sessionId;
}

export function loadJudgements(): StoredJudgement[] {
  try {
    const parsed = JSON.parse(localStorage.getItem(STORAGE_KEY) ?? "[]") as unknown;
    if (!Array.isArray(parsed)) return [];
    return parsed.filter(isStoredJudgement);
  } catch {
    return [];
  }
}

export async function storeJudgement(record: JudgementRecord): Promise<StoredJudgement[]> {
  const records = loadJudgements();
  const existing = records.findIndex((item) => item.record.id === record.id);
  const stored = { record, synced: false };
  if (existing >= 0) records[existing] = stored;
  else records.push(stored);
  persist(records);

  const synced = await postJudgement(record);
  if (synced) {
    const item = records.find((candidate) => candidate.record.id === record.id);
    if (item) item.synced = true;
    persist(records);
  }
  return records;
}

export async function retryPendingJudgements(): Promise<StoredJudgement[]> {
  const records = loadJudgements();
  for (const item of records) {
    if (!item.synced && await postJudgement(item.record)) {
      item.synced = true;
      persist(records);
    }
  }
  return records;
}

export function downloadJudgements(records: StoredJudgement[]) {
  const payload = {
    exportedAt: new Date().toISOString(),
    records: records.map((item) => item.record),
    schemaVersion: 1,
  };
  const blob = new Blob([JSON.stringify(payload, null, 2)], { type: "application/json" });
  const url = URL.createObjectURL(blob);
  const anchor = document.createElement("a");
  anchor.href = url;
  anchor.download = `lofi-judgements-${new Date().toISOString().slice(0, 10)}.json`;
  anchor.click();
  URL.revokeObjectURL(url);
}

async function postJudgement(record: JudgementRecord) {
  try {
    const response = await fetch("/api/judgements", {
      body: JSON.stringify(record),
      headers: { "Content-Type": "application/json" },
      method: "POST",
    });
    return response.ok;
  } catch {
    return false;
  }
}

function persist(records: StoredJudgement[]) {
  localStorage.setItem(STORAGE_KEY, JSON.stringify(records));
}

function isStoredJudgement(value: unknown): value is StoredJudgement {
  if (!value || typeof value !== "object") return false;
  const item = value as Partial<StoredJudgement>;
  return typeof item.synced === "boolean"
    && !!item.record
    && item.record.schemaVersion === 1
    && typeof item.record.id === "string";
}
