"""Build the unchanged Rust kernel for native control and executing browser experiments."""
import argparse,copy,hashlib,json,os,pathlib,shutil,subprocess
p=argparse.ArgumentParser();p.add_argument('--bindgen',required=True);a=p.parse_args();R=pathlib.Path(__file__).resolve().parents[2];B=R/'browser/explorer';C=B/'.cache';C.mkdir(exist_ok=True)
source_guard=False
if (R/'.git').exists():
 subprocess.run(['git','diff','--exit-code','df08dec373b3daecd73313750766e397aa047e9f','--','src','Cargo.toml','Cargo.lock','tests','verify','benches','.pitfall','docs','.roles','.github/ci.yml','LICENSE','NOTICE'],cwd=R,check=True)
 source_guard=True
target=pathlib.Path(os.environ.get('CARGO_TARGET_DIR',R/'target')).resolve();env=dict(os.environ,CARGO_TARGET_DIR=str(target));suffix='.exe' if os.name=='nt' else ''
def run(cmd,**kw):return subprocess.run(cmd,cwd=R,env=kw.pop('env',env),check=True,**kw)
for n in ['app.js','worker.js']:run(['node','--check',str(B/'web'/n)])
run(['cargo','+1.95.0','build','--manifest-path',str(B/'Cargo.toml'),'--bin','metis-partition-explorer-cli','--release','--locked'])
wenv=dict(env);wenv['CARGO_ENCODED_RUSTFLAGS']='\x1f'.join(['--remap-path-prefix='+str(R)+'=/metis','-C','link-arg=--max-memory=67108864'])
run(['cargo','+1.95.0','build','--manifest-path',str(B/'Cargo.toml'),'--lib','--target','wasm32-unknown-unknown','--release','--locked'],env=wenv)
S=R/'target/partition-prototype/METIS-CORE';S.mkdir(parents=True,exist_ok=True)
for f in (B/'web').iterdir():
 if f.is_file():shutil.copyfile(f,S/f.name)
run([str(pathlib.Path(a.bindgen).resolve()),str(target/'wasm32-unknown-unknown/release/metis_partition_explorer.wasm'),'--target','web','--out-dir',str(S/'pkg')])
def graph(n,edges,weighted=False):
 rows=[[]for _ in range(n)]
 for u,v in edges:rows[u].append(v);rows[v].append(u)
 x=[0];adj=[]
 for row in rows:adj+=sorted(row);x.append(len(adj))
 return dict(xadj=x,adjncy=adj,vwgt=[1+(v%3)for v in range(n)] if weighted else [],adjwgt=[])
examples={'grid':json.loads((B/'web/example.json').read_text(encoding='utf-8')),'path':graph(12,[(i,i+1)for i in range(11)]),'weighted_path':graph(12,[(i,i+1)for i in range(11)],True),'star':graph(9,[(0,i)for i in range(1,9)]),'clique':graph(8,[(u,v)for u in range(8)for v in range(u+1,8)]),'dumbbell':graph(12,[(u,v)for base in [0,6]for u in range(base,base+6)for v in range(u+1,base+6)]+[(5,6)])}
cases=[]
def add(label,g=None,**controls):
 req=dict(graph_json=json.dumps(g if g is not None else examples['grid'],separators=(',',':')),k=4,seed='7',**{})
 req.update(controls);request=json.dumps(req,separators=(',',':'))
 binary=str(target/'release'/('metis-partition-explorer-cli'+suffix));output=run([binary],input=request.encode(),stdout=subprocess.PIPE).stdout.decode();repeat=run([binary],input=request.encode(),stdout=subprocess.PIPE).stdout.decode();assert output==repeat,label+' not native-repeatable';json.loads(output);cases.append(dict(label=label,request=request,native_output=output))
add('example')
for mode in ['kway','recursive']:
 for method in ['shem','hem','min_degree','two_hop']:
  for seed in ['0','7','18446744073709551615']:add('grid '+mode+' '+method+' '+seed,mode=mode,coarsening=method,seed=seed)
for family,g in examples.items():
 for mode in ['kway','recursive']:add(family+' '+mode,g,mode=mode)
add('weighted targets',examples['weighted_path'],k=2,target_weights=[0.25,0.75]);add('contiguity',contiguity=True,min_connectivity=True);add('multi trial',ncuts=3)
for name,key,value in [('zero parts','k',0),('too many parts','k',16),('browser parts','k',17),('zero trials','ncuts',0),('trials ceiling','ncuts',9),('zero iterations','niter',0),('iterations ceiling','niter',21),('bad ufactor','ufactor',1001),('bad seed','seed','18446744073709551616'),('unsafe seed number','seed',9007199254740993),('invalid targets','target_weights',[0.5,0.5])]:add(name,examples['path'],**{key:value})
for name,g in [('empty',dict(xadj=[0],adjncy=[])),('asymmetric',dict(xadj=[0,1,1],adjncy=[1])),('disconnected',dict(xadj=[0,0,0],adjncy=[])),('negative weight',dict(examples['path'],vwgt=[-1]*12)),('browser weight',dict(examples['path'],vwgt=[1001]*12)),('graph extension',dict(examples['path'],unknown=True))]:add(name,g)
add('malformed graph',graph_json='{');add('oversized graph',graph_json=' '*(1048576+1));add('browser vertices',graph(257,[(i,i+1)for i in range(256)]))
(C/'native-cases.json').write_text(json.dumps(cases,indent=2)+'\n',encoding='utf-8')
wasm=(S/'pkg/metis_partition_explorer_bg.wasm').read_bytes();proof=dict(prototype_only=True,native_cases=len(cases),wasm_bytes=len(wasm),wasm_sha256=hashlib.sha256(wasm).hexdigest(),native_pointer_bits=json.loads(cases[0]['native_output'])['target_pointer_bits'],cross_target_assignment_identity='Must be measured; not assumed',core_source_checked_against_entry=source_guard,publication_qualified=False);(C/'prototype-build.json').write_text(json.dumps(proof,indent=2)+'\n',encoding='utf-8');print(json.dumps(proof))
