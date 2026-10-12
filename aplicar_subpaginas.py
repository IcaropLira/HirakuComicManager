#!/usr/bin/env python3
"""Aplica o patch de subpáginas no src/index.html do Hiraku Comics Manager.
Uso:  python3 aplicar_subpaginas.py src/index.html
Faz backup em index.html.bak. Se algum trecho não for encontrado, nada é alterado e o script diz qual."""
import sys,shutil
P=sys.argv[1] if len(sys.argv)>1 else "src/index.html"
raw=open(P,"rb").read().decode("utf-8")
crlf="\r\n" in raw
s=raw.replace("\r\n","\n")
FAIL=[]
def idx(x):
    i=s.find(x)
    if i<0:FAIL.append((0,x));return 0
    return i
def rep(old,new,count=1):
    global s
    n=s.count(old)
    if n!=count:
        FAIL.append((n,old[:90].replace("\n","\\n")));return
    s=s.replace(old,new)

# ---------- CSS ----------
css='''.np-row.dragging{opacity:.4}.np-row.over>button:first-of-type,.np-list.over{outline:2px dashed var(--acc);outline-offset:-2px}
.np-icon[data-np-toggle]{cursor:pointer;width:14px;text-align:center}.np-icon[data-np-toggle]:hover{color:var(--ink)}
.np-add{opacity:0;border:0;border-radius:4px;background:none;color:var(--mut);font-size:15px;cursor:pointer}.np-row:hover .np-add,.np-add:focus-visible{opacity:1}.np-add:hover{color:var(--acc);background:var(--bg)}
.np-crumbs .crumb{color:var(--mut)}.np-crumbs .crumb:hover{color:var(--acc)}.np-crumbs b{color:var(--ink);font-weight:600}
.nb .sp{display:inline-flex;flex:1;align-items:center;gap:8px;min-width:0;margin:2px 0;padding:6px 10px;border:1px solid var(--ln);border-radius:6px;background:var(--pn);cursor:pointer;user-select:none}
.nb .sp:hover{border-color:var(--acc)}.nb .sp b{font-weight:600}.nb .sp small{color:var(--mut)}.nb .sp.bad{opacity:.55;border-style:dashed}
.nl-sp{margin:6px 0}
'''
rep('</style></head><body>',css+'</style></head><body>')

# ---------- Páginas (localStorage) ----------
rep('let NPSEL=localStorage.getItem("terricola.pages.selected")||"";',
    'let NPSEL=localStorage.getItem("terricola.pages.selected")||"";\nlet NPCOL=new Set();try{NPCOL=new Set(JSON.parse(localStorage.getItem("terricola.pages.collapsed")||"[]"))}catch(e){}')
rep('localStorage.setItem("terricola.pages.selected",NPSEL)}catch(e){}}',
    'localStorage.setItem("terricola.pages.selected",NPSEL);localStorage.setItem("terricola.pages.collapsed",JSON.stringify([...NPCOL]))}catch(e){}}')
rep('if(path)path.textContent=npPath(p);','if(path)path.innerHTML=npCrumbs(p);')

# npTreeRow + novos helpers
a=idx('function npTreeRow(');b=idx('function npPath(')
newrow='''function npTreeRow(p,depth,pages){const children=pages.filter(x=>x.parentId===p.id),open=!NPCOL.has(p.id),t=esc(p.title||"Sem título"),
 row=`<div class="np-row ${p.id===NPSEL?"active":""}" data-id="${esc(p.id)}" draggable="true" style="--depth:${depth}"><button data-np-open="${esc(p.id)}"><span class="np-icon"${children.length?` data-np-toggle="${esc(p.id)}" title="Expandir ou recolher"`:""}>${children.length?(open?"▾":"▸"):"▤"}</span><span class="np-label"><b>${t}</b><small>${npTime(p.updated)}</small></span></button><button class="np-add" data-np-child="${esc(p.id)}" aria-label="Nova subpágina em ${t}" title="Nova subpágina">＋</button><button class="np-delete" data-np-delete="${esc(p.id)}" aria-label="Excluir ${t}" title="Excluir página">×</button></div>`;
 return row+(open?children.map(x=>npTreeRow(x,depth+1,pages)).join(""):"")}
'''
s=s[:a]+newrow+s[b:]

