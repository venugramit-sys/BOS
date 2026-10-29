const { chromium } = require('D:/Work Studio/Bos/ui/node_modules/playwright');
const fs=require('fs'); const out='D:/Work Studio/Bos/reports/qa_dev_journey_screens/'; const B='http://localhost:5173';
const log={steps:[],console:[],failed:[],http:[]};
(async()=>{
 const b=await chromium.launch({channel:'msedge'}); const ctx=await b.newContext({viewport:{width:1440,height:900}}); const p=await ctx.newPage();
 p.on('console',m=>{if(['error','warning'].includes(m.type()))log.console.push(m.type()+': '+m.text().slice(0,300))});
 p.on('pageerror',e=>log.console.push('pageerror: '+e.message.slice(0,300)));
 p.on('requestfailed',q=>log.failed.push(q.url()+' '+(q.failure()||{}).errorText));
 p.on('response',s=>{if(s.status()>=400)log.http.push(s.status()+' '+s.request().method()+' '+s.url())});
 const step=async(name,fn)=>{const t=Date.now();let err=null;try{await fn()}catch(e){err=e.message.slice(0,300)}
   const ms=Date.now()-t; await p.screenshot({path:out+name+'.png',fullPage:true}).catch(()=>{});
   const text=(await p.evaluate(()=>document.body.innerText).catch(()=>'')).slice(0,800);
   log.steps.push({name,ms,err,url:p.url(),text});};
 await step('01_first_load',async()=>{await p.goto(B+'/',{waitUntil:'networkidle',timeout:60000})});
 const ctrls=await p.$$eval('input,button,a,select,textarea',els=>els.map(e=>({tag:e.tagName,type:e.type,name:e.name||e.id||e.placeholder,text:(e.innerText||e.value||'').trim().slice(0,40),href:e.href||''})));
 log.controls_root=ctrls.slice(0,80);
 for(const path of ['/dev-studio.html','/api-studio.html','/showcase.html']){
   await step('02_'+path.replace(/\W/g,'_'),async()=>{await p.goto(B+path,{waitUntil:'networkidle',timeout:60000})});
   log['controls'+path]=(await p.$$eval('input,button,a,select,textarea',els=>els.map(e=>(e.tagName+':'+(e.name||e.id||e.placeholder||'')+':'+(e.innerText||'').trim().slice(0,30))))).slice(0,80);
 }
 // login attempt with documented username only (no password documented)
 await p.goto(B+'/',{waitUntil:'networkidle'});
 await step('03_login_empty_submit',async()=>{const btn=await p.$('button[type=submit], form button'); if(!btn) throw new Error('no login button'); await btn.click(); await p.waitForTimeout(1500)});
 await step('04_login_emergency_nopass',async()=>{const u=await p.$('input[type=text],input[name*=user i],input[autocomplete=username]'); if(!u) throw new Error('no username field'); await u.fill('emergency.admin'); const btn=await p.$('button[type=submit], form button'); await btn.click(); await p.waitForTimeout(2000)});
 // direct protected API calls from browser
 log.api=await p.evaluate(async()=>{const r={};for(const [m,u,bd] of [['GET','/api/auth/status'],['GET','/api/entities'],['POST','/api/roles',{code:'QA_DEV',name:'QA Dev'}],['POST','/api/compile',{source:'table 50000 X {'}],['GET','/api/packages']]){try{const x=await fetch(u,{method:m,headers:{'content-type':'application/json'},body:bd?JSON.stringify(bd):undefined});r[m+' '+u]=x.status+' '+(await x.text()).slice(0,200)}catch(e){r[m+' '+u]='ERR '+e.message}}return r});
 await step('05_mobile',async()=>{await p.setViewportSize({width:390,height:844}); await p.goto(B+'/',{waitUntil:'networkidle'})});
 fs.writeFileSync('D:/Work Studio/Bos/reports/qa_dev_journey_log.json',JSON.stringify(log,null,1)); await b.close();
})();
