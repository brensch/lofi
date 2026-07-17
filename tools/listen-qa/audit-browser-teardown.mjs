#!/usr/bin/env node

import { spawn } from "node:child_process";
import { mkdtempSync, rmSync } from "node:fs";
import os from "node:os";
import path from "node:path";

const targetUrl = process.argv[2] ?? "http://127.0.0.1:5173/judge";
const replayCount = Number(process.argv[3] ?? 3);
const port = 9333;
const profile = mkdtempSync(path.join(os.tmpdir(), "lofi-chrome-audit-"));
const chrome = spawn("google-chrome", [
  "--headless=new",
  "--no-sandbox",
  "--disable-gpu",
  "--autoplay-policy=no-user-gesture-required",
  `--remote-debugging-port=${port}`,
  `--user-data-dir=${profile}`,
  "about:blank",
], { stdio: "ignore" });

try {
  const page = await waitForPage(port);
  const client = await connect(page.webSocketDebuggerUrl);
  await client.send("Page.enable");
  await client.send("Runtime.enable");
  await client.send("Page.addScriptToEvaluateOnNewDocument", { source: createAuditSource() });
  await client.send("Page.navigate", { url: targetUrl });
  await waitFor(client, `document.querySelector(".start-candidate")`);

  const runs = [];
  for (let index = 0; index < replayCount; index += 1) {
    const selector = index === 0 ? ".start-candidate" : ".replay-button";
    await evaluate(client, `document.querySelector("${selector}").click()`);
    await waitFor(client, `document.querySelector(".transport-status")?.innerText.includes("Candidate playing")`);
    await waitFor(client, `document.querySelector(".transport-status")?.innerText.includes("Ready to judge")`, 40_000);
    await delay(300);
    runs.push(await evaluate(client, `structuredClone(window.__lofiAudit)`));
  }

  const version = await evaluate(client, `document.querySelector(".brand-name small")?.innerText`);
  await client.send("Page.navigate", { url: "about:blank" });
  await delay(500);
  const afterNavigation = await evaluate(client, `structuredClone(window.__lofiAudit ?? {})`);
  client.close();

  const finalRun = runs.at(-1);
  const passed = finalRun.created === replayCount
    && finalRun.disposeMessages === replayCount
    && finalRun.disconnectCalls === replayCount
    && finalRun.contextsSuspended === replayCount;
  process.stdout.write(`${JSON.stringify({ version, runs, afterNavigation, passed }, null, 2)}\n`);
  if (!passed) process.exitCode = 1;
} finally {
  chrome.kill("SIGTERM");
  await new Promise((resolve) => chrome.once("exit", resolve));
  rmSync(profile, { force: true, recursive: true });
}

function createAuditSource() {
  return `
(() => {
  const audit = window.__lofiAudit = {
    contextsClosed: 0,
    contextsCreated: 0,
    contextsResumed: 0,
    contextsSuspended: 0,
    created: 0,
    disconnectCalls: 0,
    disposeMessages: 0,
  };
  const NativeContext = window.AudioContext;
  window.AudioContext = class extends NativeContext {
    constructor(...args) {
      super(...args);
      audit.contextsCreated += 1;
    }
    close() {
      audit.contextsClosed += 1;
      return super.close();
    }
    resume() {
      audit.contextsResumed += 1;
      return super.resume();
    }
    suspend() {
      audit.contextsSuspended += 1;
      return super.suspend();
    }
  };
  const NativeWorkletNode = window.AudioWorkletNode;
  function AuditedWorkletNode(...args) {
    const node = new NativeWorkletNode(...args);
    audit.created += 1;
    const nativeDisconnect = node.disconnect.bind(node);
    let disconnected = false;
    node.disconnect = (...disconnectArgs) => {
      if (!disconnected) {
        disconnected = true;
        audit.disconnectCalls += 1;
      }
      return nativeDisconnect(...disconnectArgs);
    };
    const nativePostMessage = node.port.postMessage.bind(node.port);
    let disposed = false;
    node.port.postMessage = (message, ...messageArgs) => {
      if (message?.type === "dispose" && !disposed) {
        disposed = true;
        audit.disposeMessages += 1;
      }
      return nativePostMessage(message, ...messageArgs);
    };
    return node;
  }
  AuditedWorkletNode.prototype = NativeWorkletNode.prototype;
  window.AudioWorkletNode = AuditedWorkletNode;
})();
`;
}

async function waitForPage(debugPort) {
  const endpoint = `http://127.0.0.1:${debugPort}/json/list`;
  for (let attempt = 0; attempt < 100; attempt += 1) {
    try {
      const pages = await (await fetch(endpoint)).json();
      const page = pages.find((item) => item.type === "page");
      if (page) return page;
    } catch {}
    await delay(50);
  }
  throw new Error("Chrome DevTools endpoint did not start");
}

async function connect(url) {
  const socket = new WebSocket(url);
  await new Promise((resolve, reject) => {
    socket.onopen = resolve;
    socket.onerror = reject;
  });
  let requestId = 0;
  const pending = new Map();
  socket.onmessage = (event) => {
    const message = JSON.parse(event.data);
    if (!message.id || !pending.has(message.id)) return;
    const { resolve, reject } = pending.get(message.id);
    pending.delete(message.id);
    if (message.error) reject(new Error(message.error.message));
    else resolve(message.result);
  };
  return {
    close: () => socket.close(),
    send: (method, params = {}) => new Promise((resolve, reject) => {
      const id = ++requestId;
      pending.set(id, { reject, resolve });
      socket.send(JSON.stringify({ id, method, params }));
    }),
  };
}

async function evaluate(client, expression) {
  const response = await client.send("Runtime.evaluate", {
    awaitPromise: true,
    expression,
    returnByValue: true,
  });
  if (response.exceptionDetails) throw new Error(response.exceptionDetails.text);
  return response.result.value;
}

async function waitFor(client, expression, timeoutMs = 10_000) {
  const deadline = Date.now() + timeoutMs;
  while (Date.now() < deadline) {
    if (await evaluate(client, `Boolean(${expression})`)) return;
    await delay(100);
  }
  throw new Error(`timed out waiting for ${expression}`);
}

function delay(milliseconds) {
  return new Promise((resolve) => setTimeout(resolve, milliseconds));
}
