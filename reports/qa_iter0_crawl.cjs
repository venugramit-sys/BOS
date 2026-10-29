const { chromium } = require('D:/Work Studio/Bos/ui/node_modules/playwright');
const fs=require('fs'); const out='D:/Work Studio/Bos/reports/qa_iter0_screens/';
(async()=>{
 const b=await chromium.launch({channel:'msedge'}); const res=[];
 const pages=['/','/index.html','/dev-studio.html','/api-studio.html','/showcase.html','/does-not-exist'];
 for(const path of pages){
  const ctx=await b.newContext({viewport:{width:1440,height:900}}); const p=await ctx.newPage();
  const r={path,console:[],failed:[],http:[]};
  p.on('console',m=>{if(['error','warning'].includes(m.type()))r.console.push(m.type()+': '+m.text().slice(0,300))});
  p.on('pageerror',e=>r.console.push('pageerror: '+e.message.slice(0,300)));
  p.on('requestfailed',q=>r.failed.push(q.url()+' '+(q.failure()||{}).errorText));
  p.on('response',s=>{if(s.status()>=400)r.http.push(s.status()+' '+s.url())});
  const t=Date.now();
  try{await p.goto('http://localhost:5173'+path,{waitUntil:'networkidle',timeout:30000});}catch(e){r.console.push('goto: '+e.message.slice(0,200))}
  r.loadMs=Date.now()-t; await p.waitForTimeout(1500);
  const n=path.replace(/[^a-z0-9]/gi,'_')||'root';
  await p.screenshot({path:out+n+'.png',fullPage:true});
  r.title=await p.title(); r.text=(await p.evaluate(()=>document.body.innerText)).slice(0,600);
  // click nav items
  const items=await p.$$('nav a, nav button, [role=tab], [role=menuitem], aside button');
  r.navCount=items.length; r.clicks=[];
  for(let i=0;i<Math.min(items.length,25);i++){
   try{const all=await p.$$('nav a, nav button, [role=tab], [role=menuitem], aside button'); if(!all[i])break;
    const label=((await all[i].innerText())||(await all[i].getAttribute('aria-label'))||'').trim().slice(0,40);
    if(/delete|drop|reset|sign ?out|logout/i.test(label)){r.clicks.push('SKIP '+label);continue}
    await all[i].click({timeout:3000}); await p.waitForTimeout(700);
    await p.screenshot({path:out+n+'_nav'+i+'.png'}); r.clicks.push(i+': '+label);
    await p.keyboard.press('Escape');
   }catch(e){r.clicks.push(i+' ERR '+e.message.slice(0,100))}
  }
  res.push(r); await ctx.close();
 }
 // mobile
 const m=await b.newContext({viewport:{width:390,height:844}}); const mp=await m.newPage();
 await mp.goto('http://localhost:5173/',{waitUntil:'networkidle'}).catch(()=>{}); await mp.screenshot({path:out+'root_mobile.png',fullPage:true});
 fs.writeFileSync('D:/Work Studio/Bos/reports/qa_iter0_ui.json',JSON.stringify(res,null,1)); await b.close();
})();

