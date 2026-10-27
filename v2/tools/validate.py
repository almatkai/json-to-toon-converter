"""Full Draft 2020-12 validation, negative cases and immutable evidence verification."""
from pathlib import Path
import copy,json,hashlib,base64
from jsonschema import Draft202012Validator
r=Path(__file__).resolve().parents[1]
schema=json.loads((r/'fixtures/beta/schemas/extension-v2.schema.json').read_text())
Draft202012Validator.check_schema(schema);v=Draft202012Validator(schema)
m=json.loads((r/'manifest.json').read_text());v.validate(m)
negative=[]
for key,value in [('author','meshal'),('schemaVersion',1),('runtime','javascript'),('id','raycast.bad'),('icon','file:///tmp/icon.png'),('slug','Bad Slug')]:
 x=copy.deepcopy(m);x[key]=value;negative.append(x)
for key in schema['required']:
 x=copy.deepcopy(m);del x[key];negative.append(x)
for mutation in ['empty-commands','bad-platform','duplicate-platform','bad-verification','missing-command-runtime']:
 x=copy.deepcopy(m)
 if mutation=='empty-commands':x['commands']=[]
 elif mutation=='bad-platform':x['commands'][0]['platforms']=['Plan9']
 elif mutation=='duplicate-platform':x['commands'][0]['platforms']=['macOS','macOS']
 elif mutation=='bad-verification':x['verification'][0]['state']='tested'
 else:del x['commands'][0]['runtime']
 negative.append(x)
assert all(list(v.iter_errors(x)) for x in negative)
assert m['preferences']==json.loads((r/'fixtures/own/package.json').read_text())['preferences']
assert [x['state']for x in m['verification']]==['not-run','not-run']
icon=(r/'assets/extension-icon.png').read_bytes()
assert len(icon)==159623 and hashlib.sha256(icon).hexdigest()=='a0d60a4ee7f9074dc52ad478a89e11a4b13e244eff9863d45cf2c69081a1a384'
assert icon==(r/'fixtures/original/extensions/json-to-toon-converter/assets/extension-icon.png').read_bytes()
assert icon==(r/'fixtures/own/assets/extension-icon.png').read_bytes()
lock=json.loads((r/'fixtures/own/package-lock.json').read_text());assert lock['packages']['node_modules/@toon-format/toon']['version']=='1.3.0'
registry=json.loads((r/'fixtures/library/registry-1.3.0.json').read_text())
tar=(r/'fixtures/library/toon-format-toon-1.3.0.tgz').read_bytes()
assert registry['dist']['integrity']=='sha512-'+base64.b64encode(hashlib.sha512(tar).digest()).decode()
index=json.loads((r/'INDEX.json').read_text())
for item in index['files']:
 p=Path(item['path']);assert not p.is_absolute() and '..' not in p.parts
 data=(r/p).read_bytes()
 assert len(data)==item['size'],item['path']
 assert hashlib.sha256(data).hexdigest()==item['sha256'],item['path']
 assert 'sha256-'+base64.b64encode(hashlib.sha256(data).digest()).decode()==item['sri'],item['path']
print(f'PASS Draft 2020-12 manifest + {len(negative)} negatives; icon, locked library tarball, {len(index["files"])} indexed records')
