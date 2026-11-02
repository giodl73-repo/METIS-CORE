const $ = (id) => document.getElementById(id),
  encoder = new TextEncoder();
let worker,
  ready = false,
  engineFailed = false,
  generation = 0,
  revision = 0,
  pending = null,
  deadline,
  startupDeadline,
  opened = null,
  readToken = 0,
  rawReport = null,
  rawRequest = null;
const status = (text) => ($("status").textContent = text);
function clear() {
  rawReport = null;
  rawRequest = null;
  $("request-download").disabled = true;
  $("download").disabled = true;
  $("report").textContent = "";
  $("rows").replaceChildren();
  $("findings").replaceChildren();
  $("summary").textContent = "Inputs changed; partition again.";
  $("visual").replaceChildren();
  $("metrics").textContent = "";
}
function changed() {
  const wasComputing = pending !== null;
  revision++;
  readToken++;
  pending = null;
  clearTimeout(deadline);
  clear();
  if (wasComputing) {
    start();
    return;
  }
  $("run").disabled = !ready;
  if (ready) status("Inputs changed. Partition again.");
}
function fail(message) {
  engineFailed = true;
  generation++;
  ready = false;
  pending = null;
  clearTimeout(deadline);
  clearTimeout(startupDeadline);
  worker?.terminate();
  clear();
  $("run").disabled = true;
  status(message + " Inputs retained. Restart the engine.");
}
function start() {
  engineFailed = false;
  pending = null;
  clearTimeout(deadline);
  clearTimeout(startupDeadline);
  worker?.terminate();
  generation++;
  const own = generation;
  ready = false;
  clear();
  $("run").disabled = true;
  status("Loading Rust engine…");
  const began = performance.now();
  try {
    worker = new Worker(new URL("./worker.js", import.meta.url), {
      type: "module",
    });
  } catch {
    fail("Workers are unavailable.");
    return;
  }
  worker.onmessageerror = () => {
    if (own === generation) fail("Worker message could not be read.");
  };
  startupDeadline = setTimeout(() => {
    if (own === generation) fail("Engine startup exceeded 12 seconds.");
  }, 12000);
  worker.onerror = () => {
    if (own === generation) fail("Engine failed.");
  };
  worker.onmessage = ({ data }) => {
    if (own !== generation) return;
    if (data.startupError) {
      fail("Engine failed to load.");
      return;
    }
    if (data.ready) {
      clearTimeout(startupDeadline);
      ready = true;
      $("run").disabled = false;
      $("status").dataset.startupMs = performance.now() - began;
      $("status").dataset.workerInitializationMs = data.initializationMs;
      status("Rust ready. Partition a graph to inspect its assignment.");
      return;
    }
    if (!pending || data.id !== pending.id || data.revision !== revision)
      return;
    clearTimeout(deadline);
    const request = pending;
    pending = null;
    $("run").disabled = false;
    if (data.error) {
      fail("Partition worker failed.");
      return;
    }
    $("status").dataset.queryRoundtripMs = performance.now() - request.start;
    $("status").dataset.workerComputationMs = data.computationMs;
    $("status").dataset.inputBytes = encoder.encode($("graph").value).length;
    try {
      render(data.result);
      rawRequest = request.request;
      $("request-download").disabled = false;
    } catch {
      fail("Worker returned an unreadable report.");
    }
  };
}
function render(text) {
  rawReport = text;
  $("report").textContent = text;
  $("download").disabled = false;
  const report = JSON.parse(text);
  $("rows").replaceChildren();
  $("findings").replaceChildren();
  $("visual").replaceChildren();
  if (report.result !== "ok") {
    const e =
      typeof report.error === "string"
        ? report.error
        : JSON.stringify(report.error);
    $("summary").textContent = `${report.result}: ${e}`;
    status(`Partition returned ${report.result}. Inputs retained.`);
    return;
  }
  const r = report.record;
  $("summary").textContent =
    `Native partition validation passed. ${r.vertices} vertices, ${r.controls.k} parts. All parts contiguous: ${r.all_parts_contiguous}.`;
  $("metrics").textContent =
    `Weighted edge cut: ${r.edge_cut} · Total vertex weight: ${r.total_vertex_weight}`;
  status(
    "Partition completed. This is a heuristic result, not a quality or readiness certificate.",
  );
  const graph = JSON.parse($("graph").value),
    colors = [
      "#186579",
      "#d17720",
      "#8557a2",
      "#3a8461",
      "#b84c58",
      "#5475ad",
      "#8a743a",
      "#68596c",
      "#238e8b",
      "#a05d27",
      "#a84d85",
      "#636fa2",
      "#6d8c36",
      "#a44e36",
      "#448b9e",
      "#897646",
    ];
  const n = r.vertices,
    cols = Math.ceil(Math.sqrt(n)),
    rows = Math.ceil(n / cols),
    w = 640,
    h = Math.max(260, rows * 52),
    ns = "http://www.w3.org/2000/svg";
  const svg = document.createElementNS(ns, "svg");
  svg.setAttribute("viewBox", `0 0 ${w} ${h}`);
  svg.setAttribute("role", "img");
  svg.setAttribute(
    "aria-label",
    `${n} vertices colored by native part assignment; dashed edges cross parts. Index layout.`,
  );
  const pos = (v) => [
    30 + ((v % cols) * (w - 60)) / Math.max(1, cols - 1),
    30 + (Math.floor(v / cols) * (h - 60)) / Math.max(1, rows - 1),
  ];
  for (let v = 0; v < n; v++)
    for (let j = graph.xadj[v]; j < graph.xadj[v + 1]; j++) {
      const u = graph.adjncy[j];
      if (u <= v) continue;
      const line = document.createElementNS(ns, "line"),
        a = pos(v),
        b = pos(u),
        cut = r.assignment[v] !== r.assignment[u];
      for (const [key, value] of Object.entries({
        x1: a[0],
        y1: a[1],
        x2: b[0],
        y2: b[1],
        stroke: cut ? "#ad553e" : "#b7c8c2",
        "stroke-width": cut ? 2 : 1,
        "stroke-dasharray": cut ? "5 3" : "none",
      }))
        line.setAttribute(key, String(value));
      svg.append(line);
    }
  for (let v = 0; v < n; v++) {
    const [x, y] = pos(v),
      circle = document.createElementNS(ns, "circle"),
      label = document.createElementNS(ns, "text"),
      title = document.createElementNS(ns, "title");
    circle.setAttribute("cx", x);
    circle.setAttribute("cy", y);
    circle.setAttribute("r", "11");
    circle.setAttribute("fill", colors[r.assignment[v]]);
    title.textContent = `Vertex ${v}, part ${r.assignment[v]}, weight ${graph.vwgt?.[v] ?? 1}`;
    circle.append(title);
    label.setAttribute("x", x);
    label.setAttribute("y", y + 4);
    label.setAttribute("text-anchor", "middle");
    label.setAttribute("fill", "white");
    label.setAttribute("font-size", "10");
    label.textContent = v;
    svg.append(circle, label);
  }
  $("visual").append(svg);
  const table = document.createElement("table");
  const head = table.createTHead().insertRow();
  for (const value of ["Part", "Vertices", "Weight"]) {
    const th = document.createElement("th");
    th.textContent = value;
    head.append(th);
  }
  const body = table.createTBody();
  for (let p = 0; p < r.controls.k; p++) {
    const row = body.insertRow();
    for (const value of [p, r.part_vertex_counts[p], r.part_weights[p]])
      row.insertCell().textContent = value;
  }
  $("rows").append(table);
  const details = document.createElement("details"),
    caption = document.createElement("summary"),
    pre = document.createElement("pre");
  caption.textContent = "Every vertex assignment";
  pre.textContent = r.assignment.map((p, v) => `${v}: part ${p}`).join("\n");
  details.append(caption, pre);
  $("rows").append(details);
}
function query() {
  if (!ready) return;
  let weights;
  try {
    weights = JSON.parse($("targets").value);
  } catch {
    changed();
    status("Target weights must be a JSON array or null. Inputs retained.");
    return;
  }
  clear();
  const request = JSON.stringify({
    graph_json: $("graph").value,
    k: Number($("parts").value),
    seed: $("seed").value,
    mode: $("mode").value,
    coarsening: $("coarsening").value,
    ncuts: Number($("ncuts").value),
    niter: Number($("niter").value),
    ufactor: Number($("ufactor").value),
    target_weights: weights,
    contiguity: $("contiguity").checked,
    min_connectivity: $("min-connectivity").checked,
  });
  pending = { id: crypto.randomUUID(), start: performance.now(), request };
  worker.postMessage({ id: pending.id, revision, request });
  $("run").disabled = true;
  status("Partitioning in Rust…");
  deadline = setTimeout(() => fail("Partition exceeded 10 seconds."), 10000);
}
function save(bytes, name, type = "application/json") {
  const url = URL.createObjectURL(new Blob([bytes], { type })),
    a = document.createElement("a");
  a.href = url;
  a.download = name;
  a.click();
  setTimeout(() => URL.revokeObjectURL(url), 1000);
}
for (const id of [
  "graph",
  "mode",
  "parts",
  "seed",
  "coarsening",
  "ncuts",
  "niter",
  "ufactor",
  "targets",
  "contiguity",
  "min-connectivity",
])
  $(id).addEventListener("input", () => {
    changed();
  });
