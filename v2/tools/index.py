"""Record artifact hashes; the parent must rehash every record after staging."""
from pathlib import Path
import hashlib,base64,json,sys
r=Path(__file__).resolve().parents[1]
files=[]
for base in ['fixtures','tests','assets','integration']:
 for p in sorted((r/base).rglob('*')):
  if not p.is_file():continue
  data=p.read_bytes();digest=hashlib.sha256(data)
  item={'path':p.relative_to(r).as_posix(),'size':len(data),'sha256':digest.hexdigest(),'sri':'sha256-'+base64.b64encode(digest.digest()).decode()}
  path=item['path']
  if path.startswith('fixtures/own/'):item['origin']='git:https://github.com/almatkai/json-to-toon-converter@f72d105582202089268d31ab0d0314e5f1eef057:'+path.removeprefix('fixtures/own/')
  elif path.startswith('fixtures/beta/'):item['origin']='Tezbar git object fd300e276356587726566026f072563eb9c34cb1:'+path.removeprefix('fixtures/beta/').removesuffix('.fixture')
  elif path.startswith('fixtures/current/'):item['origin']='Tezbar git object fe71ca87f21ac4ff31505010c228f8abda0f2a89:'+path.removeprefix('fixtures/current/').removesuffix('.fixture')
  elif path.startswith('fixtures/original/'):
   item['origin']='https://raw.githubusercontent.com/raycast/extensions/6734d85992290066fd31a7f5bee3cca3d3545a1a/'+path.removeprefix('fixtures/original/').removesuffix('.http-status.txt')
   if path.endswith('.http-status.txt'):item['kind']='recorded HTTP status and request URL; not upstream license text'
  elif path.startswith('fixtures/library/'):item['origin']='npm:@toon-format/toon@1.3.0; exact tarball, extracted member or registry response as named'
  else:item['origin']='native port artifact / deterministic recorder fixture'
  files.append(item)
obj={'format':1,'note':'SHA256/SRI cover exact bytes; this is an evidence index, not a migration ledger. Rehash after staging.','files':files}
text=json.dumps(obj,indent=2)+'\n'
p=r/'INDEX.json'
if '--check' in sys.argv:assert p.read_text()==text
else:p.write_text(text)
print(f'{len(files)} indexed files')
