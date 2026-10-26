// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { expect,it,vi } from "vitest";
import { RelationshipGraphPage } from "../src/pages/RelationshipGraphPage";
import type { RelationshipGraphService } from "../src/services/RelationshipGraphService";
import type { RelationshipGraph } from "../src/domain/paper/RelationshipGraph";

(globalThis as typeof globalThis & {IS_REACT_ACT_ENVIRONMENT:boolean}).IS_REACT_ACT_ENVIRONMENT=true;
it("redraws vector coordinates with fixed glyph sizes and isolates a selected neighborhood",async()=>{
  const graph:RelationshipGraph={mode:"keywords",nodes:[
    {id:"a",label:"资源分配",kind:"keyword",value:3,paperIds:[],x:430,y:300,color:"#4468d8"},
    {id:"b",label:"通信感知",kind:"keyword",value:2,paperIds:[],x:700,y:400,color:"#159b91"},
    {id:"c",label:"独立主题",kind:"keyword",value:1,paperIds:[],x:300,y:200,color:"#a674d1"},
  ],edges:[{id:"ab",source:"a",target:"b",weight:1,paperIds:[],sharedTags:[],sharedAuthors:[]}],totalPapers:3,taggedPapers:3,omittedNodes:0,omittedEdges:0};
  const service={build:vi.fn(async()=>({graph,papers:[]}))} as unknown as RelationshipGraphService;
  const container=document.createElement("div");const root=createRoot(container);
  const button=(name:string)=>container.querySelector<HTMLButtonElement>(`button[aria-label="${name}"]`)!;
  try {
    await act(async()=>root.render(<RelationshipGraphPage service={service} onPaper={()=>{}}/>));
    const circle=()=>container.querySelector(".graph-node circle")!;
    const before={x:circle().getAttribute("cx"),r:circle().getAttribute("r"),font:container.querySelector(".graph-node text")!.getAttribute("font-size")};
    await act(async()=>button("放大关系图").click());
    expect(circle().getAttribute("cx")).not.toBe(before.x);expect(circle().getAttribute("r")).toBe(before.r);
    expect(container.querySelector(".graph-node text")!.getAttribute("font-size")).toBe(before.font);
    expect(container.querySelector('[data-testid="graph-coordinate-layer"]')!.hasAttribute("transform")).toBe(false);
    await act(async()=>container.querySelector<SVGGElement>('[aria-label="查看关键词：资源分配"]')!.dispatchEvent(new MouseEvent("click",{bubbles:true})));
    await act(async()=>container.querySelector<HTMLInputElement>(".graph-inspector input")!.click());
    expect(container.querySelectorAll(".graph-node")).toHaveLength(2);
    await act(async()=>[...container.querySelectorAll("button")].find(b=>b.textContent==="聚焦此节点")!.click());
    expect(Number(circle().getAttribute("cx"))).toBe(550);
    await act(async()=>[...container.querySelectorAll("button")].find(b=>b.textContent==="重置")!.click());
    expect(container.querySelectorAll(".graph-node")).toHaveLength(3);
  } finally {await act(async()=>root.unmount());}
});
