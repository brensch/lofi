import { copyFileSync, mkdirSync } from "node:fs";
import { fileURLToPath } from "node:url";
import { dirname, resolve } from "node:path";
import { execFileSync } from "node:child_process";

const appRoot = resolve(dirname(fileURLToPath(import.meta.url)), "..");
const repoRoot = resolve(appRoot, "../..");
const wasmSource = resolve(repoRoot, "target/wasm32-unknown-unknown/release/lofi_web.wasm");
const publicDir = resolve(appRoot, "public");
const wasmTarget = resolve(publicDir, "lofi_web.wasm");

execFileSync(
  "cargo",
  ["build", "-p", "lofi-web", "--target", "wasm32-unknown-unknown", "--release"],
  { cwd: repoRoot, stdio: "inherit" },
);
mkdirSync(publicDir, { recursive: true });
copyFileSync(wasmSource, wasmTarget);
