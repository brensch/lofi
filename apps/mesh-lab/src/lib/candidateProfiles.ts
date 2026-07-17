import type { CandidateIdentity } from "../types/judgement";
import { APP_VERSION } from "../version";

export const CANDIDATE_BARS = 8 as const;
export const CANDIDATE_NODES = 3 as const;

interface CandidateProfile {
  contrastGroup: number;
  family: number;
  id: string;
  seed: number;
  sourceSlot: number;
  startPhrase: number;
}

// Seeds are chosen against catalog.pack so each source covers sharply different
// rhythmic states. IDs and families are stored for analysis but never shown
// before a verdict, preserving the blind comparison.
const PROFILES: CandidateProfile[] = [
  { id: "a-half", sourceSlot: 0, contrastGroup: 0, family: 0, seed: 0, startPhrase: 10 },
  { id: "a-double", sourceSlot: 0, contrastGroup: 0, family: 1, seed: 3, startPhrase: 8 },
  { id: "a-walk", sourceSlot: 0, contrastGroup: 0, family: 2, seed: 6, startPhrase: 13 },
  { id: "a-sparse", sourceSlot: 0, contrastGroup: 0, family: 3, seed: 12, startPhrase: 17 },
  { id: "b-half", sourceSlot: 1, contrastGroup: 1, family: 0, seed: 1, startPhrase: 12 },
  { id: "b-double", sourceSlot: 1, contrastGroup: 1, family: 1, seed: 4, startPhrase: 58 },
  { id: "b-walk", sourceSlot: 1, contrastGroup: 1, family: 2, seed: 7, startPhrase: 24 },
  { id: "b-sparse", sourceSlot: 1, contrastGroup: 1, family: 3, seed: 10, startPhrase: 14 },
  { id: "c-half", sourceSlot: 2, contrastGroup: 1, family: 0, seed: 2, startPhrase: 21 },
  { id: "c-double", sourceSlot: 2, contrastGroup: 1, family: 1, seed: 5, startPhrase: 6 },
  { id: "c-walk", sourceSlot: 2, contrastGroup: 1, family: 2, seed: 8, startPhrase: 13 },
  { id: "c-sparse", sourceSlot: 2, contrastGroup: 1, family: 3, seed: 11, startPhrase: 14 },
];

export function candidateKey(candidate: Pick<CandidateIdentity, "seed" | "startPhrase">) {
  return `${candidate.seed}:${candidate.startPhrase}`;
}

export function newCandidate(
  sequence: number,
  previous: CandidateIdentity | undefined,
  seen: Set<string>,
): CandidateIdentity {
  let available = eligibleProfiles(previous, seen);
  if (!available.length) {
    seen.clear();
    available = eligibleProfiles(previous, seen);
  }
  const profile = available[randomIndex(available.length)];
  const bpm = profile.sourceSlot === 0 ? 72 : 80;
  const durationMs = Math.round((CANDIDATE_BARS * 4 * 60_000) / bpm);
  const candidate: CandidateIdentity = {
    bars: CANDIDATE_BARS,
    bpm,
    buildRevision: `${APP_VERSION}+${__BUILD_REVISION__}`,
    durationMs,
    nodeCount: CANDIDATE_NODES,
    profileId: profile.id,
    sampleRate: 0,
    seed: profile.seed,
    sequence,
    sourceSlot: profile.sourceSlot,
    startPhrase: profile.startPhrase,
  };
  seen.add(candidateKey(candidate));
  return candidate;
}

function eligibleProfiles(previous: CandidateIdentity | undefined, seen: Set<string>) {
  const previousProfile = previous
    ? PROFILES.find((profile) => profile.id === previous.profileId)
    : undefined;
  return PROFILES.filter((profile) =>
    !seen.has(`${profile.seed}:${profile.startPhrase}`)
    && profile.contrastGroup !== previousProfile?.contrastGroup
    && profile.family !== previousProfile?.family
  );
}

function randomIndex(length: number) {
  return crypto.getRandomValues(new Uint32Array(1))[0] % length;
}
