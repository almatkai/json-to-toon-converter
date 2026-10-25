"""Audit the actual Cargo archive and test its extracted self-contained Rust sources."""
from pathlib import Path
import subprocess,tarfile,json,hashlib,tempfile,os
os.environ.pop("SUPERCMD_EXTENSION_PATHS",None)
r=Path(__file__).resolve().parents[1]
subprocess.run(['cargo','+stable','package','--allow-dirty','--locked','--offline','--no-verify'],cwd=r,check=True)
crate=r/'target/package/tezbar-json-to-toon-native-0.1.0.crate'
index=json.loads((r/'INDEX.json').read_text())
with tarfile.open(crate,'r:gz') as archive:
 members=archive.getmembers();prefix='tezbar-json-to-toon-native-0.1.0/'
 names={m.name.removeprefix(prefix):m for m in members if m.isfile()}
 for m in members:
  assert m.name.startswith(prefix) and '..' not in Path(m.name).parts and not Path(m.name).is_absolute()
  assert not m.issym() and not m.islnk()
 for p in names:
  assert not any(x in Path(p).parts for x in ['.work','target','node_modules','generated']),p
  assert not p.startswith('dist/'),p
 for record in index['files']:
  p=record['path'];assert p in names,f'missing indexed artifact: {p}'
  data=archive.extractfile(names[p]).read()
  assert len(data)==record['size'] and hashlib.sha256(data).hexdigest()==record['sha256'],p
 for p in ['adapter.rs','src/lib.rs','src/main.rs','harness/Cargo.toml.in','harness/Cargo.lock','harness/tauri-stub/Cargo.toml.in','tools/prepare_harness.py','tests/surfaces.json','INDEX.json','NOTICES.md','LICENSE']:
  assert p in names,p
 for p in names:
  if p.startswith(('src/','tools/','harness/','integration/')):
   assert (b'/' + b'Users/') not in archive.extractfile(names[p]).read(),p
 dest=Path(tempfile.mkdtemp(prefix='package-audit-',dir=r/'.work'))
 archive.extractall(dest)
 extracted=dest/prefix.rstrip('/')
subprocess.run(['python3','tools/prepare_harness.py'],cwd=extracted,check=True)
for chain in ['stable','1.85.0']:
 for manifest in ['Cargo.toml','.work/harness/Cargo.toml']:
  subprocess.run(['cargo','+'+chain,'test','--locked','--offline','--manifest-path',manifest],cwd=extracted,check=True)
print(f'PASS actual archive: {len(names)} files, {len(index["files"])} indexed records; extracted core + real beta harness offline tests on stable and 1.85.0; retained at {extracted.relative_to(r)}')
