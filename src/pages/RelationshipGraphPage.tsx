import { useEffect,useMemo,useRef,useState } from "react";
import type { Paper } from "../domain/paper/Paper";
import { graphLabel,type GraphMode,type RelationshipGraph } from "../domain/paper/RelationshipGraph";
import { RelationshipGraphService } from "../services/RelationshipGraphService";
import { InlineTaskProgress } from "../components/TaskProgress";
import { taskId } from "../services/ProgressService";
import { normalizeAppError } from "../types/errors";
import { GraphIcon } from "../components/Icons";

interface Props { service:RelationshipGraphService; onPaper(id:string):void }
export function RelationshipGraphPage({service,onPaper}:Props) {
  const [mode,setMode]=useState<GraphMode>("keywords");const [maxNodes,setMaxNodes]=useState(40);const [minShared,setMinShared]=useState(1);const [includeAuthors,setIncludeAuthors]=useState(true);
  const [graph,setGraph]=useState<RelationshipGraph>();const [papers,setPapers]=useState<Paper[]>([]);const [selected,setSelected]=useState<string>();const [selectedEdge,setSelectedEdge]=useState<string>();
  const [busy,setBusy]=useState(false);const [error,setError]=useState<string>();const [requestId,setRequestId]=useState<string>();const [exporting,setExporting]=useState(false);const [exportId,setExportId]=useState<string>();const [notice,setNotice]=useState<string>();
  const [query,setQuery]=useState("");const [zoom,setZoom]=useState(1);const [pan,setPan]=useState({x:0,y:0});
  const drag=useRef<{x:number;y:number;px:number;py:number;scale:number}|undefined>(undefined);const svg=useRef<SVGSVGElement>(null);const generation=useRef(0);
  async function build() {
    const revision=++generation.current;const id=taskId("graph");setRequestId(id);setBusy(true);setError(undefined);setNotice(undefined);
    try {const result=await service.build({mode,maxNodes,minShared,includeAuthors},id);if(revision!==generation.current)return;setGraph(result.graph);setPapers(result.papers);setSelected(undefined);setSelectedEdge(undefined);setZoom(1);setPan({x:0,y:0});}
    catch(error){if(revision===generation.current)setError(normalizeAppError(error).message);}
    finally{if(revision===generation.current)setBusy(false);}
  }
  useEffect(()=>{void build();return()=>{generation.current++;};},[service,mode,maxNodes,minShared,includeAuthors]);
  const byId=useMemo(()=>new Map(graph?.nodes.map(node=>[node.id,node])??[]),[graph]);
  const node=selected ? byId.get(selected):undefined;const edge=graph?.edges.find(edge=>edge.id===selectedEdge);
  const connected=useMemo(()=>new Set(graph?.edges.filter(edge=>edge.source===selected||edge.target===selected).flatMap(edge=>[edge.source,edge.target])??[]),[graph,selected]);
  const relatedPapers=papers.filter(paper=>(edge?.paperIds ?? node?.paperIds ?? []).includes(paper.id));
  const matches=(label:string)=>!query.trim()||label.toLowerCase().includes(query.trim().toLowerCase());
  async function exportImage(format:"svg"|"png") {
    if(!graph)return;const id=taskId("export");setExportId(id);setExporting(true);setError(undefined);setNotice(undefined);
    try{const path=await service.export(graph,format,id);if(path)setNotice(`图片已保存：${path}`);}catch(error){setError(normalizeAppError(error).message);}finally{setExporting(false);}
  }
  return <main className="page-shell graph-page">
    <header className="page-header"><div><span className="eyebrow">RESEARCH CONNECTIONS</span><h1><GraphIcon/>论文关系图</h1><p>从共同主题发现关联，从连线回到论文。</p></div><div className="graph-export-actions"><button className="button secondary" disabled={busy||exporting||!graph?.nodes.length} onClick={()=>void exportImage("svg")}>导出 SVG</button><button className="button primary" disabled={busy||exporting||!graph?.nodes.length} onClick={()=>void exportImage("png")}>导出 PNG 图片</button></div></header>
    <div className="graph-content">
      <section className="graph-toolbar" aria-label="关系图选项"><div className="segmented-control" aria-label="关系图模式"><button className={mode==="keywords"?"active":""} aria-pressed={mode==="keywords"} onClick={()=>setMode("keywords")}>关键词网络</button><button className={mode==="papers"?"active":""} aria-pressed={mode==="papers"} onClick={()=>setMode("papers")}>论文网络</button></div>
        <label>显示节点<select value={maxNodes} onChange={event=>setMaxNodes(Number(event.target.value))}>{[20,40,60,80].map(value=><option key={value} value={value}>{value} 个以内</option>)}</select></label>
        <label>{mode==="keywords"?"最少共同文献":"最少共同关键词"}<select value={minShared} onChange={event=>setMinShared(Number(event.target.value))}>{[1,2,3,5].map(value=><option key={value} value={value}>{value}</option>)}</select></label>
        {mode==="papers"&&<label className="graph-checkbox"><input type="checkbox" checked={includeAuthors} onChange={event=>setIncludeAuthors(event.target.checked)}/>包含共同作者</label>}
        <button className="button secondary" disabled={busy} onClick={()=>void build()}>刷新数据</button>
      </section>
      {busy&&<InlineTaskProgress id={requestId}/>} {exporting&&exportId&&<InlineTaskProgress id={exportId}/>} {error&&<p className="analysis-error" role="alert">{error}</p>}{notice&&<p className="graph-notice" role="status">{notice}</p>}
      <div className="graph-statistics"><div><strong>{graph?.totalPapers??0}</strong><span>本地论文</span></div><div><strong>{graph?.taggedPapers??0}</strong><span>已有关键词</span></div><div><strong>{graph?.nodes.length??0}</strong><span>{mode==="keywords"?"主题节点":"论文节点"}</span></div><div><strong>{graph?.edges.length??0}</strong><span>可核对的关联</span></div></div>
      <div className="graph-workspace"><section className="graph-canvas-panel">
        <div className="graph-canvas-header"><div><h2>{mode==="keywords"?"关键词共现网络":"论文主题与作者网络"}</h2><p>{mode==="keywords"?"节点大小表示出现论文数，连线粗细表示共同出现的论文数。":"连线来自共同关键词或共同署名；点击连线查看依据。"}</p></div><input aria-label="高亮查找节点" placeholder="高亮查找节点…" value={query} onChange={event=>setQuery(event.target.value)}/></div>
        {!graph?.nodes.length&&!busy?<div className="graph-empty"><GraphIcon/><h3>还没有可绘制的数据</h3><p>{mode==="keywords"?"先导入并分析论文，生成主题标签后即可建立关键词网络。":"导入论文后即可查看论文节点和共同署名关联。"}</p></div>:<svg ref={svg} className="relationship-graph" viewBox="0 0 1100 680" role="group" aria-label={mode==="keywords"?"关键词共现关系图":"论文关联关系图"}
          onPointerDown={event=>{if((event.target as Element).closest("[data-interactive]"))return;const rect=event.currentTarget.getBoundingClientRect();drag.current={x:event.clientX,y:event.clientY,px:pan.x,py:pan.y,scale:1100/rect.width};event.currentTarget.setPointerCapture(event.pointerId);}}
          onPointerMove={event=>{const current=drag.current;if(current)setPan({x:current.px+(event.clientX-current.x)*current.scale,y:current.py+(event.clientY-current.y)*current.scale});}}
          onPointerUp={()=>{drag.current=undefined;}} onPointerCancel={()=>{drag.current=undefined;}}>
          <defs><pattern id="graph-grid" width="24" height="24" patternUnits="userSpaceOnUse"><circle cx="1" cy="1" r=".8" fill="#dce3ef"/></pattern></defs><rect width="1100" height="680" fill="url(#graph-grid)"/>
          <g transform={`translate(${550*(1-zoom)+pan.x},${340*(1-zoom)+pan.y}) scale(${zoom})`}>
            {graph?.edges.map(link=>{const a=byId.get(link.source)!,b=byId.get(link.target)!;const active=selectedEdge===link.id||selected===a.id||selected===b.id;return <g key={link.id} data-interactive="true" role="button" tabIndex={0} aria-label={`查看关系：${a.label} 与 ${b.label}`} onClick={()=>{setSelectedEdge(link.id);setSelected(undefined);}} onKeyDown={event=>{if(event.key==="Enter"||event.key===" "){event.preventDefault();setSelectedEdge(link.id);setSelected(undefined);}}}><title>{mode==="keywords"?`${link.weight} 篇论文中共同出现`:`共同关键词 ${link.sharedTags.length} 个；共同署名 ${link.sharedAuthors.length} 位`}</title><line x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke={active?"#6584d9":"#bfcae1"} strokeWidth={active?2.5+Math.sqrt(link.weight):1+Math.min(4,Math.sqrt(link.weight))} opacity={selected&&!active?0.15:active?0.95:0.46}/><line x1={a.x} y1={a.y} x2={b.x} y2={b.y} stroke="transparent" strokeWidth={14}/></g>;})}
            {graph?.nodes.map((item,index)=>{const active=selected===item.id;const faded=(!matches(item.label))||(selected&&!active&&!connected.has(item.id));const r=12+Math.min(16,Math.sqrt(item.value)*3);return <g key={item.id} data-interactive="true" className="graph-node" role="button" tabIndex={0} aria-pressed={active} aria-label={`查看${item.kind==="keyword"?"关键词":"论文"}：${item.label}`} opacity={faded?0.2:1} onClick={()=>{setSelected(item.id);setSelectedEdge(undefined);}} onKeyDown={event=>{if(event.key==="Enter"||event.key===" "){event.preventDefault();setSelected(item.id);setSelectedEdge(undefined);}}}><title>{item.label}{item.kind==="keyword"?` · ${item.value} 篇论文`:""}</title>{active&&<circle cx={item.x} cy={item.y} r={r+8} fill={item.color} opacity={0.13}/>}<circle cx={item.x} cy={item.y} r={r} fill={item.color} stroke="white" strokeWidth="3"/>{item.kind==="paper"&&<text x={item.x} y={item.y+5} textAnchor="middle" fill="white" fontSize="15" fontWeight="700">P{index+1}</text>}<text x={item.x} y={item.y+r+26} textAnchor="middle" fill="#334560" fontSize={item.kind==="keyword"?21:18} fontWeight={active?700:500} paintOrder="stroke" stroke="#fbfcff" strokeWidth="4">{item.kind==="keyword"?graphLabel(item,index):item.label.length>27?`${item.label.slice(0,26)}…`:item.label}</text></g>;})}
          </g>
        </svg>}
        <div className="graph-canvas-footer"><span>拖动空白处平移 · 点击节点或连线查看详情</span><div><button aria-label="缩小关系图" onClick={()=>setZoom(value=>Math.max(.45,value-.15))}>−</button><span>{Math.round(zoom*100)}%</span><button aria-label="放大关系图" onClick={()=>setZoom(value=>Math.min(2.5,value+.15))}>＋</button><button onClick={()=>{setZoom(1);setPan({x:0,y:0});setSelected(undefined);setSelectedEdge(undefined);}}>重置</button></div></div>
      </section>
      <aside className="graph-inspector"><span className="eyebrow">CONNECTION DETAILS</span><h2>{edge?"这条关系从哪里来":node?node.kind==="keyword"?"关键词与论文":"论文与相邻节点":"探索论文之间的联系"}</h2>
        {node&&<><h3>{node.label}</h3>{node.kind==="keyword"&&<p>出现在 {node.paperIds.length} 篇论文中，与 {graph?.edges.filter(link=>link.source===node.id||link.target===node.id).length} 个显示节点相连。</p>}</>}
        {edge&&<><h3>{byId.get(edge.source)?.label}</h3><span className="graph-relation-divider">关联于</span><h3>{byId.get(edge.target)?.label}</h3>{edge.sharedTags.length>0&&<div><h4>{mode==="keywords"?"共同出现的关键词":"共同关键词"}</h4><div className="detail-tags">{edge.sharedTags.map(tag=><span key={tag}>{tag}</span>)}</div></div>}{edge.sharedAuthors.length>0&&<div><h4>共同署名（身份待核实）</h4><p>{edge.sharedAuthors.join(", ")}</p></div>}</>}
        {!node&&!edge&&<p>选择一个关键词，查看它覆盖哪些论文；切换论文网络，观察论文间的共同主题和作者联系。</p>}
        {relatedPapers.length>0&&<><h4>关联论文 · {relatedPapers.length}</h4><div className="graph-paper-list">{relatedPapers.map(paper=><button key={paper.id} onClick={()=>onPaper(paper.id)}><strong>{paper.title}</strong><span>{paper.authors.slice(0,3).join(", ")||"作者未识别"}</span><small>{paper.publication_venue||"出版物未识别"}</small><em>打开论文详情 ↗</em></button>)}</div></>}
        <div className="graph-reading-guide"><strong>连线的含义</strong><p>关键词连线表示在同一篇论文中出现；论文连线表示共享关键词或署名。它们用于发现关联，不代表引用、因果或已确认的课题组关系。</p></div>
      </aside></div>
      {graph&&(graph.omittedNodes>0||graph.omittedEdges>0)&&<p className="graph-coverage">为保持可读性，当前省略 {graph.omittedNodes} 个节点、{graph.omittedEdges} 条较弱连线。可调整节点数或共同项门槛。导出内容与当前筛选一致，并附完整节点图例。</p>}
    </div>
  </main>;
}
