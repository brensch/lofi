import type { CandidateIdentity } from "../types/judgement";
import { APP_VERSION } from "../version";

export const CANDIDATE_BARS = 8 as const;
export const CANDIDATE_NODES = 3 as const;

interface CandidateProfile {
  bpm?: number;
  contrastGroup: number;
  engine: "loops" | "symbolic";
  family: number;
  id: string;
  seed: number;
  sourceSlot: number;
  startPhrase: number;
}

// Symbolic candidates that passed every property gate and craft window in
// the overnight sweep (tools/listen-qa/candidates.py, 48 seeds, 13
// survivors). The eight below span all three groove signatures, flat- and
// sharp-side keys, and both dense and skeletal characters. Families rotate
// so consecutive symbolic trials change rhythmic identity.
const SYMBOLIC_PROFILES: CandidateProfile[] = [
  { id: "s-window-6", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 0, seed: 6, startPhrase: 1, bpm: 78 },
  { id: "s-polar-2", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 1, seed: 2, startPhrase: 2, bpm: 78 },
  { id: "s-polar-17", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 2, seed: 17, startPhrase: 1, bpm: 80 },
  { id: "s-window-13", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 3, seed: 13, startPhrase: 2, bpm: 76 },
  { id: "s-polar-42", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 0, seed: 42, startPhrase: 5, bpm: 74 },
  { id: "s-window-26", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 1, seed: 26, startPhrase: 1, bpm: 82 },
  { id: "s-float-19", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 2, seed: 19, startPhrase: 2, bpm: 76 },
  { id: "s-window-31", engine: "symbolic", sourceSlot: 8, contrastGroup: 2, family: 3, seed: 31, startPhrase: 6, bpm: 72 },
];

// Loop seeds are chosen against catalog.pack so each source covers sharply
// different rhythmic states. Symbolic seeds come from the gated overnight
// sweep (tools/listen-qa/candidates.py). IDs, families, and engines are
// stored for analysis but never shown before a verdict, preserving the blind
// comparison. Symbolic profiles share contrast group 2, so consecutive
// trials always cross between the engines.
const PROFILES: CandidateProfile[] = [
  { id: "a-half", engine: "loops", sourceSlot: 0, contrastGroup: 0, family: 0, seed: 0, startPhrase: 10 },
  { id: "a-double", engine: "loops", sourceSlot: 0, contrastGroup: 0, family: 1, seed: 3, startPhrase: 8 },
  { id: "a-walk", engine: "loops", sourceSlot: 0, contrastGroup: 0, family: 2, seed: 6, startPhrase: 13 },
  { id: "a-sparse", engine: "loops", sourceSlot: 0, contrastGroup: 0, family: 3, seed: 12, startPhrase: 17 },
  { id: "b-half", engine: "loops", sourceSlot: 1, contrastGroup: 1, family: 0, seed: 1, startPhrase: 12 },
  { id: "b-double", engine: "loops", sourceSlot: 1, contrastGroup: 1, family: 1, seed: 4, startPhrase: 58 },
  { id: "b-walk", engine: "loops", sourceSlot: 1, contrastGroup: 1, family: 2, seed: 7, startPhrase: 24 },
  { id: "b-sparse", engine: "loops", sourceSlot: 1, contrastGroup: 1, family: 3, seed: 10, startPhrase: 14 },
  { id: "c-half", engine: "loops", sourceSlot: 2, contrastGroup: 1, family: 0, seed: 2, startPhrase: 21 },
  { id: "c-double", engine: "loops", sourceSlot: 2, contrastGroup: 1, family: 1, seed: 5, startPhrase: 6 },
  { id: "c-walk", engine: "loops", sourceSlot: 2, contrastGroup: 1, family: 2, seed: 8, startPhrase: 13 },
  { id: "c-sparse", engine: "loops", sourceSlot: 2, contrastGroup: 1, family: 3, seed: 11, startPhrase: 14 },
  ...SYMBOLIC_PROFILES,
];

export function candidateKey(
  candidate: Pick<CandidateIdentity, "engine" | "seed" | "startPhrase">,
) {
  return `${candidate.engine}:${candidate.seed}:${candidate.startPhrase}`;
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
  const bpm = profile.bpm ?? (profile.sourceSlot === 0 ? 72 : 80);
  const durationMs = Math.round((CANDIDATE_BARS * 4 * 60_000) / bpm);
  const candidate: CandidateIdentity = {
    bars: CANDIDATE_BARS,
    bpm,
    buildRevision: `${APP_VERSION}+${__BUILD_REVISION__}`,
    durationMs,
    engine: profile.engine,
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
    !seen.has(`${profile.engine}:${profile.seed}:${profile.startPhrase}`)
    && profile.contrastGroup !== previousProfile?.contrastGroup
    && profile.family !== previousProfile?.family
  );
}

function randomIndex(length: number) {
  return crypto.getRandomValues(new Uint32Array(1))[0] % length;
}
