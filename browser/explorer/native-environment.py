"""Record optional reference availability; never claims skipped bodies executed."""
import json,os,pathlib,platform,shutil,subprocess
P=pathlib.Path(__file__).resolve().parent;R=P.parents[1]
paths=[os.environ.get('METIS_GPMETIS'),os.environ.get('METIS_CORE_GPMETIS'),r'C:\src\metis\build\programs\gpmetis.exe',r'C:\src\metis\build\Windows\programs\Release\gpmetis.exe',r'C:\src\metis\build\programs\Release\gpmetis.exe',r'C:\src\apportionment\bin\gpmetis.exe',shutil.which('gpmetis')]
found=next((p for p in paths if p and pathlib.Path(p).is_file()),None)
fixtures={n:any((d/n).is_file() for d in [R.parent/'metis/graphs',pathlib.Path('C:/src/metis/graphs')]) for n in ['4elt.graph','test.mgraph','copter2.graph']}
receipt={'platform':platform.platform(),'pointer_bits_expected':64,'gpmetis_available_path':found,'external_fixtures_available':fixtures,'heavy_reference_requested':os.environ.get('METIS_CORE_HEAVY_PARITY')=='1','scope':'Availability only. Native counts include optional bodies that return without reference execution when tools/data are absent. No gpmetis quality or formal verification certification.'}
(P/'.cache').mkdir(exist_ok=True);(P/'.cache/native-environment.json').write_text(json.dumps(receipt,indent=2)+'\n',encoding='utf-8');print(json.dumps(receipt))