rep('function notionPages(){','''function npCrumbs(page){const parts=[],seen=new Set();let q=page;while(q&&!seen.has(q.id)){seen.add(q.id);parts.unshift(q);q=NPAGES.find(x=>x.id===q.parentId)}
 return parts.map((x,i)=>i===parts.length-1?`<b>${esc(x.title||"Sem título")}</b>`:`<span class="crumb" data-np-open="${esc(x.id)}">${esc(x.title||"Sem título")}</span> / `).join("")}
const npRef=id=>"▤ [[np:"+id+"]]";
function npLinkAdd(pid,cid){const p=NPAGES.find(x=>x.id===pid);if(!p)return;const c=p.content||"";if(c.includes(npRef(cid)))return;p.content=c.replace(/\\s+$/,"")+(c.trim()?"\\n":"")+npRef(cid)+"\\n"}
function npLinkDel(pid,cid){const p=NPAGES.find(x=>x.id===pid);if(p&&p.content)p.content=p.content.split("\\n").filter(l=>l.trim()!==npRef(cid)).join("\\n")}
function npMake(parentId,title){const id=Date.now().toString(36)+Math.random().toString(36).slice(2,7),p={id,parentId:parentId||"",title:title||"",content:"",updated:Date.now()};NPAGES.unshift(p);return p}
function npMove(id,to){const p=NPAGES.find(x=>x.id===id);to=to||"";if(!p||p.parentId===to||id===to)return;
 for(let q=NPAGES.find(x=>x.id===to),n=0;q&&n<500;q=NPAGES.find(x=>x.id===q.parentId),n++)if(q.id===id)return;
 if(p.parentId)npLinkDel(p.parentId,id);p.parentId=to;if(to){npLinkAdd(to,id);NPCOL.delete(to)}p.updated=Date.now();npSave();render()}
function notionPages(){''')

rep('${esc(npPath(selected))}</span><span class="np-status"','${npCrumbs(selected)}</span><span class="np-status"')
rep(' const rows=roots.map(p=>npTreeRow(p,0,pages)).join("");',
    ' for(let q=selected&&NPAGES.find(x=>x.id===selected.parentId),n=0;q&&n<500;q=NPAGES.find(x=>x.id===q.parentId),n++)NPCOL.delete(q.id);\n const rows=roots.map(p=>npTreeRow(p,0,pages)).join("");')
rep('bmount($("#np-be"),selected.content||"",md=>{selected.content=md;npTouch(selected)},{});','bmount($("#np-be"),selected.content||"",md=>{selected.content=md;npTouch(selected)},{np:true});')

# npCreate / npRemove
a=idx('function npCreate(');b=idx('function npRemove(')
s=s[:a]+'''function npCreate(parentId=""){const p=npMake(parentId);if(parentId){npLinkAdd(parentId,p.id);NPCOL.delete(parentId)}NPSEL=p.id;npSave();f.view="pages";render();const title=$("#np-title");if(title){title.focus();title.select()}}
'''+s[b:]
rep('NPAGES=NPAGES.filter(p=>!removed.has(p.id));',
    'NPAGES.forEach(p=>{if(!removed.has(p.id)&&p.content)p.content=p.content.split("\\n").filter(l=>![...removed].some(id=>l.trim()===npRef(id))).join("\\n")});NPAGES=NPAGES.filter(p=>!removed.has(p.id));')

# click handler
rep('const t=e.target.closest("[data-np-new],[data-np-open],[data-np-delete]");if(!t)return;',
    'const t=e.target.closest("[data-np-new],[data-np-open],[data-np-delete],[data-np-child],[data-np-toggle]");if(!t)return;\n if(t.dataset.npToggle){const id=t.dataset.npToggle;NPCOL.has(id)?NPCOL.delete(id):NPCOL.add(id);npSave();render();return}\n if(t.dataset.npChild){npCreate(t.dataset.npChild);return}')

