import fs from 'node:fs';
import path from 'node:path';
import vm from 'node:vm';
import assert from 'node:assert/strict';
import { createRequire } from 'node:module';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';
import { encode } from '../fixtures/library/package/dist/index.mjs';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const require=createRequire(import.meta.url);
const ts=require(path.join(root,'.work/v1/node_modules/typescript'));
let seed=0x67abcd12;
function random(){seed=(Math.imul(seed,1664525)+1013904223)>>>0;return seed/2**32;}
const fixed=[
 'null','true','false','-0','-0.000e+9','1e400','-1e400','1e-999','9007199254740993','18446744073709551615','1000000000000000128','1e-6','1e-7','1e20','1e21','5e-324','2.2250738585072014e-308',
 '{}','[]','{"a":{}}','{"":[]}','{"": [1,2]}','{"z":1,"10":2,"2":3,"a":4,"01":5,"4294967295":6,"0":7}',
 '{"b":1,"a":2,"b":3}','{"__proto__":null,"a":1}','{"__proto__":{"a":3},"b":1}',
 '[{"a":1},{"__proto__":{"a":2},"b":3}]',
 '[{"length":2},{"__proto__":[1,2,3],"b":4}]',
 '"Қазақ 日本 😀"','"\\ud83d\\ude00"','"\\u0000\\b\\f\\n\\r\\t\\\\\\\""',
 '["", "null","false","01","1e4","+1","1.",".1","-word","a,b","a|b","a:b","a[b]","a{b}"," a ","\\ufeffa","a\\u0085","x\\u2028y"]',
 '[1,null,true,"a"]','[[1,2],[],[3]]','[[[1]],{"a":2},false]',
 '[{"a":1,"b":2},{"b":3,"a":4}]','[{},{"a":1}]','[{},{}]',
 '[{"a":{"x":1},"b":2},{"a":[{"x":1},{"x":2}]}]',
 '[{"a":[{},{}],"z":4},{"a":[[1],[2]],"z":6}]',
 '[{"": [1,2],"next":4}]','{"a.b":{"c":1},"a":{"b":2}}',
 '"\\ud800"','"\\udc00"','{"\\ud800":1}','[1,,2]','[1,]','{"a":1,}','NaN','Infinity','01','+1','1.','[','{"private-secret":', '\ufeff1','\u00a01', '\u00851','"\\x00"', '"bad\nline"',
];
for(let i=0;i<1500;i++){
 const b=Buffer.alloc(8); b.writeUInt32LE(Math.floor(random()*2**32),0);b.writeUInt32LE(Math.floor(random()*2**32),4);
 const n=b.readDoubleLE();if(Number.isFinite(n))fixed.push(JSON.stringify(n));
 const exponent=Math.floor(random()*800)-400;
 fixed.push(`${random()<.5?'-':''}${Math.floor(random()*1e16)}.${Math.floor(random()*1e8)}e${exponent}`);
}
const keys=['a','b','2','01','__proto__','','x.y','a,b','Қазақ'];
function value(d=0){if(d>3||random()<.5){return [null,true,false,Math.floor(random()*1e16),'', 'a,b','😀','<script>'][Math.floor(random()*8)];} if(random()<.5)return Array.from({length:Math.floor(random()*5)},()=>value(d+1));const o=Object.create(null);for(let i=0;i<Math.floor(random()*5);i++)o[keys[Math.floor(random()*keys.length)]]=value(d+1);return o;}
for(let i=0;i<1500;i++)fixed.push(JSON.stringify(value()));
const cases=fixed.map(input=>{try{return {input,ok:true,output:encode(JSON.parse(input))};}catch{return {input,ok:false};}});
const nativeCases=cases.map(c=>{if(c.ok && /[\ud800-\udfff]/u.test(c.output)) { const d={...c}; delete d.output; d.utf16Output=Array.from({length:c.output.length},(_,i)=>c.output.charCodeAt(i));return d;}return c;});
const nativeFile=path.join(root,'tests/oracle-native.json');
const nativeBytes=JSON.stringify(nativeCases,null,2)+'\n';
if(!fs.existsSync(nativeFile) && process.argv.includes('--record-native'))fs.writeFileSync(nativeFile,nativeBytes);else assert.equal(fs.readFileSync(nativeFile,'utf8'),nativeBytes);
const destination=path.join(root,'tests/oracle.json');
const bytes=JSON.stringify(cases,null,2)+'\n';
if(process.argv.includes('--record')){if(fs.existsSync(destination))throw Error('Oracle fixture is immutable; refuse overwrite');fs.writeFileSync(destination,bytes);}else assert.equal(fs.readFileSync(destination,'utf8'),bytes);
const binary=process.env.TOON_BINARY||path.join(root,'target/debug/tezbar-json-to-toon-native');
const run=spawnSync(binary,[],{input:cases.map(c=>JSON.stringify(c.input)).join('\n')+'\n',encoding:'utf8',maxBuffer:20*1024*1024});assert.equal(run.status,0,run.stderr);
const replies=run.stdout.trim().split('\n').map(JSON.parse);assert.equal(replies.length,cases.length);
for(let i=0;i<cases.length;i++){const c=cases[i],r=replies[i];if(c.ok && !r.ok && r.error.startsWith('Unpaired UTF-16 surrogate'))continue;assert.equal(r.ok,c.ok,JSON.stringify(c));if(c.ok)assert.equal(r.output,c.output,JSON.stringify(c));}
// Execute the OWN unmodified command (only import resolution is intercepted).
// JSON.parse and the imported encoder must share a realm: TOON tests object
// prototypes. Inject the matching JSON/Error intrinsics instead of accidentally
// turning ordinary objects into null through a VM-only cross-realm boundary.
const source=fs.readFileSync(path.join(root,'fixtures/own/src/convert-json.ts'),'utf8');
const js=ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,target:ts.ScriptTarget.ES2022}}).outputText;
async function command(test){const events=[];const effect=(name)=>async text=>{events.push([name,text]);if(test.fail===name)throw Error('private-secret');};const api={Clipboard:{readText:async()=>{events.push(['read']);if(test.fail==='read')throw Error('private-secret');return test.clipboard??'';},copy:effect('copy'),paste:effect('paste')},getPreferenceValues:()=>test.preferences??{defaultAction:'copy',inputSource:'auto'},getSelectedText:async()=>{events.push(['selection']);if(test.fail==='selection')throw Error('private-secret');return test.selected??'';},showHUD:effect('hud'),showToast:async v=>events.push(['toast',v]),Toast:{Style:{Failure:'failure'}}};const exports={};vm.runInNewContext(js,{exports,JSON,Error,require:name=>name==='@raycast/api'?api:{encode},console:{error:()=>events.push(['log-error'])}});await exports.default();return events;}
const flows=[{}, {selected:'1'}, {selected:'invalid',clipboard:'1'},{selected:'\u00a0\ufeff',clipboard:'{"x":1}'},{fail:'selection',clipboard:'1'},{fail:'read'},{selected:'1',fail:'copy'},{selected:'1',fail:'hud'}, {selected:'1',preferences:{defaultAction:'paste',inputSource:'auto'}},{selected:'1',preferences:{defaultAction:true,inputSource:'auto'}},{clipboard:'1',preferences:{}},{clipboard:'1',preferences:{defaultAction:'copy',inputSource:'clipboard'}},{selected:'1',preferences:{defaultAction:'paste',inputSource:'auto'},fail:'paste'},{selected:'{}'},{selected:'""'},{selected:'\u0085',clipboard:'1'}];
const flowResults=[];for(const input of flows)flowResults.push({input,events:await command(input)});
const flowFile=path.join(root,'tests/source-flows.json');const flowBytes=JSON.stringify(flowResults,null,2)+'\n';
if(process.argv.includes('--record-flows')){if(fs.existsSync(flowFile))throw Error('Flow fixture immutable; refuse overwrite');fs.writeFileSync(flowFile,flowBytes);}else assert.equal(fs.readFileSync(flowFile,'utf8'),flowBytes);
assert.deepEqual(flowResults[2].events.map(x=>x[0]),['selection','log-error','toast']);
assert.deepEqual(flowResults[6].events.map(x=>x[0]),['selection','copy','log-error','toast']);
assert.deepEqual(flowResults[7].events.map(x=>x[0]),['selection','copy','hud','log-error','toast']);
console.log(`PASS ${cases.length} JSON.parse + TOON 1.3.0 cases and ${flows.length} executable OWN command flows`);
