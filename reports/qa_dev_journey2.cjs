const { chromium } = require('D:/Work Studio/Bos/ui/node_modules/playwright');
const out='D:/Work Studio/Bos/reports/qa_dev_journey_screens/'; const r={};
(async()=>{const b=await chromium.launch({channel:'msedge'});const p=await (await b.newContext({viewport:{width:1440,height:900}})).newPage();
 const codes=[]; p.on('response',s=>{if(s.status()>=400)codes.push(s.status()+' '+s.request().method()+' '+s.url())});
 await p.goto('http://localhost:5173/',{waitUntil:'networkidle'});
 r.overlay=await p.evaluate(()=>{const o=document.querySelector('vite-error-overlay');return o&&o.shadowRoot?o.shadowRoot.textContent.replace(/\s+/g,' ').slice(0,1200):null});
 await p.keyboard.press('Escape'); await p.evaluate(()=>document.querySelector('vite-error-overlay')?.remove());
 await p.fill('#fb-user','emergency.admin'); let t=Date.now(); await p.click('button[type=submit]'); await p.waitForTimeout(2500); r.loginMs=Date.now()-t;
 await p.screenshot({path:out+'06_login_fallback_form_emergency_nopass.png'}); r.loginText=(await p.evaluate(()=>document.body.innerText)).slice(0,400);
 await p.goto('http://localhost:5173/dev-studio.html',{waitUntil:'networkidle'});
 const tries=[['07_create_meta_object','text=Create Meta-Object'],['08_add_object','text=+ Object'],['09_build_menu','#bits-c9'],['10_database_menu','#bits-c7'],['11_ctrl_k',null]];
 for(const [n,sel] of tries){try{ t=Date.now(); if(sel) await p.click(sel,{timeout:5000}); else await p.keyboard.press('Control+K'); await p.waitForTimeout(1200);
   r[n]={ms:Date.now()-t,text:(await p.evaluate(()=>{const d=document.querySelector('[role=dialog],[role=menu]');return (d||document.body).innerText})).slice(0,500)};
   await p.screenshot({path:out+n+'.png'});
   if(n==='07_create_meta_object'){ const ta=await p.$('textarea, .cm-content, .monaco-editor textarea'); r[n].editor=!!ta;
     const btns=await p.$$eval('[role=dialog] button',e=>e.map(x=>x.innerText.trim()).filter(Boolean)); r[n].buttons=btns;}
   await p.keyboard.press('Escape'); await p.waitForTimeout(400);}catch(e){r[n]='ERR '+e.message.slice(0,150)}}
 r.codes=codes; console.log(JSON.stringify(r,null,1)); await b.close();})();