# drag & drop de páginas (aninhar arrastando)
rep('$("#view").addEventListener("input",e=>{const t=e.target;\n if(t.id==="np-title")','''let NPDRAG="";
document.addEventListener("dragstart",e=>{const r=e.target.closest&&e.target.closest(".np-row");if(!r)return;NPDRAG=r.dataset.id;e.dataTransfer.effectAllowed="move";e.dataTransfer.setData("text/plain","np");r.classList.add("dragging")});
document.addEventListener("dragover",e=>{if(!NPDRAG)return;const z=e.target.closest&&e.target.closest(".np-list");if(!z)return;e.preventDefault();document.querySelectorAll(".np-row.over,.np-list.over").forEach(x=>x.classList.remove("over"));(e.target.closest(".np-row")||z).classList.add("over")});
document.addEventListener("drop",e=>{if(!NPDRAG)return;const z=e.target.closest&&e.target.closest(".np-list");if(!z)return;e.preventDefault();const r=e.target.closest(".np-row"),id=NPDRAG;NPDRAG="";npMove(id,r?r.dataset.id:"")});
document.addEventListener("dragend",()=>{NPDRAG="";document.querySelectorAll(".np-row.over,.np-list.over,.np-row.dragging").forEach(x=>x.classList.remove("over","dragging"))});
$("#view").addEventListener("input",e=>{const t=e.target;\n if(t.id==="np-title")''')

# ---------- Editor de blocos: bloco "Subpágina" ----------
rep('["img","🖼","Imagem"]];','["img","🖼","Imagem"],["subpg","▤","Subpágina"]];')
rep('pag:"page pagina",','pag:"page pagina",subpg:"page pagina subpagina subpage",')
rep('const BIMG=/','const btypes=S=>BT.filter(x=>x[0]!=="subpg"||(S.ctx&&(S.ctx.np||S.ctx.host))).concat(S.ctx&&S.ctx.script?SCRIPTBT:[]);\nconst BIMG=/')
rep('  else if((x=l.match(/^!\\[\\[','  else if(x=l.match(/^▤ \\[\\[([^\\]|]+)(?:\\|[^\\]]*)?\\]\\]\\s*$/))B.push({t:"subpg",txt:x[1].trim(),ind:dep});\n  else if((x=l.match(/^!\\[\\[')
rep('img:()=>"![["+t+"]]",','img:()=>"![["+t+"]]",subpg:()=>"▤ [["+t+"]]",')
rep('types=BT.concat(S.ctx&&S.ctx.script?SCRIPTBT:[]),it=','types=btypes(S),it=')
rep('types=BT.concat(g.S.ctx&&g.S.ctx.script?SCRIPTBT:[])','types=btypes(g.S)')
rep('ed=b.t!=="div"&&b.t!=="img",types=BT.concat(S.ctx&&S.ctx.script?SCRIPTBT:[])','ed=b.t!=="div"&&b.t!=="img"&&b.t!=="subpg",types=btypes(S)')
rep('if(["code","eq"].includes(p.t))return;','if(["code","eq","subpg"].includes(p.t))return;')
rep('let body=b.t==="div"?"<hr>":','let body=b.t==="subpg"?bspHtml(S,b):b.t==="div"?"<hr>":')
rep('if(ctx&&ctx.script&&((B[B.length-1].t!=="text")||B[B.length-1].txt))B.push','if(B[B.length-1].t==="subpg"||(ctx&&ctx.script&&((B[B.length-1].t!=="text")||B[B.length-1].txt)))B.push')
rep('if(t==="img"){bpick(S,i);return}','if(t==="subpg"){bsubAsk(el,S,i);return}\n if(t==="img"){bpick(S,i);return}')
rep('bmount(el,pre?ED.pre:s.body,','bmount(el,pre?ED.pre:s.body,')  # noop sanity
rep('{dir:ED.path.split("/").slice(0,-1).join("/")}','{dir:ED.path.split("/").slice(0,-1).join("/"),host:ED.path}')
rep('outline.innerHTML=houtline(D,A)},{script:true});','outline.innerHTML=houtline(D,A)},{script:true,host:D.path});')

