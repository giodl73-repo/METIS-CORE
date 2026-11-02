import { test, expect } from "@playwright/test";
async function ready(page) {
  await page.goto("./");
  await expect(page.locator("#run")).toBeEnabled();
  await expect(page.locator("#graph")).not.toHaveValue("");
}
test("browser resource boundaries remain separate from native validation", async ({
  page,
}) => {
  await ready(page);
  const observed = await page.evaluate(async () => {
    const m = await import("./pkg/metis_partition_explorer.js");
    await m.default();
    const graph = (n, steps = 1) => {
      const xadj = [0],
        adjncy = [];
      for (let v = 0; v < n; v++) {
        const row = new Set();
        for (let s = 1; s <= steps; s++) {
          row.add((v + s) % n);
          row.add((v - s + n) % n);
        }
        adjncy.push(...[...row].sort((a, b) => a - b));
        xadj.push(adjncy.length);
      }
      return {
        xadj,
        adjncy,
        vwgt: Array(n).fill(1000),
        adjwgt: Array(adjncy.length).fill(1000),
      };
    };
    const run = (g, fields = {}) =>
      JSON.parse(
        m.partition_request(
          JSON.stringify({
            graph_json: JSON.stringify(g),
            k: 4,
            seed: "7",
            ...fields,
          }),
        ),
      );
    const max = graph(256, 8),
      over = graph(257),
      arc = structuredClone(max);
    arc.adjncy.push(0);
    arc.adjwgt.push(1000);
    const weight = graph(8);
    weight.vwgt[0] = 1001;
    const negative = graph(8);
    negative.vwgt[0] = -1;
    return {
      max: run(max),
      vertex: run(over),
      arc: run(arc),
      weight: run(weight),
      negative: run(negative),
      trial: run(graph(8), { ncuts: 9 }),
      iteration: run(graph(8), { niter: 21 }),
      parts: run(graph(8), { k: 17 }),
      targets: run(graph(8), { target_weights: Array(17).fill(1 / 17) }),
      frame: JSON.parse(m.partition_request(" ".repeat(3 * 1024 * 1024 + 1))),
    };
  });
  expect(observed.max.result).toBe("ok");
  expect(observed.max.record.vertices).toBe(256);
  expect(observed.max.record.adjacency_entries).toBe(4096);
  expect(observed.max.record.total_vertex_weight).toBe(256000);
  for (const name of [
    "vertex",
    "arc",
    "weight",
    "trial",
    "iteration",
    "parts",
    "targets",
    "frame",
  ]) {
    expect(observed[name].result, name).toBe("browser_refusal");
    expect(observed[name].record).toBeNull();
  }
  expect(observed.negative.result).toBe("native_error");
  expect(observed.negative.error.variant).toBe("NonPositiveVertexWeight");
  expect(
    await page.evaluate(async () => {
      const m = await import("./pkg/metis_partition_explorer.js"),
        w = await m.default();
      try {
        w.memory.grow(1024);
        return false;
      } catch (e) {
        return e instanceof RangeError;
      }
    }),
  ).toBe(true);
});
test("target weights and invalid controls keep native meaning and stale state explicit", async ({
  page,
}) => {
  await ready(page);
  await page.locator("#parts").fill("2");
  await page.locator("#targets").fill("[0.25,0.75]");
  await page.locator("#contiguity").check();
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("completed");
  let report = JSON.parse(await page.locator("#report").textContent());
  expect(report.record.controls.target_weights).toEqual([0.25, 0.75]);
  expect(report.record.controls.contiguity_requested).toBe(true);
  expect(report.record.all_parts_contiguous).toBe(true);
  await page.locator("#targets").fill("[0.5,0.4]");
  await expect(page.locator("#status")).toContainText("Inputs changed");
  await expect(page.locator("#visual svg")).toHaveCount(0);
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("native_error");
  report = JSON.parse(await page.locator("#report").textContent());
  expect(report.error.variant).toBe("TargetWeightsDoNotSum");
  await page.locator("#targets").fill("{");
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("Target weights must");
  await expect(page.locator("#download")).toBeDisabled();
  await page.locator("#targets").fill("null");
  await page.locator("#seed").fill("18446744073709551616");
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("invalid_request");
  expect(
    JSON.parse(await page.locator("#report").textContent()).record,
  ).toBeNull();
});
test("graph JSON diagnostics stay inert and do not become executable markup", async ({
  page,
}) => {
  await ready(page);
  const graph = JSON.parse(await page.locator("#graph").inputValue());
  graph["<img src=x onerror=alert(1)>"] = "unused";
  await page.locator("#graph").fill(JSON.stringify(graph));
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("invalid_graph_json");
  await expect(page.locator("#summary")).toContainText("<img src=x");
  expect(await page.locator("img").count()).toBe(0);
  await expect(page.locator("#visual svg")).toHaveCount(0);
});
