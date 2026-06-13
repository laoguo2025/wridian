import { mkdir, readFile, writeFile } from "node:fs/promises";
import { existsSync } from "node:fs";
import path from "node:path";

const debugPort = Number(process.env.WRIDIAN_E2E_DEBUG_PORT || 9222);
const artifactDir = process.env.WRIDIAN_E2E_ARTIFACT_DIR
  || path.resolve(".workbench/runtime/e2e-artifacts");

async function main() {
  const playwright = await loadPlaywright();
  await mkdir(artifactDir, { recursive: true });
  const browser = await playwright.chromium.connectOverCDP(`http://127.0.0.1:${debugPort}`);
  const context = browser.contexts()[0] || await browser.newContext();
  const page = context.pages()[0] || await context.newPage();
  await page.waitForLoadState("domcontentloaded");
  await page.waitForFunction(() => Boolean(window.__WRIDIAN_E2E__), null, { timeout: 15_000 });

  const fixture = await page.evaluate(() => window.__WRIDIAN_E2E__.prepareFixture(true));
  await page.evaluate((firstDraftPath) => window.__WRIDIAN_E2E__.openFile(firstDraftPath, "works"), fixture.firstDraftPath);
  await page.getByRole("heading", { name: "第1集.md" }).waitFor({ timeout: 10_000 });

  await testConversationDrivenFileTreeEditing(page, fixture, "works");
  await testConversationDrivenFileTreeEditing(page, fixture, "knowledge");
  await testFakeSavedNewEpisodeFallback(page, fixture);

  await page.evaluate(() => window.__WRIDIAN_E2E__.setNextCocreation(JSON.stringify({
    reply: "| 风险点 | 处理 |\n| --- | --- |\n| 车站警告 | 保留悬念 |\n| 第二班车 | 延后揭示 |\n| 广播人 | 作为对手线索 |",
    edits: [],
    fileOperations: [],
    memories: [],
  })));
  await page.evaluate(() => {
    window.__WRIDIAN_E2E__.setPrompt("请用 Markdown 表格列出第1集的三个风险点");
  });
  await page.getByRole("button", { name: "发送" }).click();
  await page.getByText("请用 Markdown 表格列出第1集的三个风险点").last().waitFor({ timeout: 5_000 });
  await page.locator(".chat-markdown-table-wrap table").first().waitFor({ timeout: 10_000 });

  await testSelectionToPromptAndSend(page);

  await page.evaluate(() => window.__WRIDIAN_E2E__.setNextCocreation(JSON.stringify({
    reply: "已生成 1 处待确认正文修改。",
    edits: [{ target: "主角", replacement: "女主角", rationale: "按用户字面替换" }],
    fileOperations: [],
    memories: [],
  })));
  await page.evaluate(() => window.__WRIDIAN_E2E__.sendPrompt("把 主角 都改成 女主角"));
  await page.locator(".inline-diff del").first().waitFor({ timeout: 10_000 });
  await page.locator(".inline-diff ins").first().waitFor({ timeout: 10_000 });

  const state = await page.evaluate(() => window.__WRIDIAN_E2E__.getState());
  const screenshotPath = path.join(artifactDir, "wridian-e2e-smoke.png");
  await page.screenshot({ path: screenshotPath, fullPage: false });
  await writeFile(
    path.join(artifactDir, "wridian-e2e-smoke.json"),
    JSON.stringify({ fixture, state, screenshotPath }, null, 2),
    "utf8",
  );
  await browser.close();
  console.log(JSON.stringify({ ok: true, fixture, screenshotPath }, null, 2));
}