# layout de leitura
rep('  else if(b.t==="div")o+="<hr>";','  else if(b.t==="div")o+="<hr>";\n  else if(b.t==="subpg")o+=`<p class="nl-sp">▤ <span class="wl" data-l="${esc(b.txt)}">${esc(b.txt)}</span></p>`;')
rep(' if(b.t==="sfx")return`<div class="ld"><b>SFX</b>',' if(b.t==="subpg"){const k=kids(D.path).find(x=>x.name===b.txt);return`<div class="ln">▤ ${k?`<span class="crumb" data-sub-open="${esc(k.path)}">${esc(b.txt)}</span>`:esc(b.txt)}</div>`}\n if(b.t==="sfx")return`<div class="ld"><b>SFX</b>')

# ---------- lógica do bloco Subpágina ----------
logic='''/* ---- bloco Subpágina (/pagina): cria e abre páginas dentro de páginas, em qualquer editor ---- */
function bspInfo(S,b){const id=b.txt||"";
 if(id.startsWith("np:")){const p=NPAGES.find(x=>x.id===id.slice(3));return p?{ok:1,t:p.title||"Sem título",n:NPAGES.filter(x=>x.parentId===p.id).length}:{ok:0,t:"Página removida",n:0,np:1}}
 const h=S.ctx&&S.ctx.host,k=h&&VT?kids(h).find(x=>x.name===id):null;
 return k?{ok:1,t:id,n:kids(k.path).length}:{ok:0,t:id,n:0}}
function bspHtml(S,b){const k=bspInfo(S,b);
 return`<span class="sp${k.ok?"":" bad"}" contenteditable="false" data-sp="${esc(b.txt)}">▤ <b>${esc(k.t)}</b>${k.n?`<small>· ${k.n} subpágina${k.n>1?"s":""}</small>`:""}${k.ok||k.np?"":"<small>· não encontrada</small>"}</span>`}
async function bhostSave(){if(!window.__TAURI__)return;if(f.view==="editar"&&ED&&ED.dirty)await esave();else if(f.view==="hce"&&HD&&HD.dirty)await hsave()}
function bsubNp(el,S,i){const b=S.B[i],p=npMake(NPSEL,b.txt);b.t="subpg";b.txt="np:"+p.id;bsync(S);NPSEL=p.id;npSave();f.view="pages";render();const t=$("#np-title");if(t){t.focus();t.select()}}
function bsubAsk(el,S,i){const b=S.B[i];
 if(S.ctx&&S.ctx.np)return bsubNp(el,S,i);
 if(!window.__TAURI__||!ROOTABS){bnote("Subpáginas em arquivo funcionam no app desktop, com o Vault aberto.");brender(el);return}
 const nb=el.querySelector(`.nb[data-i="${i}"]`),r=(nb||el).getBoundingClientRect();
 bmRaw(r.left+30,r.bottom+4,`<h6>Nova subpágina</h6><div style="padding:4px 8px 8px"><input id="sp-name" placeholder="Nome da subpágina" value="${esc(b.txt||"")}" style="width:230px;font:inherit;padding:5px 8px;border:1px solid var(--ln);border-radius:5px;background:var(--bg);color:var(--ink)"><div class="mut" id="sp-msg" style="margin-top:4px;font-size:11px">Enter cria · Esc cancela</div></div>`,{el,S,i,sub:1});
 setTimeout(()=>{const x=$("#sp-name");if(x){x.focus();x.select()}})}
async function bsubMake(el,S,i,name){const b=S.B[i],host=S.ctx&&S.ctx.host;if(!host)return;
 const path=subdir(host)+"/"+name+".md";b.t="subpg";b.txt=name;brender(el);bsync(S);
 try{await bhostSave();
  if(!VT.map.has(path)){const text="---\\ntipo: sub\\n---\\n\\n### Notas\\n\\n";
   await __TAURI__.core.invoke("write_md",{path:ROOTABS+"/"+path.split("/").slice(1).join("/"),text});
   RF.push({name:path.split("/").pop(),webkitRelativePath:path,_t:text,text(){return Promise.resolve(this._t)}});
   VT=await vload(RF);vapply();await gitAuto(path)}
  subopen(path)}
 catch(e){bnote("Não consegui criar a subpágina: "+e)}}
document.addEventListener("keydown",e=>{if(e.target.id!=="sp-name")return;
 if(e.key==="Escape"){bmHide();return}if(e.key!=="Enter")return;e.preventDefault();
 const c=BM&&BM.ctx,name=(e.target.value||"").replace(/[\\\\/:*?"<>|]/g,"").trim();if(!c||!c.sub)return;
 if(!name){const m=$("#sp-msg");if(m)m.textContent="Digite um nome.";return}
 bmHide();bsubMake(c.el,c.S,c.i,name)});
document.addEventListener("click",async e=>{const sp=e.target.closest&&e.target.closest(".be .sp");if(!sp)return;const{S}=bget(sp),id=sp.dataset.sp;
 if(id.startsWith("np:")){const pid=id.slice(3);if(NPAGES.some(x=>x.id===pid)){NPSEL=pid;npSave();f.view="pages";render()}return}
 const h=S.ctx&&S.ctx.host,k=h&&VT?kids(h).find(x=>x.name===id):null;if(!k)return bnote("Subpágina não encontrada. Salve a página-mãe e tente de novo.");
 await bhostSave();subopen(k.path)});
'''
rep('function bmountAll(){',logic+'function bmountAll(){')
# tira a reexpansão automática a cada render; passa a revelar só ao navegar
rep(' for(let q=selected&&NPAGES.find(x=>x.id===selected.parentId),n=0;q&&n<500;q=NPAGES.find(x=>x.id===q.parentId),n++)NPCOL.delete(q.id);\n','')
rep('const npRef=id=>','function npReveal(id){const p=NPAGES.find(x=>x.id===id);for(let q=p&&NPAGES.find(x=>x.id===p.parentId),n=0;q&&n<500;q=NPAGES.find(x=>x.id===q.parentId),n++)NPCOL.delete(q.id)}\nconst npRef=id=>')
rep('if(t.dataset.npOpen){NPSEL=t.dataset.npOpen;npSave();','if(t.dataset.npOpen){NPSEL=t.dataset.npOpen;npReveal(NPSEL);npSave();')
rep('if(NPAGES.some(x=>x.id===pid)){NPSEL=pid;npSave();','if(NPAGES.some(x=>x.id===pid)){NPSEL=pid;npReveal(pid);npSave();')
rep('bsync(S);NPSEL=p.id;npSave();f.view="pages";render();','bsync(S);NPSEL=p.id;npReveal(p.id);npSave();f.view="pages";render();')
rep('function npCreate(parentId=""){const p=npMake(parentId);if(parentId){npLinkAdd(parentId,p.id);NPCOL.delete(parentId)}','function npCreate(parentId=""){const p=npMake(parentId);if(parentId){npLinkAdd(parentId,p.id);NPCOL.delete(parentId);npReveal(parentId)}')

if FAIL:
    print("NADA FOI ALTERADO. Trechos não encontrados (seu index.html difere do que eu recebi):")
    for n,t in FAIL:print(f"  - achei {n}x: {t}")
    sys.exit(1)
shutil.copy(P,P+".bak")
if crlf:s=s.replace("\n","\r\n")
open(P,"wb").write(s.encode("utf-8"))
print("Patch aplicado. Backup em "+P+".bak")
