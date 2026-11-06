// Execute the built Rust module. No JavaScript partitioning or audit policy.
import fs from 'node:fs/promises';
import { pathToFileURL } from 'node:url';
import path from 'node:path';
import { createHash } from 'node:crypto';

const [site, controlsPath, outputPath, abiPath] = process.argv.slice(2);
if (!site || !controlsPath || !outputPath || !abiPath) throw new Error('Use SITE CONTROLS OUTPUT RAW_ABI_OUTPUT');
const wasm = await fs.readFile(path.join(site, 'pkg/terrain_web_bg.wasm'));
const { default:init, plan_json } = await import(pathToFileURL(path.resolve(site, 'pkg/terrain_web.js')).href);
await init({ module_or_path:wasm });
const controlBytes = await fs.readFile(controlsPath, 'utf8');
if (!controlBytes.endsWith('\n')) throw new Error('Expected complete NDJSON controls');
const lines = controlBytes.slice(0,-1).split('\n');
function number(value) {
  if (typeof value === 'number') return value;
  if (value === 'NaN') return NaN;
  if (value === 'Infinity') return Infinity;
  if (value === '-Infinity') return -Infinity;
  throw new Error('Invalid control f64');
}
function execute(control) {
  try {
    const raw = plan_json(control.csv, control.count, control.graph, number(control.demand_limit), number(control.revenue_limit));
    JSON.parse(raw); // Validate the Rust serialization without rewriting it.
    return '{"Ok":' + raw + '}';
  } catch (error) { return JSON.stringify({Err:String(error)}); }
}
const output = await fs.open(outputPath, 'wx');
try {
  for (const line of lines) await output.write(execute(JSON.parse(line)) + '\n');
} finally { await output.close(); }

// Raw JavaScript usize coercion is a different transport contract. These are
// retained observations, never normalized into native typed parity cases.
const first = JSON.parse(lines[0]);
const rawCounts = [2.9, -1, 4294967297, NaN, Infinity];
const abi = rawCounts.map(count => ({raw_number:String(count), wasm_u32_transport:count>>>0, raw_result:execute({...first,count})}));
await fs.writeFile(abiPath, JSON.stringify(abi,null,2)+'\n', {flag:'wx'});
console.log(JSON.stringify({controls:lines.length, raw_abi_observations:abi.length, wasm_bytes:wasm.length, wasm_sha256:createHash('sha256').update(wasm).digest('hex')}));
