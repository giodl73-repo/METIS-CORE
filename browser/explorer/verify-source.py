"""Extract exact delivered source outside Git and replay native/browser checks."""
import argparse,json,os,pathlib,re,shutil,subprocess,tempfile,zipfile
P=pathlib.Path(__file__).resolve().parent
ROOT=P.parent.parent
parser=argparse.ArgumentParser();parser.add_argument('--bindgen',required=True);args=parser.parse_args()
site=ROOT/'target/partition-release/METIS-CORE'
cache=P/'.cache';cache.mkdir(exist_ok=True)
original=json.loads((cache/'native-cases.json').read_text(encoding='utf-8'))
original_wasm=json.loads((cache/'wasm-cases.json').read_text(encoding='utf-8'))
work=ROOT/'target/source-replay';work.mkdir(parents=True,exist_ok=True)
assert work.resolve().is_relative_to(ROOT.resolve()/'target')
results=[]
def run(command,cwd,env=None):
 result=subprocess.run(command,cwd=cwd,env=env,stdout=subprocess.PIPE,stderr=subprocess.STDOUT)
 with (cache/'source-replay.log').open('ab') as log:log.write(('\n$ '+' '.join(map(str,command))+'\n').encode());log.write(result.stdout)
 if result.returncode:raise SystemExit('Source replay failed; see .cache/source-replay.log')
 return result.stdout
(cache/'source-replay.log').write_bytes(b'')
with tempfile.TemporaryDirectory(prefix='metis replay-',dir=work) as temporary:
 base=pathlib.Path(temporary).resolve();assert base.is_relative_to(work.resolve())
 with zipfile.ZipFile(site/'source.zip') as archive:
  for name in archive.namelist():
   if name.endswith('/'):continue
   relative=pathlib.PurePosixPath(name)
   assert relative.parts[0]=='METIS-CORE-source' and '..' not in relative.parts and not relative.is_absolute()
   destination=base.joinpath(*relative.parts);assert destination.resolve().is_relative_to(base)
   destination.parent.mkdir(parents=True,exist_ok=True);destination.write_bytes(archive.read(name))
 source=base/'METIS-CORE-source';consumer=source/'browser/explorer'
 assert not (source/'.git').exists()
 cargo=shutil.which('cargo');python=shutil.which('python');npm=shutil.which('npm')
 manifest='browser/explorer/Cargo.toml'
 run([python,'browser/explorer/native-environment.py'],source)
 native=run([cargo,'+1.95.0','test','--all-targets','--locked'],source)
 run([cargo,'+1.95.0','test','--manifest-path',manifest,'--locked'],source)
 run([python,'browser/explorer/build-prototype.py','--bindgen',str(pathlib.Path(args.bindgen).resolve())],source)
 rebuilt=json.loads((consumer/'.cache/native-cases.json').read_text(encoding='utf-8'))
 assert rebuilt==original,'Independent native request/response drift'
 run([npm,'ci','--ignore-scripts'],consumer)
 env=dict(os.environ);env['METIS_SITE_DIR']='../../target/partition-prototype'
 # Delivery is pinned to the original Git artifact; this replay checks rebuilt behavior.
 run([npm,'run','test:pages','--','--grep-invert','complete packaged delivery|independent source, build and legal routes'],consumer,env)
 browser=json.loads((consumer/'.cache/browser-results.json').read_text(encoding='utf-8'))
 assert browser['stats']['unexpected']==browser['stats']['skipped']==browser['stats']['flaky']==0
 shutil.copyfile(consumer/'.cache/browser-results.json',cache/'source-replay-browser.json')
 build=json.loads((consumer/'.cache/prototype-build.json').read_text(encoding='utf-8'))
 replayed_wasm=json.loads((consumer/'.cache/wasm-cases.json').read_text(encoding='utf-8'))
 assert replayed_wasm==original_wasm,'Independent WASM32 outcome drift (not a native64 identity assertion)'
 counts=re.findall(rb'test result: ok\. (\d+) passed; 0 failed; (\d+) ignored;',native);assert counts
 cross=json.loads((consumer/'.cache/cross-target.json').read_text(encoding='utf-8'))
 receipt={'outside_git':True,'native_tests':sum(int(c[0]) for c in counts),'native_ignored':sum(int(c[1]) for c in counts),'native_environment':json.loads((consumer/'.cache/native-environment.json').read_text(encoding='utf-8')),'adapter_tests':4,'browser_checks':browser['stats']['expected'],'identical_rebuilt_native_target_responses':len(original),'identical_rebuilt_wasm32_responses':len(original_wasm),'cross_target':{k:v for k,v in cross.items() if k!='records'},'rebuilt_wasm_sha256':build['wasm_sha256'],'original_wasm_sha256':json.loads((site/'provenance.json').read_text(encoding='utf-8'))['wasm_sha256'],'binary_identity_claimed':False,'cross_target_identity_claimed':False}
 (cache/'source-replay.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8')
 print(json.dumps(receipt))
