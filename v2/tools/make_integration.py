"""Generate reviewable patches against exported committed fixtures; never edit a host."""
from pathlib import Path
import json,difflib
root=Path(__file__).resolve().parents[1]
slug='json-to-toon-converter'
manifest={'schemaVersion':2,'id':'com.tezbar.native.'+slug,'slug':slug,'name':'JSON to TOON Converter','version':'0.1.0','description':'Convert JSON data into TOON format instantly. Original Raycast extension by meshal; native adaptation by Tezbar.','icon':'/extensions/icons/'+slug+'.png','runtime':'rust','replaces':['raycast.'+slug],'platforms':['macOS','Windows'],'preferences':json.loads((root/'fixtures/own/package.json').read_text())['preferences'],'commands':[{'name':'convert-json','title':'Convert JSON','description':'Convert JSON to TOON format','platforms':['macOS','Windows'],'runtime':'rust','mode':'no-view'}],'verification':[{'platform':p,'state':'not-run'}for p in ['macOS','Windows']]}
(root/'manifest.json').write_text(json.dumps(manifest,indent=2)+'\n')
(root/'assets').mkdir(exist_ok=True)
icon=(root/'fixtures/original/extensions/json-to-toon-converter/assets/extension-icon.png').read_bytes()
(root/'assets/extension-icon.png').write_bytes(icon)
(root/'integration').mkdir(exist_ok=True)
for baseline in ['beta','current']:
 base=root/'fixtures'/baseline
 changes={}
 p='src-tauri/src/native_core/extensions/mod.rs';s=(base/p).read_text();changes[p]=s.replace('mod v2_sdk;', 'mod v2_sdk;\n#[path = "../../../native-extensions/json-to-toon-converter/adapter.rs"]\nmod v2_json_to_toon;')
 p='src-tauri/src/native_core/extensions/v2_runtime.rs';s=(base/p).read_text();s=s.replace('"perplexity" => Some(value)', '"perplexity" | "json-to-toon-converter" => Some(value)');s=s.replace('match slug {','match slug {\n        "json-to-toon-converter" => super::v2_json_to_toon::commands(),',1);needle='    match slug {\n        "amphetamine" => {';s=s.replace(needle,'    match slug {\n        "json-to-toon-converter" => super::v2_json_to_toon::handle(ctx, command, operation, payload, state).await,\n        "amphetamine" => {');assert 'v2_json_to_toon::handle' in s;changes[p]=s
 p='src-tauri/src/native_core/extensions/v2_catalog.json';s=(base/p).read_text();assert s.rstrip().endswith(']');changes[p]=s.rstrip()[:-1].rstrip()+',\n'+json.dumps(manifest,indent=2)+'\n]\n'
 p='src-tauri/src/native_core/extensions/v2_catalog.rs';s=(base/p).read_text();assert 'items.len(), 11' in s;changes[p]=s.replace('items.len(), 11','items.len(), 12')
 patch=''
 for p,s in changes.items():
  old=(base/p).read_text();assert old!=s,p
  patch+=''.join(difflib.unified_diff(old.splitlines(True),s.splitlines(True),fromfile='a/'+p,tofile='b/'+p))
 (root/f'integration/{baseline}-registration.patch').write_text(patch)
# Narrow current icon prerequisite; retain all other absolute-path policy.
p='src/renderer/extensionImages.ts';s=(root/'fixtures/current'/p).read_text();after=s.replace('export function extensionImageSource(value: string): string {','export function extensionImageSource(value: string): string {\n  if (value === "/extensions/icons/json-to-toon-converter.png") return value');assert s!=after
(root/'integration/current-icon-prerequisite.patch').write_text(''.join(difflib.unified_diff(s.splitlines(True),after.splitlines(True),fromfile='a/'+p,tofile='b/'+p)))
# Reproducible generated files for a headless harness, not host code.
(root/'harness').mkdir(exist_ok=True)

for baseline in ['beta','current']:
 p='src/renderer/src/raycast-api/index.tsx';s=(root/'fixtures'/baseline/p).read_text()
 needle='              root={root}\n              title={title}\n              onBack={onSurfaceBack}\n              onRunPrimaryAction={onRunPrimaryAction}\n              onOpenActions={openActions}\n            />\n          ) : ('
 assert needle in s
 after=s.replace(needle,needle.replace('onRunPrimaryAction={onRunPrimaryAction}','onRunPrimaryAction={() => onRunPrimaryAction()}'))
 (root/f'integration/{baseline}-detail-button-prerequisite.patch').write_text(''.join(difflib.unified_diff(s.splitlines(True),after.splitlines(True),fromfile='a/'+p,tofile='b/'+p)))

for baseline in ['beta','current']:
 p='src-tauri/Cargo.toml';s=(root/'fixtures'/baseline/(p+'.fixture')).read_text();needle='[dependencies]\n';assert needle in s
 after=s.replace(needle,needle+'ryu-js = "=1.0.2"\n',1)
 (root/f'integration/{baseline}-number-dependency.patch').write_text(''.join(difflib.unified_diff(s.splitlines(True),after.splitlines(True),fromfile='a/'+p,tofile='b/'+p)))
