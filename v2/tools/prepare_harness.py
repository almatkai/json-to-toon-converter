"""Materialize nested manifests and pinned host modules without external source paths."""
from pathlib import Path
import shutil,subprocess
r=Path(__file__).resolve().parents[1]; h=r/'.work/harness'; dest=h/'generated'
shutil.copytree(r/'harness',h,dirs_exist_ok=True,ignore=shutil.ignore_patterns('target','generated','Cargo.toml'))
for p in h.rglob('Cargo.toml.in'):
 p.with_suffix('').write_bytes(p.read_bytes())
# Copy only immutable exports. Test-only source rewrites are listed here, not
# applied to fixtures: omit unrelated host test modules and point at our tests.
for baseline in ['beta','current']:
 out=r/'.work'/('patch-check-'+baseline);out.mkdir(parents=True,exist_ok=True)
 shutil.copytree(r/'fixtures'/baseline,out,dirs_exist_ok=True)
 (out/'src-tauri/Cargo.toml').write_bytes((r/'fixtures'/baseline/'src-tauri/Cargo.toml.fixture').read_bytes())
 p=r/f'integration/{baseline}-number-dependency.patch'
 subprocess.run(['patch','--dry-run','-s','-p1','-d',str(out),'-i',str(p)],check=True)
 subprocess.run(['patch','-s','-p1','-d',str(out),'-i',str(p)],check=True)
 patch=r/f'integration/{baseline}-registration.patch'
 subprocess.run(['patch','--dry-run','-s','-p1','-d',str(out),'-i',str(patch)],check=True)
 subprocess.run(['patch','-s','-p1','-d',str(out),'-i',str(patch)],check=True)
 if baseline=='current':
  p=r/'integration/current-icon-prerequisite.patch'
  subprocess.run(['patch','--dry-run','-s','-p1','-d',str(out),'-i',str(p)],check=True)
  subprocess.run(['patch','-s','-p1','-d',str(out),'-i',str(p)],check=True)
 p=r/f'integration/{baseline}-detail-button-prerequisite.patch'
 subprocess.run(['patch','--dry-run','-s','-p1','-d',str(out),'-i',str(p)],check=True)
 subprocess.run(['patch','-s','-p1','-d',str(out),'-i',str(p)],check=True)
 # Every preceding catalog entry remains byte-equivalent as JSON, in order.
 import json
 before=json.loads((r/'fixtures'/baseline/'src-tauri/src/native_core/extensions/v2_catalog.json').read_text())
 after=json.loads((out/'src-tauri/src/native_core/extensions/v2_catalog.json').read_text())
 assert after[:-1]==before and after[-1]==json.loads((r/'manifest.json').read_text())
base=r/'.work/patch-check-beta/src-tauri/src/native_core'
for name in ['common.rs','extensions/v2_sdk.rs','extensions/native.rs','extensions/registry.rs','extensions/v2_runtime.rs','extensions/v2_catalog.rs','extensions/v2_catalog.json']:
 data=(base/name).read_text()
 if name=='extensions/native.rs':
  data=data.replace('#[cfg(test)]\nuse super::perplexity;\n','')
  tail='#[cfg(test)]\n#[path = "native_tests.rs"]\nmod tests;';assert data.endswith(tail+'\n');data=data[:-len(tail+'\n')]
 if name=='extensions/registry.rs':data=data.split('#[cfg(test)]\nmod tests {')[0]
 p=dest/'src-tauri/src/native_core'/name;p.parent.mkdir(parents=True,exist_ok=True);p.write_text(data)
port=dest/'src-tauri/native-extensions/json-to-toon-converter';(port/'src').mkdir(parents=True,exist_ok=True)
for name in ['adapter.rs','src/lib.rs']:(port/name).write_bytes((r/name).read_bytes())
# Preserve actual sdk/common unit tests. Other adapter dispatches are explicit
# unavailable stubs; actual registry/catalog/session machinery remains compiled.
mods=['amphetamine','audio_device','cleanshotx','color_picker','gif_search','google_calendar','google_search','port_manager','speedtest','timers','perplexity']
source='''// Generated from immutable beta fixtures by tools/prepare_harness.py.
#![allow(dead_code)] // Host modules expose APIs unrelated to this one-command harness.
pub mod native_core {
 #[path="../../generated/src-tauri/src/native_core/common.rs"] pub mod common;
 pub mod extensions {
  #[path="../../../generated/src-tauri/src/native_core/extensions/v2_sdk.rs"] mod v2_sdk;
  #[path="../../../generated/src-tauri/src/native_core/extensions/native.rs"] mod native;
  #[path="../../../generated/src-tauri/src/native_core/extensions/registry.rs"] mod registry;
  #[path="../../../generated/src-tauri/src/native_core/extensions/v2_catalog.rs"] mod v2_catalog;
  #[path="../../../generated/src-tauri/src/native_core/extensions/v2_runtime.rs"] mod v2_runtime;
  #[path="../../../generated/src-tauri/native-extensions/json-to-toon-converter/adapter.rs"] mod v2_json_to_toon;
  #[cfg(test)] mod perplexity {}
'''
for name in mods:
 source+=f'''  mod v2_{name} {{
   use crate::native_core::common::{{CoreContext,CoreResult}};
   use serde_json::Value;
   pub fn commands()->&'static [&'static str]{{ &[] }}
   pub async fn handle(_: &CoreContext,_: &str,_: &str,_: &Value,_: &mut Value)->CoreResult{{Err("Unrelated adapter is outside this harness".into())}}
  }}
'''
source+='  #[cfg(test)] #[path="../../../tests.rs"] mod tests;\n }\n}\n'
(h/'src/lib.rs').write_text(source)
# Rust resolves nested path attributes against implicit module directories.
(h/'src/native_core/extensions').mkdir(parents=True,exist_ok=True)
print('Prepared real beta common/SDK/NativeSessions/registry/catalog + concrete adapter; both patches checked')

subprocess.run(['rustfmt','--edition','2021','--config','skip_children=true',str(h/'src/lib.rs')],check=True)
