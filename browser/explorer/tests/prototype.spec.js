import { test, expect } from "@playwright/test";
import fs from "node:fs";
import { createHash } from "node:crypto";
const cases = JSON.parse(fs.readFileSync(".cache/native-cases.json", "utf8"));
async function ready(page) {
  await page.goto("./");
  await expect(page.locator("#run")).toBeEnabled();
  await expect(page.locator("#graph")).not.toHaveValue("");
}
function verify(request, result) {
  const g = JSON.parse(JSON.parse(request).graph_json),
    r = result.record,
    a = r.assignment,
    k = r.controls.k,
    n = g.xadj.length - 1;
  expect(r.supplied_graph_utf8_sha256).toBe(
    createHash("sha256")
      .update(JSON.parse(request).graph_json, "utf8")
      .digest("hex"),
  );
  expect(a).toHaveLength(n);
  expect(a.every((p) => Number.isInteger(p) && p >= 0 && p < k)).toBe(true);
  const count = Array(k).fill(0),
    weights = Array(k).fill(0);
  let cut = 0;
  for (let v = 0; v < n; v++) {
    count[a[v]]++;
    weights[a[v]] += g.vwgt?.length ? g.vwgt[v] : 1;
    for (let j = g.xadj[v]; j < g.xadj[v + 1]; j++)
      if (a[v] !== a[g.adjncy[j]]) cut += g.adjwgt?.length ? g.adjwgt[j] : 1;
  }
  expect(count.every((n) => n > 0)).toBe(true);
  expect(r.part_vertex_counts).toEqual(count);
  expect(r.part_weights).toEqual(weights);
  expect(r.total_vertex_weight).toBe(weights.reduce((a, b) => a + b, 0));
  expect(r.edge_cut).toBe(cut / 2);
  expect(r.native_partition_validation).toBe("passed");
  if (r.controls.contiguity_requested)
    expect(r.all_parts_contiguous).toBe(true);
}
test("measure both targets honestly and validate every repeated WASM result", async ({
  page,
}) => {
  await ready(page);
  const outputs = await page.evaluate(async (cases) => {
    const m = await import("./pkg/metis_partition_explorer.js");
    await m.default();
    return cases.map((c) => {
      const first = m.partition_request(c.request),
        second = m.partition_request(c.request);
      return { first, second };
    });
  }, cases);
  const records = [];
  for (let i = 0; i < cases.length; i++) {
    const c = cases[i],
      w = JSON.parse(outputs[i].first),
      n = JSON.parse(c.native_output);
    expect(outputs[i].first, c.label + " repeat").toBe(outputs[i].second);
    expect(w.target_pointer_bits).toBe(32);
    expect(n.target_pointer_bits).toBe(64);
    expect(w.result, c.label + " class").toBe(n.result);
    if (w.result === "ok") {
      verify(c.request, w);
      verify(c.request, n);
    }
    const nw = { ...n, target_pointer_bits: 32 };
    records.push({
      label: c.label,
      request: c.request,
      native: n,
      wasm: w,
      equal_except_target: JSON.stringify(nw) === JSON.stringify(w),
      same_assignment:
        w.result === "ok"
          ? JSON.stringify(n.record.assignment) ===
            JSON.stringify(w.record.assignment)
          : null,
      same_cut:
        w.result === "ok" ? n.record.edge_cut === w.record.edge_cut : null,
    });
  }
  fs.writeFileSync(
    ".cache/wasm-cases.json",
    JSON.stringify(
      cases.map((c, i) => ({
        label: c.label,
        request: c.request,
        output: outputs[i].first,
      })),
      null,
      2,
    ),
  );
  const proof = {
    scope:
      "Full observed native64 and WASM32 outcomes; repeated target-specific determinism and independently checked structure/cut/weight metrics. No cross-target or gpmetis quality guarantee.",
    cases: records.length,
    equal_except_target: records.filter((r) => r.equal_except_target).length,
    assignment_differences: records.filter((r) => r.same_assignment === false)
      .length,
    cut_differences: records.filter((r) => r.same_cut === false).length,
    records,
  };
  fs.writeFileSync(".cache/cross-target.json", JSON.stringify(proof, null, 2));
  console.log(
    JSON.stringify({
      cases: proof.cases,
      equal_except_target: proof.equal_except_target,
      assignment_differences: proof.assignment_differences,
      cut_differences: proof.cut_differences,
    }),
  );
});
test("real worker supports graph controls and renders exact native-derived data", async ({
  page,
}) => {
  await ready(page);
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("completed");
  const report = JSON.parse(await page.locator("#report").textContent());
  verify(
    JSON.stringify({ graph_json: await page.locator("#graph").inputValue() }),
    report,
  );
  await expect(page.locator("#visual svg circle")).toHaveCount(36);
  await expect(page.locator("#metrics")).toContainText(
    String(report.record.edge_cut),
  );
  await page.locator("#mode").selectOption("recursive");
  await expect(page.locator("#ncuts")).toHaveValue("4");
  await page.locator("#parts").fill("3");
  await page.locator("#seed").fill("18446744073709551615");
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("completed");
  expect(
    JSON.parse(await page.locator("#report").textContent()).record.controls
      .seed,
  ).toBe("18446744073709551615");
  const metrics = await page
    .locator("#status")
    .evaluate((n) => ({
      measurements: { ...n.dataset },
      user_agent: navigator.userAgent,
      hardware_concurrency: navigator.hardwareConcurrency,
      scope:
        "Worker/query observations include JS/message/serialization overhead; no comparative speed claim.",
    }));
  fs.writeFileSync(
    ".cache/worker-timings.json",
    JSON.stringify(metrics, null, 2),
  );
});
test("native CSR errors retain input and clear stale visual/report exports", async ({
  page,
}) => {
  await ready(page);
  await page.locator("#run").click();
  await expect(page.locator("#download")).toBeEnabled();
  const bad = '{"xadj":[0,1,1],"adjncy":[1]}';
  await page.locator("#graph").fill(bad);
  await expect(page.locator("#download")).toBeDisabled();
  await expect(page.locator("#visual svg")).toHaveCount(0);
  await page.locator("#run").click();
  await expect(page.locator("#status")).toContainText("native_error");
  expect(await page.locator("#graph").inputValue()).toBe(bad);
  await expect(page.locator("#visual svg")).toHaveCount(0);
  expect(
    JSON.parse(await page.locator("#report").textContent()).record,
  ).toBeNull();
});
test("own original and request downloads plus mobile keyboard path", async ({
  page,
}) => {
  await page.setViewportSize({ width: 390, height: 844 });
  await ready(page);
  const source = await page.locator("#graph").inputValue(),
    bytes = Buffer.from("\ufeff" + source.replaceAll("\n", "\r\n"));
  await page
    .locator("#file")
    .setInputFiles({
      name: "own.json",
      mimeType: "application/json",
      buffer: bytes,
    });
  await expect(page.locator("#original")).toBeEnabled();
  const event = page.waitForEvent("download");
  await page.locator("#original").click();
  expect(fs.readFileSync(await (await event).path())).toEqual(bytes);
  await page.locator("#lf").click();
  await page.locator("#seed").focus();
  await page.locator("#seed").press("Enter");
  await expect(page.locator("#status")).toContainText("completed");
  expect(
    await page.evaluate(
      () => document.documentElement.scrollWidth <= innerWidth,
    ),
  ).toBe(true);
  await page.screenshot({ path: ".cache/mobile.png", fullPage: true });
  await page.setViewportSize({ width: 1280, height: 900 });
  await page.screenshot({ path: ".cache/desktop.png", fullPage: true });
  const request = page.waitForEvent("download");
  await page.locator("#request-download").click();
  const body = JSON.parse(
    fs.readFileSync(await (await request).path(), "utf8"),
  );
  expect(body.graph_json).toBe(await page.locator("#graph").inputValue());
  expect(body.seed).toBe("7");
});
