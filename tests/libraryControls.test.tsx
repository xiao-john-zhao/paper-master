// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe,it,expect } from "vitest";
import { Sidebar } from "../src/components/Sidebar";
import { InMemoryPaperRepository } from "../src/repositories/InMemoryPaperRepository";
import { analysisVersionLabel } from "../src/domain/paper/AnalysisSource";
import type { AnalysisVersion } from "../src/services/AnalysisVersionsService";
import { RelationshipGraphPage } from "../src/pages/RelationshipGraphPage";
import type { RelationshipGraphService } from "../src/services/RelationshipGraphService";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT:boolean }).IS_REACT_ACT_ENVIRONMENT=true;

describe("library controls",()=>{
  it("keeps all keyword checkboxes visible and supports intersecting multiple tags",async()=>{
    const repository=new InMemoryPaperRepository();
    expect((await repository.list({tags:["scientific NLP","local LLM"]})).map(p=>p.id)).toEqual(["demo-section-aware"]);
    expect(await repository.list({tags:["local LLM","privacy"]})).toHaveLength(0);
    const container=document.createElement("div");const root=createRoot(container);const tags=[{name:"ISAC",count:20},{name:"UAV",count:10},{name:"Other",count:5}];let selected:string[]=[];
    const render=()=>root.render(<Sidebar view="library" filter="all" tags={tags} activeTags={selected} onView={()=>{}} onFilter={()=>{}} onTag={tag=>{selected=selected.includes(tag)?selected.filter(t=>t!==tag):[...selected,tag];render();}} onClearTags={()=>{selected=[];render();}}/>);
    try { await act(async()=>render());await act(async()=>container.querySelectorAll<HTMLInputElement>('.tag-nav-item input')[0].click());await act(async()=>container.querySelectorAll<HTMLInputElement>('.tag-nav-item input')[1].click());expect(selected).toEqual(["ISAC","UAV"]);expect(container.querySelectorAll('.tag-nav-item')).toHaveLength(3);expect(container.querySelectorAll('input:checked')).toHaveLength(2); } finally {await act(async()=>root.unmount());}
  });
  it("labels actual models and summary times while marking unknown historical identities honestly",()=>{
    const v={provider:"lmstudio",model:"qwen-local-27b",created_at:"2026-10-25T10:00:00Z",updated_at:"2026-10-25T11:00:00Z",edited:true,revision:2} as AnalysisVersion;
    expect(analysisVersionLabel(v)).toContain("LM Studio · qwen-local-27b");expect(analysisVersionLabel(v)).toContain("总结时间：2026");expect(analysisVersionLabel(v)).toContain("修订 2");
    expect(analysisVersionLabel({...v,provider:"codex",model:"Codex CLI default"})).toContain("具体模型未记录");
    expect(analysisVersionLabel({...v,provider:"extractive"})).toContain("原文摘录（非模型）");
    expect(analysisVersionLabel({...v,provider:"legacy"})).toContain("总结时间未记录");
  });
  it("changes node positions without scaling text on zoom",async()=>{
    const graph={mode:"keywords",nodes:[{id:"tag:a",label:"ISAC",kind:"keyword",value:3,paperIds:[],x:650,y:400,color:"#4468d8"}],edges:[],totalPapers:1,taggedPapers:1,omittedNodes:0,omittedEdges:0};
    const service={build:async()=>({graph,papers:[]})} as unknown as RelationshipGraphService;
    const container=document.createElement("div");const root=createRoot(container);
    try {await act(async()=>root.render(<RelationshipGraphPage service={service} onPaper={()=>{}}/>));const text=container.querySelector('.graph-node text')!;const font=text.getAttribute('font-size');const x=Number(text.getAttribute('x'));const radius=container.querySelector('.graph-node circle')!.getAttribute('r');await act(async()=>container.querySelector<HTMLButtonElement>('[aria-label="放大关系图"]')!.click());expect(Number(container.querySelector('.graph-node text')!.getAttribute('x'))).toBeGreaterThan(x);expect(container.querySelector('.graph-node text')!.getAttribute('font-size')).toBe(font);expect(container.querySelector('.graph-node circle')!.getAttribute('r')).toBe(radius);expect(container.querySelector('[data-testid="graph-coordinate-layer"]')!.getAttribute('transform')).toBeNull();}finally {await act(async()=>root.unmount());}
  });
});
