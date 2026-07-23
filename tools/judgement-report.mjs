#!/usr/bin/env node

import fs from "node:fs";

const filename = process.argv[2] ?? "target/listening-study/judgements.jsonl";
if (!fs.existsSync(filename)) {
  process.stdout.write(`No judgement data at ${filename}\n`);
  process.exit(0);
}

const records = new Map();
for (const line of fs.readFileSync(filename, "utf8").split("\n")) {
  if (!line) continue;
  try {
    const record = JSON.parse(line);
    if (record.schemaVersion === 1 && typeof record.id === "string") records.set(record.id, record);
  } catch {
    process.stderr.write("Skipped one malformed judgement line\n");
  }
}

const values = [...records.values()];
const likes = values.filter((record) => record.verdict === "like");
const dislikes = values.filter((record) => record.verdict === "dislike");
const tagStats = new Map();
for (const record of values) {
  for (const tag of record.tags ?? []) {
    const stats = tagStats.get(tag) ?? { dislike: 0, like: 0, total: 0 };
    stats[record.verdict] += 1;
    stats.total += 1;
    tagStats.set(tag, stats);
  }
}

const percent = (part, total) => total ? `${Math.round((part / total) * 100)}%` : "--";
process.stdout.write("Lofi listening study\n");
process.stdout.write(`Judgements: ${values.length} | liked: ${likes.length} (${percent(likes.length, values.length)}) | disliked: ${dislikes.length}\n`);
process.stdout.write(`Full listens: ${values.reduce((sum, record) => sum + 1 + (record.replayCount ?? 0), 0)} | notes: ${values.filter((record) => record.note).length}\n`);

// The head-to-head that decides the engine question.
for (const engine of ["symbolic", "loops"]) {
  const group = values.filter((record) => (record.candidate.engine ?? "loops") === engine);
  const groupLikes = group.filter((record) => record.verdict === "like").length;
  process.stdout.write(
    `Engine ${engine}: ${group.length} judgements, ${groupLikes} liked (${percent(groupLikes, group.length)})\n`,
  );
}

if (tagStats.size) {
  process.stdout.write("\nTag signal\n");
  const sortedTags = [...tagStats.entries()].sort((left, right) => right[1].total - left[1].total);
  for (const [tag, stats] of sortedTags) {
    process.stdout.write(`- ${tag}: ${stats.total} selections, ${stats.like} like / ${stats.dislike} dislike\n`);
  }
}

const candidates = (items) => items.map((record) => {
  const profile = record.candidate.profileId ? `/${record.candidate.profileId}` : "";
  return `${record.candidate.seed}${profile}`;
}).join(", ") || "none";
process.stdout.write(`\nLiked candidates: ${candidates(likes)}\n`);
process.stdout.write(`Disliked candidates: ${candidates(dislikes)}\n`);

const notes = values.filter((record) => record.note);
if (notes.length) {
  process.stdout.write("\nNotes\n");
  for (const record of notes) {
    process.stdout.write(`- #${record.candidate.sequence} seed ${record.candidate.seed} [${record.verdict}]: ${record.note}\n`);
  }
}