async function testFileTreeEditing(page, fixture) {
  const folderName = `E2E文件夹-${Date.now()}`;
  const fileName = "新建测试.md";
  const renamedFileName = "改名测试.md";
  const folderPath = path.join(fixture.worksRoot, folderName);
  const filePath = path.join(folderPath, fileName);
  const renamedPath = path.join(folderPath, renamedFileName);

  page.once("dialog", async (dialog) => {
    if (dialog.type() !== "prompt") throw new Error(`Unexpected dialog: ${dialog.type()}`);
    await dialog.accept(folderName);
  });
  await page.getByRole("button", { name: "新建文件夹" }).click();
  await page.getByRole("button", { name: folderName }).waitFor({ timeout: 10_000 });
  if (!existsSync(folderPath)) throw new Error(`Folder was not created: ${folderPath}`);

  await page.getByRole("button", { name: folderName }).click({ button: "right" });
  const contextMenu = page.locator(".context-menu");
  await contextMenu.getByRole("button", { name: "新建文件", exact: true }).waitFor({ timeout: 5_000 });
  page.once("dialog", async (dialog) => {
    if (dialog.type() !== "prompt") throw new Error(`Unexpected dialog: ${dialog.type()}`);
    await dialog.accept(fileName);
  });
  await contextMenu.getByRole("button", { name: "新建文件", exact: true }).click();
  await page.getByRole("button", { name: /^新建测试/i }).waitFor({ timeout: 10_000 });
  if (!existsSync(filePath)) throw new Error(`File was not created: ${filePath}`);

  await page.getByRole("button", { name: /^新建测试/i }).click({ button: "right" });
  await contextMenu.getByRole("button", { name: "重命名", exact: true }).waitFor({ timeout: 5_000 });
  page.once("dialog", async (dialog) => {
    if (dialog.type() !== "prompt") throw new Error(`Unexpected dialog: ${dialog.type()}`);
    await dialog.accept(renamedFileName);
  });
  await contextMenu.getByRole("button", { name: "重命名", exact: true }).click();
  await page.getByRole("button", { name: /^改名测试/i }).waitFor({ timeout: 10_000 });
  if (!existsSync(renamedPath)) throw new Error(`File was not renamed: ${renamedPath}`);

  await page.getByRole("button", { name: /^改名测试/i }).click({ button: "right" });
  await contextMenu.getByRole("button", { name: "移到回收站", exact: true }).waitFor({ timeout: 5_000 });
  await contextMenu.getByRole("button", { name: "移到回收站", exact: true }).click();
  await page.waitForFunction((targetPath) => !window.__WRIDIAN_E2E__.getState().workspace.files
    .flatMap(function flatten(node) { return [node, ...node.children.flatMap(flatten)]; })
    .some((node) => node.path === targetPath), renamedPath, { timeout: 10_000 });
  if (existsSync(renamedPath)) throw new Error(`File was not moved to trash: ${renamedPath}`);
}

