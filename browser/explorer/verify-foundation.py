"""Protect native algorithms, contracts, tests, verification, source pins and grants."""
import pathlib,subprocess
R=pathlib.Path(__file__).resolve().parents[2]
entry='df08dec373b3daecd73313750766e397aa047e9f'
subprocess.run(['git','diff','--exit-code',entry,'--','src','Cargo.toml','Cargo.lock','tests','verify','benches','.pitfall','docs','.roles','.github/ci.yml','LICENSE','NOTICE'],cwd=R,check=True)
subprocess.run(['git','diff','--exit-code','b4dc9663243c26771112da3329f171fc5f28c098','--','src','Cargo.toml','Cargo.lock','benches'],cwd=R,check=True)
print('Protected native foundation exact to canonical entry; core source/Cargo/benches also exact to public entry')