let acceptedText = $("graph").value;
$("graph").addEventListener("paste", (event) => {
  const field = $("graph"),
    value =
      field.value.slice(0, field.selectionStart) +
      event.clipboardData.getData("text/plain") +
      field.value.slice(field.selectionEnd);
  if (encoder.encode(value).length > 1048576) {
    event.preventDefault();
    status("Paste exceeds 1 MiB; accepted input retained.");
  }
});
$("graph").addEventListener("input", () => {
  const field = $("graph");
  if (encoder.encode(field.value).length > 1048576) {
    field.value = acceptedText;
    status("Edit exceeds 1 MiB; accepted input retained.");
  } else {
    acceptedText = field.value;
  }
});
$("form").addEventListener("submit", (e) => {
  e.preventDefault();
  query();
});
$("restart").onclick = start;
async function example() {
  changed();
  const own = revision;
  status("Loading authored example…");
  const controller = new AbortController(),
    timer = setTimeout(() => controller.abort(), 5000);
  try {
    const response = await fetch("./example.json", {
      signal: controller.signal,
    });
    if (!response.ok) throw Error("Example unavailable");
    const text = await response.text();
    if (revision !== own) return;
    changed();
    $("graph").value = text;
    acceptedText = text;
    status(
      ready
        ? "Authored example loaded. Partition the graph."
        : engineFailed
          ? "Example loaded; engine failed. Restart engine."
          : "Example loaded; Rust engine is starting.",
    );
  } catch {
    if (revision === own) status("Example could not load; open your own JSON.");
  } finally {
    clearTimeout(timer);
  }
}
$("example").onclick = example;
async function open(file) {
  if (!file) return;
  if (file.size > 1048576) {
    status("File exceeds 1 MiB; accepted input retained.");
    return;
  }
  changed();
  const token = readToken;
  status("Reading local file…");
  let timer;
  try {
    const bytes = new Uint8Array(
      await Promise.race([
        file.arrayBuffer(),
        new Promise((_, reject) => {
          timer = setTimeout(
            () => reject(Error("Read exceeded 5 seconds")),
            5000,
          );
        }),
      ]),
    );
    const text = new TextDecoder("utf-8", {
      fatal: true,
      ignoreBOM: true,
    }).decode(bytes);
    if (token !== readToken) return;
    changed();
    opened = bytes;
    $("original").disabled = false;
    $("graph").value = text;
    acceptedText = text;
    status("Opened original retained separately; inspect current text.");
  } catch (error) {
    if (token === readToken)
      status(String(error) + " Accepted input retained.");
  } finally {
    clearTimeout(timer);
  }
}
$("file").onchange = () => open($("file").files[0]);
$("graph").ondragover = (e) => e.preventDefault();
$("graph").ondrop = (e) => {
  e.preventDefault();
  open(e.dataTransfer.files[0]);
};
$("lf").onclick = () => {
  changed();
  $("graph").value = $("graph")
    .value.replace(/^\uFEFF/, "")
    .replace(/\r\n?/g, "\n");
  acceptedText = $("graph").value;
  status("Explicit LF copy is current; opened original retained.");
};
$("original").onclick = () => {
  if (opened) save(opened, "opened-graph-original.json");
};
$("current").onclick = () => save($("graph").value, "graph-current.json");
$("download").onclick = () => {
  if (rawReport) save(rawReport, "partition-report.json");
};
$("request-download").onclick = () => {
  if (rawRequest) save(rawRequest, "partition-request.json");
};
$("mode").addEventListener("change", () => {
  $("ncuts").value = $("mode").value === "recursive" ? "4" : "1";
  changed();
});
start();
example();