async function testConversationDrivenFileTreeEditing(page, fixture, library) {
  const isKnowledge = library === "knowledge";
  const root = isKnowledge ? fixture.knowledgeRoot : fixture.worksRoot;
  const treeStateKey = isKnowledge ? "knowledgeFiles" : "files";
  const libraryLabel = isKnowledge ? "知识库" : "作品库";
  const stamp = Date.now();
  const folderName = `${isKnowledge ? "知识" : "作品"}对话测试-${stamp}`;
  const fileName = "对话新建.md";
  const renamedName = "对话改名.md";
  const folderRel = folderName;
  const fileRel = `${folderName}/${fileName}`;
  const renamedRel = `${folderName}/${renamedName}`;
  const folderPath = path.join(root, folderName);
  const filePath = path.join(root, folderName, fileName);
  const renamedPath = path.join(root, folderName, renamedName);

  if (isKnowledge) {
    await page.getByRole("button", { name: "知识库", exact: true }).click();
  } else {
    await page.getByRole("button", { name: "作品库", exact: true }).click();
  }

  await runMockedPrompt(page, {
    text: `在${libraryLabel}创建文件夹 ${folderName}`,
    response: {
      reply: `已在${libraryLabel}创建文件夹。`,
      edits: [],
      fileOperations: [{ action: "createFolder", library, path: folderRel }],
      memories: [],
    },
  });
  await waitForTreePath(page, treeStateKey, folderPath);
  await page.getByRole("button", { name: folderName }).waitFor({ timeout: 10_000 });
  if (!existsSync(folderPath)) throw new Error(`${libraryLabel} folder was not created by chat: ${folderPath}`);

  await runMockedPrompt(page, {
    text: `在${libraryLabel}的 ${folderName} 里新建 ${fileName}`,
    response: {
      reply: `已在${libraryLabel}写入新文件。`,
      edits: [],
      fileOperations: [{ action: "writeFile", library, path: fileRel, content: `# ${fileName}\n\n来自对话的${libraryLabel}文件。` }],
      memories: [],
    },
  });
  await waitForTreePath(page, treeStateKey, filePath);
  await page.getByRole("button", { name: /^对话新建/i }).waitFor({ timeout: 10_000 });
  if (!existsSync(filePath)) throw new Error(`${libraryLabel} file was not created by chat: ${filePath}`);

  await runMockedPrompt(page, {
    text: `把${libraryLabel}里的 ${fileName} 重命名为 ${renamedName}`,
    response: {
      reply: `已重命名${libraryLabel}文件。`,
      edits: [],
      fileOperations: [{ action: "rename", library, path: fileRel, newName: renamedName }],
      memories: [],
    },
  });
  await waitForTreePath(page, treeStateKey, renamedPath);
  await page.getByRole("button", { name: /^对话改名/i }).waitFor({ timeout: 10_000 });
  if (!existsSync(renamedPath)) throw new Error(`${libraryLabel} file was not renamed by chat: ${renamedPath}`);

  await runMockedPrompt(page, {
    text: `把${libraryLabel}里的 ${renamedName} 移到回收站`,
    response: {
      reply: `已移到回收站。`,
      edits: [],
      fileOperations: [{ action: "trash", library, path: renamedRel }],
      memories: [],
    },
  });
  await waitForTreePathGone(page, treeStateKey, renamedPath);
  if (existsSync(renamedPath)) throw new Error(`${libraryLabel} file was not trashed by chat: ${renamedPath}`);
  await page.getByText(`${libraryLabel} / ${renamedRel}`).waitFor({ timeout: 10_000 });
}

async function testFakeSavedNewEpisodeFallback(page, fixture) {
  await page.getByRole("button", { name: "作品库", exact: true }).click();
  const targetPath = path.join(fixture.worksRoot, "测试", "第2集.md");
  const before = await page.evaluate(() => window.__WRIDIAN_E2E__.getState().editorContent);
  await runMockedPrompt(page, {
    text: "根据第1集剧情，续写第2集，在作品库里新建个文档保存",
    response: {
      reply: "已根据第1集剧情续写第2集，新建 `works/第2集.docx` 保存。\n\n## 第2集\n\n主角走进新的冲突，车站广播再次响起。",
      edits: [{
        target: "主角",
        replacement: "## 第2集\n\n主角走进新的冲突，车站广播再次响起。",
        rationale: "错误地把新文档内容当成当前正文修改",
      }],
      fileOperations: [],
      memories: [{ branch: "drama", title: "第2集", summary: "不应在 fallback 前写入记忆", reason: "测试", sourcePath: "第1集.md" }],
    },
  });
  await waitForTreePath(page, "files", targetPath);
  await page.getByRole("button", { name: /^第2集/i }).waitFor({ timeout: 10_000 });

  const after = await page.evaluate(() => window.__WRIDIAN_E2E__.getState().editorContent);
  if (after !== before) {
    throw new Error("New episode fallback changed the currently opened draft");
  }
  const pendingEdits = await page.evaluate(() => window.__WRIDIAN_E2E__.getState().pendingEdits.length);
  if (pendingEdits !== 0) {
    throw new Error(`New episode fallback created pending draft edits: ${pendingEdits}`);
  }
  const inlineDiffCount = await page.locator(".inline-diff del, .inline-diff ins").count();
  if (inlineDiffCount !== 0) {
    throw new Error(`New episode fallback rendered inline diff nodes: ${inlineDiffCount}`);
  }
  const content = await readFile(targetPath, "utf8");
  if (!content.includes("## 第2集") || !content.includes("车站广播再次响起")) {
    throw new Error(`New episode file content is incomplete: ${content}`);
  }
  if (content.includes("已根据第1集剧情续写第2集")) {
    throw new Error("New episode file kept the fake saved operation line");
  }
}

