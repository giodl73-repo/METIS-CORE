import { test, expect } from "@playwright/test";
import fs from "node:fs";
const cases = JSON.parse(fs.readFileSync(".cache/native-cases.json", "utf8")),
  example = JSON.parse(JSON.parse(cases[0].request).graph_json);
// Synthetic lifecycle fault only; never a partition result or WASM golden oracle.
const staleFault = JSON.stringify({
  schema_version: "metis.browser-partition.v1",
  result: "invalid_request",
  error: "Injected stale response",
  record: null,
  target_pointer_bits: 32,
});
async function ready(page) {
  await page.goto("./");
  await expect(page.locator("#run")).toBeEnabled();
  await expect(page.locator("#graph")).not.toHaveValue("");
}
async function intercept(page) {
  await page.addInitScript(() => {
    const Actual = window.Worker;
    window.__workers = [];
    window.__sent = [];
    window.__hold = false;
    window.Worker = class {
      constructor(url, options) {
        this.actual = new Actual(url, options);
        this.actual.onmessage = (e) => this.onmessage?.(e);
        this.actual.onerror = (e) => this.onerror?.(e);
        window.__workers.push(this);
      }
      postMessage(data) {
        window.__sent.push(data);
        if (!window.__hold) this.actual.postMessage(data);
      }
      terminate() {
        this.terminated = true;
        this.actual.terminate();
      }
    };
  });
}
test("delayed reads cannot replace newer edits or selections; actual read timeout", async ({
  page,
}) => {
  await page.addInitScript(() => {
    const original = File.prototype.arrayBuffer;
    File.prototype.arrayBuffer = function () {
      if (this.name === "slow.json")
        return new Promise((resolve) =>
          setTimeout(() => original.call(this).then(resolve), 300),
        );
      if (this.name === "stuck.json") return new Promise(() => {});
      return original.call(this);
    };
  });
  await ready(page);
  const raw = JSON.stringify(example);
  await page.locator("#file").setInputFiles({
    name: "slow.json",
    mimeType: "application/json",
    buffer: Buffer.from(raw),
  });
  await page.locator("#graph").fill("{");
  await page.waitForTimeout(450);
  expect(await page.locator("#graph").inputValue()).toBe("{");
  await page.locator("#file").setInputFiles({
    name: "slow.json",
    mimeType: "application/json",
    buffer: Buffer.from(raw),
  });
  await page.locator("#file").setInputFiles({
    name: "new.json",
    mimeType: "application/json",
    buffer: Buffer.from(raw + " "),
  });
  await expect(page.locator("#status")).toContainText("Opened original");
  await page.waitForTimeout(450);
  expect(await page.locator("#graph").inputValue()).toBe(raw + " ");
  await page.locator("#file").setInputFiles({
    name: "stuck.json",
    mimeType: "application/json",
    buffer: Buffer.from("{}"),
  });
  await expect(page.locator("#status")).toContainText(
    "Read exceeded 5 seconds",
    { timeout: 6500 },
  );
  expect(await page.locator("#graph").inputValue()).toBe(raw + " ");
});
test("whole paste, edit, oversized file and dropped-file admission retain source", async ({
  page,
}) => {
  await ready(page);
  const accepted = await page.locator("#graph").inputValue();
  await page.locator("#file").setInputFiles({
    name: "large.json",
    mimeType: "application/json",
    buffer: Buffer.alloc(1048577, 32),
  });
  await expect(page.locator("#status")).toContainText("exceeds 1 MiB");
  expect(await page.locator("#graph").inputValue()).toBe(accepted);
  await page.locator("#file").setInputFiles({
    name: "invalid-utf8.json",
    mimeType: "application/json",
    buffer: Buffer.from([255]),
  });
  await expect(page.locator("#status")).toContainText("retained");
  expect(await page.locator("#graph").inputValue()).toBe(accepted);
  const prevented = await page.locator("#graph").evaluate((field) => {
    field.setSelectionRange(0, 0);
    const data = new DataTransfer();
    data.setData("text/plain", " ".repeat(1048576));
    const event = new ClipboardEvent("paste", {
      clipboardData: data,
      bubbles: true,
      cancelable: true,
    });
    field.dispatchEvent(event);
    return event.defaultPrevented;
  });
  expect(prevented).toBe(true);
  expect(await page.locator("#graph").inputValue()).toBe(accepted);
  await page.locator("#graph").evaluate((field) => {
    field.value = " ".repeat(1048577);
    field.dispatchEvent(new Event("input", { bubbles: true }));
  });
  expect(await page.locator("#graph").inputValue()).toBe(accepted);
  const raw = JSON.stringify(example);
  await page.locator("#graph").evaluate((field, raw) => {
    const transfer = new DataTransfer();
    transfer.items.add(
      new File([raw], "drop.json", { type: "application/json" }),
    );
    field.dispatchEvent(
      new DragEvent("drop", {
        dataTransfer: transfer,
        bubbles: true,
        cancelable: true,
      }),
    );
  }, raw);
  await expect(page.locator("#status")).toContainText("Opened original");
  expect(await page.locator("#graph").inputValue()).toBe(raw);
});
test("wrong IDs, stale revisions and terminated generations cannot publish", async ({
  page,
}) => {
  await intercept(page);
  await ready(page);
  await page.evaluate(() => (window.__hold = true));
  await page.locator("#run").click();
  await page.locator("#seed").fill("123");
  expect(await page.evaluate(() => window.__workers[0].terminated)).toBe(true);
  await expect(page.locator("#run")).toBeEnabled();
  await page.evaluate((result) => {
    const sent = window.__sent.at(-1);
    window.__workers.at(-1).onmessage({ data: { ...sent, result } });
  }, staleFault);
  await expect(page.locator("#download")).toBeDisabled();
  await page.locator("#run").click();
  await page.evaluate((result) => {
    const sent = window.__sent.at(-1),
      w = window.__workers.at(-1);
    w.onmessage({ data: { ...sent, id: "wrong", result } });
    w.onmessage({ data: { ...sent, revision: sent.revision - 1, result } });
  }, staleFault);
  await expect(page.locator("#download")).toBeDisabled();
  await page.locator("#restart").click();
  await expect(page.locator("#run")).toBeEnabled();
  await page.evaluate((result) => {
    const w = window.__workers[0],
      sent = window.__sent.at(-1);
    w.onmessage({ data: { ready: true } });
    w.onmessage({ data: { ...sent, result } });
    w.onerror({ message: "old crash" });
  }, staleFault);
  await expect(page.locator("#download")).toBeDisabled();
  await expect(page.locator("#status")).toContainText("Rust ready");
  expect(await page.locator("#seed").inputValue()).toBe("123");
  await page.evaluate(() => (window.__hold = false));
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("completed");
  await expect(page.locator("#summary")).toContainText(
    "Native partition validation passed",
  );
});
test("actual computation timeout and current worker crash retain inputs and clear exports", async ({
  page,
}) => {
  await intercept(page);
  await ready(page);
  await page.locator("#run").click();
  await expect(page.locator("#download")).toBeEnabled();
  await page.evaluate(() => (window.__hold = true));
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("exceeded 10 seconds", {
    timeout: 12000,
  });
  await expect(page.locator("#download")).toBeDisabled();
  await expect(page.locator("#request-download")).toBeDisabled();
  const source = await page.locator("#graph").inputValue();
  await page.evaluate(() => (window.__hold = false));
  await page.locator("#restart").click();
  await expect(page.locator("#run")).toBeEnabled();
  await page.locator("#run").click();
  await expect(page.locator("#download")).toBeEnabled();
  await page.evaluate(() =>
    window.__workers.at(-1).onerror({ message: "current crash" }),
  );
  await expect(page.locator("#status")).toContainText("Engine failed");
  expect(await page.locator("#graph").inputValue()).toBe(source);
  await expect(page.locator("#download")).toBeDisabled();
});
test("request download preserves graph and exact native controls", async ({
  page,
}) => {
  await ready(page);
  await page.locator("#seed").fill("18446744073709551615");
  await page.locator("#parts").fill("2");
  await page.locator("#targets").fill("[0.25,0.75]");
  await page.locator("#run").click();
  await expect(page.locator("#request-download")).toBeEnabled();
  const event = page.waitForEvent("download");
  await page.locator("#request-download").click();
  const request = JSON.parse(
    fs.readFileSync(await (await event).path(), "utf8"),
  );
  expect(request.graph_json).toBe(await page.locator("#graph").inputValue());
  expect(request.seed).toBe("18446744073709551615");
  expect(request.target_weights).toEqual([0.25, 0.75]);
  expect(request.k).toBe(2);
  expect(request.mode).toBe("kway");
  const exact = await page.evaluate(async (input) => {
    const m = await import("./pkg/metis_partition_explorer.js");
    await m.default();
    return m.partition_request(input);
  }, JSON.stringify(request));
  expect(await page.locator("#report").textContent()).toBe(exact);
});

test("actual startup deadline retains graph and restart recovers", async ({
  page,
}) => {
  test.setTimeout(30000);
  await page.route("**/worker.js", async (route) => {
    await new Promise((r) => setTimeout(r, 13500));
    try {
      await route.continue();
    } catch {}
  });
  await page.goto("./");
  await page.locator("#graph").fill("{");
  await expect(page.locator("#status")).toContainText("exceeded 12 seconds", {
    timeout: 15000,
  });
  expect(await page.locator("#graph").inputValue()).toBe("{");
  await page.unroute("**/worker.js");
  await page.locator("#restart").click();
  await expect(page.locator("#run")).toBeEnabled();
  expect(await page.locator("#graph").inputValue()).toBe("{");
});
