#!/usr/bin/env node
import { spawnSync } from "node:child_process";
import { request } from "node:https";

const [ownerRepo = "laoguo2025/wridian", branch = "master"] = process.argv.slice(2);
const apiBase = "https://api.github.com";

function run(command, args, options = {}) {
  const result = spawnSync(command, args, {
    encoding: options.encoding ?? "buffer",
    maxBuffer: 256 * 1024 * 1024,
    ...options,
  });
  if (result.status !== 0) {
    throw new Error(`${command} ${args.join(" ")} failed\n${String(result.stderr || "")}`);
  }
  return result.stdout;
}

function text(command, args) {
  return run(command, args, { encoding: "utf8" }).trim();
}

function authToken() {
  if (process.env.GH_TOKEN) return process.env.GH_TOKEN.trim();
  return text("gh", ["auth", "token"]);
}

const token = authToken();

function api(method, path, body, attempt = 1) {
  const payload = body === undefined ? undefined : JSON.stringify(body);
  return new Promise((resolve, reject) => {
    const req = request(`${apiBase}${path}`, {
      method,
      headers: {
        Accept: "application/vnd.github+json",
        Authorization: `Bearer ${token}`,
        "Content-Type": "application/json",
        "User-Agent": "wridian-github-tree-sync",
        "X-GitHub-Api-Version": "2022-11-28",
      },
    }, (res) => {
      const chunks = [];
      res.on("data", (chunk) => chunks.push(chunk));
      res.on("end", () => {
        const raw = Buffer.concat(chunks).toString("utf8");
        const parsed = raw ? JSON.parse(raw) : null;
        if (res.statusCode >= 200 && res.statusCode < 300) {
          resolve(parsed);
          return;
        }
        const retryable = [401, 403, 429, 500, 502, 503, 504].includes(res.statusCode);
        if (retryable && attempt < 6) {
          const delay = 1500 * attempt;
          setTimeout(() => {
            api(method, path, body, attempt + 1).then(resolve, reject);
          }, delay);
          return;
        }
        reject(new Error(`${method} ${path} failed: ${res.statusCode}\n${raw}`));
      });
    });
    req.on("error", (error) => {
      if (attempt < 6) {
        const delay = 1500 * attempt;
        setTimeout(() => {
          api(method, path, body, attempt + 1).then(resolve, reject);
        }, delay);
        return;
      }
      reject(error);
    });
    if (payload) req.write(payload);
    req.end();
  });
}

function localTreeEntries() {
  const output = run("git", ["ls-tree", "-r", "-z", "HEAD"]).toString("utf8");
  return output.split("\0").filter(Boolean).map((line) => {
    const tab = line.indexOf("\t");
    const meta = line.slice(0, tab).split(" ");
    return {
      mode: meta[0],
      type: meta[1],
      sha: meta[2],
      path: line.slice(tab + 1),
    };
  });
}

function gitBlobBase64(sha) {
  return run("git", ["cat-file", "-p", sha]).toString("base64");
}

const localTreeSha = text("git", ["show", "-s", "--format=%T", "HEAD"]);
const branchInfo = await api("GET", `/repos/${ownerRepo}/branches/${branch}`);
const remoteSha = branchInfo.commit.sha;
const remoteCommit = await api("GET", `/repos/${ownerRepo}/git/commits/${remoteSha}`);
const remoteTreeSha = remoteCommit.tree.sha;
const remoteTree = await api("GET", `/repos/${ownerRepo}/git/trees/${remoteTreeSha}?recursive=1`);
const remoteMap = new Map(remoteTree.tree.filter((entry) => entry.type === "blob").map((entry) => [entry.path, entry.sha]));
const entries = localTreeEntries();
const localMap = new Map(entries.map((entry) => [entry.path, entry.sha]));
const tree = [];
let uploaded = 0;
let reused = 0;
let deleted = 0;

for (const entry of entries) {
  if (remoteMap.get(entry.path) === entry.sha) {
    reused += 1;
    continue;
  }
  process.stderr.write(`upload ${entry.path}\n`);
  const blob = await api("POST", `/repos/${ownerRepo}/git/blobs`, {
    content: gitBlobBase64(entry.sha),
    encoding: "base64",
  });
  if (blob.sha !== entry.sha) {
    throw new Error(`blob sha mismatch for ${entry.path}: local ${entry.sha}, remote ${blob.sha}`);
  }
  tree.push({ path: entry.path, mode: entry.mode, type: "blob", sha: blob.sha });
  uploaded += 1;
}

for (const [remotePath] of remoteMap) {
  if (!localMap.has(remotePath)) {
    tree.push({ path: remotePath, mode: "100644", type: "blob", sha: null });
    deleted += 1;
  }
}

if (!tree.length && remoteTreeSha === localTreeSha) {
  console.log(JSON.stringify({ status: "noop", remoteSha, treeSha: remoteTreeSha, uploaded, reused, deleted }, null, 2));
  process.exit(0);
}

const newTree = await api("POST", `/repos/${ownerRepo}/git/trees`, { base_tree: remoteTreeSha, tree });
if (newTree.sha !== localTreeSha) {
  throw new Error(`new remote tree ${newTree.sha} does not match local HEAD tree ${localTreeSha}`);
}

const commit = await api("POST", `/repos/${ownerRepo}/git/commits`, {
  message: `Sync Wridian ${text("node", ["-p", "require('./package.json').version"])} release`,
  tree: newTree.sha,
  parents: [remoteSha],
});
await api("PATCH", `/repos/${ownerRepo}/git/refs/heads/${branch}`, { sha: commit.sha, force: false });

console.log(JSON.stringify({
  status: "updated",
  previousRemoteSha: remoteSha,
  newCommitSha: commit.sha,
  treeSha: newTree.sha,
  uploaded,
  reused,
  deleted,
}, null, 2));
