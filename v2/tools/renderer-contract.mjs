// Executes actual pinned Surface/Detail/action-overlay code with headless React hooks.
// This verifies callback/parser contracts, not browser layout or desktop keyboard QA.
import fs from 'node:fs';import path from 'node:path';import vm from 'node:vm';import assert from 'node:assert/strict';import {createRequire}from'node:module';import{fileURLToPath}from'node:url';
const root=path.resolve(path.dirname(fileURLToPath(import.meta.url)),'..');
const require=createRequire(import.meta.url);const ts=require(path.join(root,'.work/v1/node_modules/typescript'));
let listeners=[];let states=[];const effects=[];
const jsx=(type,props)=>({type,props:props??{}});
const react={useState:initial=>[states.length?states.shift():initial,()=>{}],useEffect:f=>effects.push(f),useMemo:f=>f(),useCallback:f=>f,useRef:v=>({current:v}),createContext:()=>({Provider:'Provider'}),useContext:()=>({actions:[]})};
const marker=name=>Object.assign(()=>null,{marker:name});
const Markdown=marker('Markdown');
const stubs={'react':react,'react/jsx-runtime':{jsx,jsxs:jsx,Fragment:'Fragment'},'../../colorConverter':{},'../../ui/Markdown':{Markdown},'../../ui/primitives':{cx:()=>''},'./form-runtime':{FormRuntime:marker('Form')},'./grid-runtime':{GridRuntime:marker('Grid')},'./list-runtime':{ListRuntime:marker('List')},'./hooks':{NavigationContext:{Provider:'NavProvider'},setRuntimeContext:()=>{}}};
function load(source,imports={}){const exports={};vm.runInNewContext(ts.transpileModule(source,{compilerOptions:{module:ts.ModuleKind.CommonJS,jsx:ts.JsxEmit.ReactJSX,target:ts.ScriptTarget.ES2022}}).outputText,{exports,require:name=>imports[name]??stubs[name]??{},window:{addEventListener:(name,cb)=>listeners.push([name,cb]),removeEventListener:()=>{}},console});return exports;}
const dir=path.join(root,'fixtures/beta/src/renderer/src/raycast-api');
const registry=load(fs.readFileSync(path.join(dir,'action-runtime-registry.tsx'),'utf8'));
const overlay=load(fs.readFileSync(path.join(dir,'action-runtime-overlay.tsx'),'utf8'));
const action=load(fs.readFileSync(path.join(dir,'action-runtime.tsx'),'utf8'),{'./action-runtime-overlay':overlay});
const detail=load(fs.readFileSync(path.join(dir,'detail-runtime.tsx'),'utf8'));
const dependencies={'./detail-runtime':detail,'./action-runtime-registry':registry,'./action-runtime':action};
const baseline=load(fs.readFileSync(path.join(dir,'index.tsx'),'utf8'),dependencies);
const patched=load(fs.readFileSync(path.join(root,'.work/patch-check-beta/src/renderer/src/raycast-api/index.tsx'),'utf8'),dependencies);
function walk(n,predicate){if(Array.isArray(n)){for(const child of n){const r=walk(child,predicate);if(r)return r;}return null;}if(!n||typeof n!=='object')return null;if(predicate(n))return n;const children=n.props?.children;for(const child of Array.isArray(children)?children:[children]){const r=walk(child,predicate);if(r)return r;}return null;}
const views=JSON.parse(fs.readFileSync(path.join(root,'tests/surfaces.json'),'utf8'));
assert.equal(views.length,2);assert.equal(views[0].mode,'view');assert.ok(views[0].message.includes('unconfirmed'));
assert.deepEqual(views[1],{ok:true,mode:'no-view',message:'Converted! TOON format copied to clipboard'});
for(const surface of [baseline,patched]){
 for(const v of views.filter(v=>v.mode==='view')){let calls=[];effects.length=0;listeners=[];states=[];
 const props={...v,sessionId:'test-session',extensionId:'test-extension',commandName:'convert-json',onBack:()=>{},onSearchTextChanged:()=>{},onLoadMore:()=>{},onInvokeAction:(...args)=>calls.push(args)};
 const tree=surface.ExtensionRuntimeSurface(props);for(const f of effects.splice(0))f();
 const d=walk(tree,n=>n.type===detail.DetailRuntime);assert.ok(d,'actual rootKind selects Detail');
 const rendered=detail.DetailRuntime(d.props);const markdown=walk(rendered,n=>n.type===Markdown);assert.equal(markdown.props.text,v.root.props.markdown);assert.ok(!markdown.props.text.includes('private'));assert.ok(!markdown.props.text.includes('```'));
 for(const[,key]of listeners)key({key:'Enter',preventDefault:()=>{}});
 assert.equal(calls.length,v.actions.length?1:0);if(v.actions.length)assert.equal(calls[0][0],v.actions[0].id);
 const enter=walk(rendered,n=>n.type==='button'&&n.props.children==='Enter');calls=[];const event={type:'click'};enter.props.onClick(event);
 if(v.actions.length){if(surface===baseline)assert.equal(calls[0][0],event,'baseline click leaks MouseEvent as actionId');else assert.equal(calls[0][0],v.actions[0].id,'prerequisite fixes actual button contract');}
 }
}
let executed=[];effects.length=0;listeners=[];states=[];
const tree=overlay.ActionPanelOverlay({actions:views[0].actions,onClose:()=>{},onExecute:a=>executed.push(a.id)});for(const f of effects.splice(0))f();
const retry=walk(tree,n=>n.type==='button');retry.props.onClick();assert.equal(executed[0],views[0].actions[0].id);
for(const[,key]of listeners)key({key:'Enter',preventDefault:()=>{}});assert.equal(executed[1],views[0].actions[0].id);
const latest=fs.readFileSync(path.join(root,'fixtures/current/src/renderer/extensionImages.ts'),'utf8');const fixed=fs.readFileSync(path.join(root,'.work/patch-check-current/src/renderer/extensionImages.ts'),'utf8');
const a=load(latest),b=load(fixed),url='/extensions/icons/json-to-toon-converter.png';assert.notEqual(a.extensionImageSource(url),url);assert.equal(b.extensionImageSource(url),url);for(const other of ['/tmp/private.png','/extensions/icons/other.png','https://example.com/a.png'])assert.equal(b.extensionImageSource(other),a.extensionImageSource(other));
console.log('PASS pinned Surface/Detail/overlay callbacks; baseline click defect reproduced, exact prerequisite fixes it; current icon URL defect and narrow fix verified');
