import type { Paper } from "./Paper";

export type GraphMode = "keywords" | "papers";
export interface GraphNode { id:string; label:string; kind:"keyword"|"paper"; value:number; paperIds:string[]; x:number; y:number; color:string }
export interface GraphEdge { id:string; source:string; target:string; weight:number; paperIds:string[]; sharedTags:string[]; sharedAuthors:string[] }
export interface RelationshipGraph { mode:GraphMode; nodes:GraphNode[]; edges:GraphEdge[]; totalPapers:number; taggedPapers:number; omittedNodes:number; omittedEdges:number }
export interface GraphOptions { mode:GraphMode; maxNodes?:number; minShared?:number; includeAuthors?:boolean }
const palette=["#4468d8","#159b91","#a674d1","#df9850","#6296c8","#d67490"];
const key=(text:string)=>text.trim().replace(/\s+/g," ").toLocaleLowerCase();
function unique(values:string[]) { return [...new Map(values.map(value=>[key(value),value.trim()])).entries()].filter(([value])=>value.length>0); }

export function buildRelationshipGraph(papers:Paper[], options:GraphOptions):RelationshipGraph {
  const maxNodes=Math.max(1,Math.min(80,Math.floor(options.maxNodes ?? 40)));
  const minShared=Math.max(1,options.minShared ?? 1);
  const entries=[...new Map(papers.map(paper=>[paper.id,paper])).values()];
  const tags=new Map<string,{label:string;ids:Set<string>}>();
  const paperTags=new Map(entries.map(paper=>[paper.id,unique(paper.tags)]));
  for(const paper of entries) for(const [name,label] of paperTags.get(paper.id)!) {
    const tag=tags.get(name) ?? {label,ids:new Set<string>()}; tag.ids.add(paper.id); tags.set(name,tag);
  }
  let nodes:GraphNode[]=[]; const edges:GraphEdge[]=[]; let totalNodes=0;
  if(options.mode==="keywords") {
    totalNodes=tags.size;
    nodes=[...tags].sort((a,b)=>b[1].ids.size-a[1].ids.size || a[0].localeCompare(b[0])).slice(0,maxNodes).map(([name,tag],i)=>({id:`tag:${name}`,label:tag.label,kind:"keyword",value:tag.ids.size,paperIds:[...tag.ids].sort(),x:0,y:0,color:palette[i%palette.length]}));
    for(let i=0;i<nodes.length;i++) for(let j=i+1;j<nodes.length;j++) {
      const ids=nodes[i].paperIds.filter(id=>nodes[j].paperIds.includes(id));
      if(ids.length>=minShared) edges.push({id:`${nodes[i].id}|${nodes[j].id}`,source:nodes[i].id,target:nodes[j].id,weight:ids.length,paperIds:ids,sharedTags:[nodes[i].label,nodes[j].label],sharedAuthors:[]});
    }
  } else {
    totalNodes=entries.length;
    const chosen=[...entries].sort((a,b)=>b.created_at.localeCompare(a.created_at) || a.id.localeCompare(b.id)).slice(0,maxNodes);
    nodes=chosen.map((paper,i)=>({id:paper.id,label:paper.title,kind:"paper",value:paperTags.get(paper.id)!.length,paperIds:[paper.id],x:0,y:0,color:palette[i%palette.length]}));
    for(let i=0;i<chosen.length;i++) for(let j=i+1;j<chosen.length;j++) {
      const a=chosen[i],b=chosen[j];
      const otherTags=new Set(paperTags.get(b.id)!.map(([name])=>name));
      const sharedTags=paperTags.get(a.id)!.filter(([name])=>otherTags.has(name)).map(([,label])=>label);
      const otherAuthors=new Set(unique(b.authors).map(([name])=>name));
      const sharedAuthors=options.includeAuthors===false ? [] : unique(a.authors).filter(([name])=>otherAuthors.has(name)).map(([,label])=>label);
      if(sharedTags.length>=minShared || sharedAuthors.length) edges.push({id:`${a.id}|${b.id}`,source:a.id,target:b.id,weight:sharedTags.length+sharedAuthors.length,paperIds:[a.id,b.id],sharedTags,sharedAuthors});
    }
  }
  edges.sort((a,b)=>b.weight-a.weight || a.id.localeCompare(b.id));
  return {mode:options.mode,nodes,edges:edges.slice(0,240),totalPapers:entries.length,taggedPapers:entries.filter(paper=>paperTags.get(paper.id)!.length>0).length,omittedNodes:totalNodes-nodes.length,omittedEdges:Math.max(0,edges.length-240)};
}

