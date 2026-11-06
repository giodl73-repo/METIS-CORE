"""Compare complete original native/Rust-WASM planner results without normalization."""
import argparse
import csv
import hashlib
import io
import json
import os
from pathlib import Path
import subprocess
import sys

ROOT = Path(__file__).resolve().parents[1]
HEADER = 'site_id,demand,revenue,latitude,longitude\n'


def fixture(rows):
    stream = io.StringIO(newline='')
    writer = csv.writer(stream, lineterminator='\n')
    writer.writerow(['site_id', 'demand', 'revenue', 'latitude', 'longitude'])
    writer.writerows(rows)
    return stream.getvalue()


def controls():
    rows = []
    sample = (ROOT / 'web/sample.csv').read_bytes().decode('utf-8')
    def add(name, text=sample, count=3, graph=False, demand=1.25, revenue=1.5, ok=True, sites=18, expected=None):
        rows.append(dict(name=name, arguments=dict(csv=text, count=count, graph=graph, demand_limit=demand, revenue_limit=revenue), expected_ok=ok, expected_sites=sites, expected_fields=expected or {}))
    for graph in [False, True]:
        for count in [1, 2, 3, 4, 8, 18]:
            add(f'sample-{graph}-{count}', count=count, graph=graph)
        for limit in [1.0, 1.25, 2.0, 10.0]:
            add(f'limit-{graph}-{limit}', graph=graph, demand=limit, revenue=limit)
        for limit in [0.999, 10.001, 'NaN', 'Infinity', '-Infinity']:
            add(f'invalid-limit-{graph}-{limit}', graph=graph, demand=limit, ok=False)
        for count in [0, 19, 33]:
            add(f'invalid-count-{graph}-{count}', count=count, graph=graph, ok=False)
        city = lambda n: fixture([(f'S{i}', i % 5 + 1, (i % 7 + 1) * 10, i / 10000, (i % 7) / 10000) for i in range(n)])
        add(f'count32-{graph}', city(32), 32, graph, sites=32)
        add(f'sites2000-{graph}', city(2000), 8, graph, sites=2000)
        add(f'sites2001-{graph}', city(2001), 8, graph, ok=False, sites=2001)
        add(f'zero-totals-{graph}', fixture([('a',0,0,0,0),('b',0,0,1,1)]), 2, graph, sites=2, expected=dict(demand_ratio=1.0,revenue_ratio=1.0,passes=True))
        add(f'zero-positive-{graph}', fixture([('a',0,0,0,0),('b',1,1,1,1)]), 2, graph, sites=2, expected=dict(demand_ratio=None,revenue_ratio=None,passes=False))
        add(f'workload-ceiling-{graph}', fixture([('a',1e12,1e12,-90,-180),('b',1e12,1e12,90,180)]), 2, graph, sites=2)
        add(f'quoted-unicode-{graph}', fixture([('Été, <alpha&"β>',1,10,0,0),('東京',2,20,1,1)]), 2, graph, sites=2)
        add(f'crlf-{graph}', sample.replace('\r\n','\n').replace('\n','\r\n'), graph=graph)
    for name, text in [('empty',''),('header-only',HEADER),('missing-header','wrong,header\na,b\n'),('bad-number',fixture([('a','bad',1,0,0)])),('duplicate-id',fixture([('a',1,1,0,0),('a',1,1,1,1)]))]:
        add(name, text, 1, ok=False)
    for name, values in [('negative-demand',(-1,1,0,0)),('large-demand',(1e12+1,1,0,0)),('nan-demand',('NaN',1,0,0)),('infinite-revenue',(1,'inf',0,0)),('bad-latitude',(1,1,90.001,0)),('bad-longitude',(1,1,0,-180.001))]:
        add(name, fixture([('a',*values)]), 1, ok=False)
    for token in ['a', 'é']:
        for size in [499999, 500000, 500001]:
            tail = ',1,1,0,0\n'
            remaining = size - len((HEADER + tail).encode())
            width = len(token.encode())
            text = HEADER + token * (remaining // width) + 'a' * (remaining % width) + tail
            assert len(text.encode()) == size
            add(f'utf8-budget-{token}-{size}', text, 1, ok=size<=500000, sites=1)
    return rows


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('--site', type=Path, required=True)
    parser.add_argument('--output', type=Path, required=True)
    args = parser.parse_args()
    output = Path(os.path.abspath(args.output))
    for parent in [output, *output.parents]:
        if parent.is_symlink() or (parent.exists() and getattr(parent.lstat(), 'st_file_attributes', 0) & 0x400):raise ValueError('Output uses link/junction')
    if output.exists() or output == ROOT or ROOT in output.parents or output in ROOT.parents:raise ValueError('Use a new output outside source')
    metadata = json.loads(subprocess.check_output(['cargo','metadata','--locked','--offline','--no-deps','--format-version','1'],cwd=ROOT))
    target = Path(metadata['target_directory']).resolve()
    if output == target or output in target.parents or target in output.parents:raise ValueError('Output overlaps compiled target')
    output.mkdir(parents=True)
    cases = controls()
    (output/'controls.json').write_text(json.dumps(cases,ensure_ascii=False,indent=2,allow_nan=False)+'\n',encoding='utf-8',newline='\n')
    control_file = output/'controls.jsonl'
    control_file.write_bytes(b''.join((json.dumps(c['arguments'],ensure_ascii=False,separators=(',',':'),allow_nan=False)+'\n').encode() for c in cases))
    commands = []
    def run(name, command, data=None):
        process = subprocess.run([str(x) for x in command],cwd=ROOT,input=data,capture_output=True)
        (output/(name+'.stdout')).write_bytes(process.stdout);(output/(name+'.stderr')).write_bytes(process.stderr)
        commands.append(dict(name=name,command=[str(x) for x in command],exit_code=process.returncode))
        (output/'commands.json').write_text(json.dumps(commands,indent=2)+'\n',encoding='utf-8')
        if process.returncode:raise RuntimeError(name+' failed; retained outputs at '+str(output))
        return process.stdout
    run('build-native',['cargo','build','--locked','--offline','--release','-p','terrain-web','--example','planner-control'])
    native = target/'release/examples'/('planner-control.exe' if os.name=='nt' else 'planner-control')
    native_raw = run('native',[native],control_file.read_bytes())
    run('wasm',['node',ROOT/'tools/check-planner.mjs',args.site.resolve(),control_file,output/'wasm.jsonl',output/'raw-js-abi.json'])
    wasm_raw = (output/'wasm.jsonl').read_bytes()
    assert native_raw.endswith(b'\n') and wasm_raw.endswith(b'\n')
    before, after = native_raw[:-1].split(b'\n'), wasm_raw[:-1].split(b'\n')
    assert len(before) == len(after) == len(cases)
    results = []
    for case, native_line, wasm_line in zip(cases,before,after):
        value = json.loads(native_line)
        ok = 'Ok' in value
        expected = ok == case['expected_ok']
        if ok:
            plan = value['Ok'];geo = json.loads(plan['geojson'])
            expected = expected and plan['site_count']==case['expected_sites'] and plan['territory_count']==case['arguments']['count'] and sum(p['site_count'] for p in plan['summaries'])==plan['site_count'] and len(geo['features'])==plan['site_count']+plan['territory_count']
            expected = expected and all(plan[k]==v for k,v in case['expected_fields'].items())
        results.append(dict(name=case['name'],exact=native_line==wasm_line,expected=expected,native_ok=ok,native_sha256=hashlib.sha256(native_line).hexdigest(),wasm_sha256=hashlib.sha256(wasm_line).hexdigest()))
    receipt = dict(schema='terrain.planner-parity.v1',controls=len(cases),all_exact=native_raw==wasm_raw,all_results_exact=all(x['exact'] for x in results),all_expectations_met=all(x['expected'] for x in results),cases=results,native_sha256=hashlib.sha256(native_raw).hexdigest(),wasm_sha256=hashlib.sha256(wasm_raw).hexdigest(),raw_abi_observations=5,commands=commands,scope='Complete unchanged Rust planning Result serialization and whole NDJSON byte streams, all summary/ratio/pass/ID/SVG/GeoJSON bytes and errors, both methods, full2000-site/32-territory/500000-UTF8-byte/value bounds and independent conservation/zero expectations. Named f64 tokens are test transport only. Raw JS usize coercion observations separate. Source/input/archive/default/live delivery qualification remains separate.')
    (output/'receipt.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
    print(json.dumps({k:receipt[k] for k in ['controls','all_exact','all_expectations_met','raw_abi_observations']}))
    if not receipt['all_exact'] or not receipt['all_expectations_met']:raise SystemExit(1)


if __name__=='__main__':main()
