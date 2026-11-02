"""Verify complete public files, source archive, notices and separate budgets."""
import argparse,hashlib,json,pathlib,zipfile
parser=argparse.ArgumentParser();parser.add_argument('site',type=pathlib.Path);parser.add_argument('--commit',required=True);a=parser.parse_args();site=a.site.resolve()
def sha(data):return hashlib.sha256(data).hexdigest()
def read(name):return json.loads((site/name).read_text(encoding='utf-8'))
proof=read('provenance.json');inv=read('inventory.json');assert proof['commit']==inv['commit']==a.commit
actual={p.relative_to(site).as_posix() for p in site.rglob('*') if p.is_file()}
assert not any(p.is_symlink() or any(part.startswith('.') for part in p.relative_to(site).parts) for p in site.rglob('*'))
records={r['path']:r for r in inv['files']};assert len(records)==len(inv['files'])
assert actual==set(records)|{'inventory.json'},actual^set(records)
for name,record in records.items():
 data=(site/name).read_bytes();assert len(data)==record['bytes'] and sha(data)==record['sha256'],name
budgets=proof['delivery_budgets'];total=sum((site/name).stat().st_size for name in actual)
assert total<=budgets['complete_site_bytes']
assert sum((site/name).stat().st_size for name in proof['runtime_files'])==inv['runtime_bytes']<=budgets['initial_runtime_bytes']
assert (site/'source.zip').stat().st_size<=budgets['optional_source_zip_bytes']
assert sha((site/'source.zip').read_bytes())==proof['source_zip_sha256']
assert sha((site/'pkg/metis_partition_explorer_bg.wasm').read_bytes())==proof['wasm_sha256']
with zipfile.ZipFile(site/'source.zip') as archive:
 assert archive.comment.decode()==a.commit
 pins=read('source-files.json');expected={'METIS-CORE-source/'+r['path']:r for r in pins}
 assert len(expected)==len(pins)==proof['source_files']
 names=[n for n in archive.namelist() if not n.endswith('/')]
 assert len(names)==len(set(names)) and set(names)==set(expected)
 for name,record in expected.items():
  path=pathlib.PurePosixPath(name);assert not path.is_absolute() and '..' not in path.parts
  data=archive.read(name);assert len(data)==record['bytes'] and sha(data)==record['sha256']
packages=read('dependencies.json');assert len(packages)==proof['legal_packages']
for package in packages:
 assert package['notices']
 for notice in package['notices']:assert sha((site/notice['path']).read_bytes())==notice['sha256']
rust=read('rust/provenance.json');assert len(rust['notices'])==proof['rust_notice_files']
assert sha((site/'rust/COPYRIGHT-library.html').read_bytes())==rust['library_copyright_sha256']
for notice in rust['notices']:assert sha((site/notice['path']).read_bytes())==notice['sha256']
assert not any(name.endswith(('native-cases.json','browser-results.json','request.json','cargo-metadata.json')) for name in actual)
print(json.dumps({'commit':a.commit,'files':len(actual),'total_bytes':total,'runtime_bytes':inv['runtime_bytes'],'source_files':len(pins),'legal_packages':len(packages),'rust_notices':len(rust['notices']),'verified':True}))