export function layoutGraph(graph:RelationshipGraph,width=1100,height=680):RelationshipGraph {
  const nodes=graph.nodes.map((node,i)=>({...node,x:width/2+Math.cos(i*2.39996)*Math.sqrt(i+1)*32,y:height/2+Math.sin(i*2.39996)*Math.sqrt(i+1)*28}));
  const indices=new Map(nodes.map((node,i)=>[node.id,i]));
  for(let iteration=0;iteration<180;iteration++) {
    const forces=nodes.map(()=>({x:0,y:0}));
    for(let i=0;i<nodes.length;i++) for(let j=i+1;j<nodes.length;j++) {
      const dx=nodes[i].x-nodes[j].x,dy=nodes[i].y-nodes[j].y;
      const distance=Math.max(10,Math.hypot(dx,dy)); const repulsion=7200/(distance*distance);
      forces[i].x+=dx/distance*repulsion; forces[i].y+=dy/distance*repulsion;
      forces[j].x-=dx/distance*repulsion; forces[j].y-=dy/distance*repulsion;
    }
    for(const edge of graph.edges) {
      const a=indices.get(edge.source)!,b=indices.get(edge.target)!;
      const dx=nodes[b].x-nodes[a].x,dy=nodes[b].y-nodes[a].y,distance=Math.max(1,Math.hypot(dx,dy));
      const pull=(distance-155)*0.003*Math.min(3,Math.sqrt(edge.weight));
      forces[a].x+=dx/distance*pull; forces[a].y+=dy/distance*pull;
      forces[b].x-=dx/distance*pull; forces[b].y-=dy/distance*pull;
    }
    for(let i=0;i<nodes.length;i++) {
      const node=nodes[i],force=forces[i];
      node.x=Math.max(90,Math.min(width-90,node.x+(force.x+(width/2-node.x)*0.0015)*5));
      node.y=Math.max(60,Math.min(height-65,node.y+(force.y+(height/2-node.y)*0.0015)*5));
    }
  }
  return {...graph,nodes};
}
export function graphLabel(node:GraphNode,index:number) {
  if(node.kind==="paper") return `P${index+1}`;
  const label=node.label.split(/[（(]/)[0].trim();
  return label.length>22 ? `${label.slice(0,21)}…` : label;
}
const escapeXml=(text:string)=>text.replace(/[&<>"']/g,char=>({"&":"&amp;","<":"&lt;",">":"&gt;",'"':"&quot;","'":"&apos;"}[char]!));
export function graphSvg(graph:RelationshipGraph):string {
  const width=1600, chartHeight=1080;
  const lines=graph.nodes.map((node,index)=>`${node.kind==="paper" ? `P${index+1}` : `${index+1}`}  ${node.label}`);
  const wrap=(text:string)=>{const parts:string[]=[];let line="",units=0;for(const char of text){const size=/[\u0000-\u00ff]/.test(char)?0.55:1;if(units+size>43){parts.push(line);line="";units=0;}line+=char;units+=size;}if(line)parts.push(line);return parts;};
  const legends=lines.map(wrap); const rowHeights=Array.from({length:Math.ceil(legends.length/2)},(_,row)=>Math.max(legends[row*2]?.length ?? 0,legends[row*2+1]?.length ?? 0)*22+16);
  const height=chartHeight+rowHeights.reduce((sum,value)=>sum+value,0)+80;
  const index=new Map(graph.nodes.map((node,i)=>[node.id,{...node,index:i}]));
  const edges=graph.edges.map(edge=>{const a=index.get(edge.source)!,b=index.get(edge.target)!;return `<line x1="${a.x}" y1="${a.y}" x2="${b.x}" y2="${b.y}" stroke="#bdc9e3" stroke-width="${Math.min(6,1+Math.sqrt(edge.weight))}"/>`;}).join("");
  const nodes=graph.nodes.map((node,i)=>`<g><circle cx="${node.x}" cy="${node.y}" r="${12+Math.min(16,Math.sqrt(node.value)*3)}" fill="${node.color}"/><text x="${node.x}" y="${node.y+47}" text-anchor="middle" fill="#273750" font-size="16">${escapeXml(graphLabel(node,i))}</text><title>${escapeXml(node.label)}</title></g>`).join("");
  let legendY=chartHeight+15;let legend="";
  for(let row=0;row<rowHeights.length;row++){for(let column=0;column<2;column++){const i=row*2+column;if(!legends[i])continue;const x=45+column*780;legend+=`<circle cx="${x}" cy="${legendY-5}" r="5" fill="${graph.nodes[i].color}"/>`+legends[i].map((line,j)=>`<text x="${x+15}" y="${legendY+j*22}" font-size="16" fill="#435169">${escapeXml(line)}</text>`).join("");}legendY+=rowHeights[row];}
  return `<svg xmlns="http://www.w3.org/2000/svg" width="${width}" height="${height}" viewBox="0 0 ${width} ${height}" font-family="Segoe UI, Microsoft YaHei, sans-serif"><rect width="100%" height="100%" fill="#ffffff"/><text x="45" y="45" font-size="28" fill="#22324e" font-weight="700">${graph.mode==="keywords" ? "关键词共现关系图" : "论文关联关系图"}</text><text x="45" y="77" font-size="15" fill="#65738c">${graph.mode==="keywords" ? "节点大小：出现论文数；连线粗细：共同出现论文数" : "连线依据：共同关键词或共同署名；同名作者身份需核实"} · ${graph.nodes.length} 节点 / ${graph.edges.length} 条关系</text><g transform="translate(57.5,100) scale(1.35)">${edges}${nodes}</g>${legend}<text x="45" y="${height-25}" font-size="12" fill="#728097">paper-master · 本地论文库元数据关系 · 非引文网络</text></svg>`;
}
