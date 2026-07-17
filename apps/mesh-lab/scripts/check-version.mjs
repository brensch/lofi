import fs from "node:fs";

const appSource = fs.readFileSync(new URL("../src/version.ts", import.meta.url), "utf8");
const workletSource = fs.readFileSync(
  new URL("../src/audio/mesh-worklet.js", import.meta.url),
  "utf8",
);
const appVersion = appSource.match(/APP_VERSION = "([^"]+)"/)?.[1];
const workletVersion = workletSource.match(/WORKLET_VERSION = "([^"]+)"/)?.[1];

if (!appVersion || appVersion !== workletVersion) {
  throw new Error(`version mismatch: app=${appVersion ?? "missing"}, worklet=${workletVersion ?? "missing"}`);
}

process.stdout.write(`release ${appVersion}\n`);