async function testSelectionToPromptAndSend(page) {
  await page.evaluate(() => {
    const editor = document.querySelector(".draft-editor");
    if (!editor) throw new Error("Draft editor not found");
    const walker = document.createTreeWalker(editor, NodeFilter.SHOW_TEXT);
    let node = walker.nextNode();
    while (node) {
      const text = node.textContent || "";
      const index = text.indexOf("主角");
      if (index >= 0) {
        const range = document.createRange();
        range.setStart(node, index);
        range.setEnd(node, index + "主角".length);
        const selection = window.getSelection();
        selection.removeAllRanges();
        selection.addRange(range);
        document.dispatchEvent(new Event("selectionchange"));
        editor.dispatchEvent(new MouseEvent("mouseup", { bubbles: true }));
        return;
      }
      node = walker.nextNode();
    }
    throw new Error("Selection target not found");
  });
  await page.getByRole("button", { name: "添加到对话" }).waitFor({ timeout: 10_000 });
  await page.getByRole("button", { name: "添加到对话" }).click();
  await page.locator(".prompt-attachment").filter({ hasText: "选区" }).waitFor({ timeout: 10_000 });
  await page.evaluate(() => window.__WRIDIAN_E2E__.setNextCocreation(JSON.stringify({
    reply: "已读取选区：主角。",
    edits: [],
    fileOperations: [],
    memories: [],
  })));
  await page.evaluate(() => window.__WRIDIAN_E2E__.setPrompt("解释我刚才划选的词"));
  await page.getByRole("button", { name: "发送" }).click();
  await page.getByText("解释我刚才划选的词").last().waitFor({ timeout: 10_000 });
  await page.getByText("已读取选区：主角。").last().waitFor({ timeout: 10_000 });
}

async function runMockedPrompt(page, { text, response }) {
  await page.evaluate((output) => window.__WRIDIAN_E2E__.setNextCocreation(JSON.stringify(output)), response);
  await page.evaluate((promptText) => window.__WRIDIAN_E2E__.sendPrompt(promptText), text);
  await page.getByText(text).last().waitFor({ timeout: 10_000 });
}

async function waitForTreePath(page, treeStateKey, targetPath) {
  await page.waitForFunction(({ key, path: itemPath }) => {
    const bridge = window.__WRIDIAN_E2E__;
    const state = bridge?.getState?.();
    return Boolean(state?.workspace?.[key]
      ?.flatMap(function flatten(node) { return [node, ...node.children.flatMap(flatten)]; })
      .some((node) => node.path === itemPath));
  }, { key: treeStateKey, path: targetPath }, { timeout: 10_000 });
}

async function waitForTreePathGone(page, treeStateKey, targetPath) {
  await page.waitForFunction(({ key, path: itemPath }) => {
    const bridge = window.__WRIDIAN_E2E__;
    const state = bridge?.getState?.();
    return Boolean(state?.workspace) && !state.workspace[key]
      .flatMap(function flatten(node) { return [node, ...node.children.flatMap(flatten)]; })
      .some((node) => node.path === itemPath);
  }, { key: treeStateKey, path: targetPath }, { timeout: 10_000 });
}

async function loadPlaywright() {
  try {
    return await import("playwright");
  } catch {
    throw new Error("Playwright is not installed. Run `npm install --save-dev playwright` before `node scripts/e2e-smoke.mjs`.");
  }
}

main().catch((error) => {
  console.error(error instanceof Error ? error.stack || error.message : String(error));
  process.exit(1);
});
