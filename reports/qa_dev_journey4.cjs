const { chromium } = require('D:/Work Studio/Bos/ui/node_modules/playwright');
const out='D:/Work Studio/Bos/reports/qa_dev_journey_screens/'; const r={codes:[]};
(async()=>{const b=await chromium.launch({channel:'msedge'});const p=await (await b.newContext({viewport:{width:1440,height:900}})).newPage();
 p.on('response',s=>{if(s.status()>=400)r.codes.push(s.status()+' '+s.request().method()+' '+s.url())});
 await p.goto('http://localhost:5173/dev-studio.html',{waitUntil:'networkidle'});
 await p.click('text=Create Meta-Object'); await p.waitForTimeout(800);
 await p.click('[role=dialog] button:has-text("Table")'); 
 const inp=await p.$('[role=dialog] input'); await inp.fill('QaItem'); await p.screenshot({path:out+'12_create_table_filled.png'});
 let t=Date.now(); await p.click('text=Create & Open'); await p.waitForTimeout(3000); r.ms=Date.now()-t;
 await p.screenshot({path:out+'13_create_table_result.png'}); r.text=(await p.evaluate(()=>document.body.innerText)).slice(0,700);
 await p.click('text=Code').catch(e=>r.codeErr=e.message.slice(0,100)); await p.waitForTimeout(1200); await p.screenshot({path:out+'14_code_tab.png'}); r.code=(await p.evaluate(()=>document.body.innerText)).slice(0,500); const ed=await p.$('.cm-content, .monaco-editor textarea'); if(ed){await ed.click({force:true}); await p.keyboard.press('Control+End'); await p.keyboard.type('\n trigger OnInsert() { let x = ;; }\n table broken {{'); await p.waitForTimeout(1500); await p.screenshot({path:out+'15_syntax_error_typed.png'}); r.diag=await p.$$eval('.squiggly-error,.cm-diagnostic,.cm-lintRange,[class*=diagnos],[class*=error],[role=alert]',e=>e.map(x=>x.innerText.slice(0,120)).slice(0,10)); t=Date.now(); await p.click('text=Save & Apply').catch(e=>r.applyErr=e.message.slice(0,100)); await p.waitForTimeout(3000); r.applyMs=Date.now()-t; await p.screenshot({path:out+'16_save_apply_result.png'}); r.after=await p.$$eval('[role=alert],[role=status],[data-sonner-toast],.toast',e=>e.map(x=>x.innerText.slice(0,200)));} else r.noEditor=true;
console.log(JSON.stringify(r,null,1)); await b.close();})();


