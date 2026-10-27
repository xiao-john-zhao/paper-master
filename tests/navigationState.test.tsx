// @vitest-environment jsdom
import { act, StrictMode } from "react";
import { createRoot, type Root } from "react-dom/client";
import { afterEach, beforeEach, expect, it, vi } from "vitest";
import { App } from "../src/app/App";
import { createApplicationServices, type ApplicationServices } from "../src/services/ApplicationServices";
import { LibraryCacheService } from "../src/services/LibraryCacheService";
import { progressService } from "../src/services/ProgressService";
import { DEFAULT_SETTINGS } from "../src/settings/InMemorySettingsStorage";
import type { ResearchResult } from "../src/services/ResearchService";
import type { PaperComparisonResult, ConnectionResult } from "../src/domain/ai/types";

vi.mock("@tauri-apps/api/event", () => ({ listen: vi.fn(async () => () => {}) }));
(globalThis as typeof globalThis & {IS_REACT_ACT_ENVIRONMENT:boolean}).IS_REACT_ACT_ENVIRONMENT = true;

function deferred<T>() {
  let resolve!: (value:T)=>void;
  let reject!: (reason:Error)=>void;
  const promise=new Promise<T>((yes,no)=>{resolve=yes;reject=no;});
  return {promise,resolve,reject};
}
const researchResult: ResearchResult = {schema_version:"2", intent:{summary:"已完成并保留的检索需求",topic:"资源分配",target_authors:[],reference_title:"",relationship:"any",year_from:null,year_to:null,search_queries:["fairness"],needs_clarification:false,clarification_question:""}, papers:[],limitations:"测试结果，无外部请求"};
const comparisonResult: PaperComparisonResult={schema_version:"2",common_points:["共同研究问题"],key_differences:["论文一与论文二的假设不同"],problem_comparison:"问题比较",scenario_comparison:"场景比较",method_comparison:"方法比较",contribution_comparison:"贡献比较",results_comparison:"结果比较",limitations_comparison:"局限比较",evidence:[],summary:"后台完成的对比结果"};
let root:Root;
let container:HTMLDivElement;
let services:ApplicationServices;
const click=async(element:Element)=>act(async()=>{(element as HTMLElement).click();});
function button(text:string,scope:ParentNode=container) {
  const found=[...scope.querySelectorAll("button")].find(b=>b.textContent===text);
  if(!found)throw new Error(`Missing button: ${text}`);return found;
}
const navigate=async(text:string)=>click(button(text,container.querySelector(".sidebar")!));
async function setValue(element:HTMLInputElement|HTMLTextAreaElement|HTMLSelectElement,value:string) {
  await act(async()=>{
    const prototype=element instanceof HTMLTextAreaElement?HTMLTextAreaElement.prototype:element instanceof HTMLSelectElement?HTMLSelectElement.prototype:HTMLInputElement.prototype;
    Object.getOwnPropertyDescriptor(prototype,"value")!.set!.call(element,value);
    element.dispatchEvent(new Event(element instanceof HTMLSelectElement?"change":"input",{bubbles:true}));
  });
}
async function mount() {
  await act(async()=>root.render(<StrictMode><App services={services}/></StrictMode>));
  await act(async()=>{await vi.advanceTimersByTimeAsync(1);});
}
beforeEach(()=>{
  vi.useFakeTimers();services=createApplicationServices();
  vi.spyOn(LibraryCacheService.prototype,"status").mockResolvedValue({directory:"isolated-test",schema_version:2,pending:0,restored:0,unreadable:0});
  vi.spyOn(services.paperService,"authorContext").mockRejectedValue(new Error("No native author lookup in this test"));
  container=document.createElement("div");document.body.append(container);root=createRoot(container);
});
afterEach(async()=>{
  await act(async()=>root.unmount());container.remove();
  for(const task of progressService.snapshot()) if(task.status==="running")progressService.finish(task.id,"cancelled");
  progressService.clearFinished();vi.restoreAllMocks();vi.useRealTimers();
});

it("retains research input, unsaved settings, metadata/AI drafts and graph camera across actual sidebar navigation",async()=>{
  const storageRead=vi.spyOn(services.settingsStorage,"get");
  const storageSave=vi.spyOn(services.settingsStorage,"save");
  const graphBuild=vi.spyOn(services.graphService,"build");
  await mount();
  expect(graphBuild).not.toHaveBeenCalled(); // unopened pages stay lazy
  await click(button("编辑题名、作者与摘要"));
  const title=container.querySelector<HTMLInputElement>(".metadata-editor input")!;
  await setValue(title,"尚未保存的题名草稿");
  await click(button("编辑 AI 分析内容"));
  const analysis=container.querySelector<HTMLTextAreaElement>('textarea[aria-label="核心方法"]')!;
  await setValue(analysis,"尚未保存的 AI 方法草稿");
  await navigate("联网检索论文");
  const request=container.querySelector<HTMLTextAreaElement>("#research-request")!;
  await setValue(request,"请保留我的检索输入");
  const limit=container.querySelector<HTMLSelectElement>('[aria-label="候选篇数"]')!;
  await setValue(limit,"10");
  const reference=container.querySelector<HTMLSelectElement>("#research-reference")!;
  await setValue(reference,"demo-retrieval");
  await navigate("Settings");
  await setValue(container.querySelector<HTMLSelectElement>(".settings-page select")!,"openai");
  const key=container.querySelector<HTMLInputElement>('input[type="password"]')!;
  await setValue(key,"test-only-unsaved-key");
  const reads=storageRead.mock.calls.length;
  await navigate("Diagnostics");await navigate("联网检索论文");
  expect(container.querySelector("#research-request")).toBe(request);
  expect(request.value).toBe("请保留我的检索输入");expect(limit.value).toBe("10");expect(reference.value).toBe("demo-retrieval");
  await navigate("Settings");expect(container.querySelector('input[type="password"]')).toBe(key);expect(key.value).toBe("test-only-unsaved-key");
  expect(storageRead).toHaveBeenCalledTimes(reads);expect(storageSave).not.toHaveBeenCalled();
  expect((await services.settingsStorage.get()).api_key).toBe(""); // no accidental persistence of drafts
  await navigate("论文关系图");
  const graphCalls=graphBuild.mock.calls.length;
  await click(container.querySelector('[aria-label="放大关系图"]')!);
  const camera=container.querySelector(".graph-canvas-footer")!.textContent;
  await navigate("全部论文");
  expect(container.querySelector(".metadata-editor input")).toBe(title);expect(title.value).toBe("尚未保存的题名草稿");
  expect(container.querySelector('textarea[aria-label="核心方法"]')).toBe(analysis);expect(analysis.value).toBe("尚未保存的 AI 方法草稿");
  await navigate("论文关系图");expect(container.querySelector(".graph-canvas-footer")!.textContent).toBe(camera);expect(graphBuild).toHaveBeenCalledTimes(graphCalls);
  expect(container.querySelectorAll('.preserved-view:not([hidden])')).toHaveLength(1);
});

it("continues the same online search while hidden or unfocused and shows its result on return",async()=>{
  services.tauri=true;
  const pending=deferred<ResearchResult>();
  const cancel=vi.spyOn(services.researchService,"cancel").mockResolvedValue(true);
  const search=vi.spyOn(services.researchService,"search").mockImplementation(async(_request,_limit,id)=>{
    progressService.begin(id,"research");const result=await pending.promise;progressService.finish(id);return result;
  });
  await mount();await navigate("联网检索论文");
  const input=container.querySelector<HTMLTextAreaElement>("#research-request")!;await setValue(input,"持续进行的检索");
  await act(async()=>{container.querySelector(".research-search form")!.dispatchEvent(new Event("submit",{bubbles:true,cancelable:true}));});
  const id=search.mock.calls[0][2];expect(input.disabled).toBe(true);
  await navigate("Settings");await navigate("Diagnostics");
  await act(async()=>{window.dispatchEvent(new Event("blur"));document.dispatchEvent(new Event("visibilitychange"));window.dispatchEvent(new Event("focus"));});
  expect(cancel).not.toHaveBeenCalled();expect(progressService.snapshot().find(t=>t.id===id)?.status).toBe("running");
  await act(async()=>pending.resolve(researchResult));
  await navigate("联网检索论文");
  expect(container.querySelector("#research-request")).toBe(input);expect(input.value).toBe("持续进行的检索");expect(input.disabled).toBe(false);
  expect(container.querySelector(".research-intent")!.textContent).toContain("已完成并保留的检索需求");
  expect(search).toHaveBeenCalledTimes(1);expect(cancel).not.toHaveBeenCalled();
  expect(progressService.snapshot().find(t=>t.id===id)?.status).toBe("completed");
});

it("keeps explicit research cancellation working after navigating away and back",async()=>{
  services.tauri=true;const pending=deferred<ResearchResult>();
  const search=vi.spyOn(services.researchService,"search").mockImplementation(()=>pending.promise);
  const cancel=vi.spyOn(services.researchService,"cancel").mockImplementation(async()=>{pending.reject(new Error("已明确取消"));return true;});
  await mount();await navigate("联网检索论文");await setValue(container.querySelector<HTMLTextAreaElement>("#research-request")!,"需要明确取消的任务");
  await act(async()=>{container.querySelector(".research-search form")!.dispatchEvent(new Event("submit",{bubbles:true,cancelable:true}));});
  await navigate("全部论文");await navigate("联网检索论文");expect(cancel).not.toHaveBeenCalled();
  await click(button("取消",container.querySelector(".research-page")!));
  expect(cancel).toHaveBeenCalledTimes(1);expect(cancel).toHaveBeenCalledWith(search.mock.calls[0][2]);
  expect(container.querySelector<HTMLTextAreaElement>("#research-request")!.disabled).toBe(false);
});

it("retains a connection task and its completed result when settings is hidden",async()=>{
  await services.settingsStorage.save({...DEFAULT_SETTINGS,ai_provider:"openai",api_base_url:"https://api.example.invalid/v1",api_model:"test-model",privacy_mode:"codex_online"});
  const pending=deferred<ConnectionResult>();const connection=vi.spyOn(services.providers.openai,"checkConnection").mockImplementation(()=>pending.promise);
  await mount();await navigate("Settings");
  await click(button("测试 API 连接 / 读取模型"));
  expect(container.querySelector<HTMLFieldSetElement>("fieldset.form-grid")!.disabled).toBe(true);
  await navigate("全部论文");
  await act(async()=>pending.resolve({state:"connected",message:"后台连接完成",models:["test-model"],loaded_models:["test-model"]}));
  await navigate("Settings");
  expect(container.querySelector(".connection-result")!.textContent).toContain("后台连接完成");
  expect(container.querySelector<HTMLFieldSetElement>("fieldset.form-grid")!.disabled).toBe(false);
  expect(connection).toHaveBeenCalledTimes(1);
});

it("hides and reopens a pending comparison without cancelling it, then retains its edit draft",async()=>{
  const pending=deferred<{result:PaperComparisonResult;requestId:string}>();
  const compare=vi.spyOn(services.comparisonService,"compare").mockImplementation((_papers,id)=>pending.promise);
  const cancel=vi.spyOn(services.comparisonService,"cancel").mockResolvedValue(undefined);
  await mount();
  const checkboxes=container.querySelectorAll<HTMLInputElement>(".compare-check input");await click(checkboxes[0]);await click(checkboxes[1]);
  await click(button("Compare (2)"));
  const id=compare.mock.calls[0][1]!;
  await click(container.querySelector('[aria-label="暂时收起对比"]')!);
  expect(container.querySelector<HTMLElement>('[data-view="comparison"]')!.hidden).toBe(true);expect(cancel).not.toHaveBeenCalled();
  await navigate("Settings");await click(button("查看进行中的对比",container.querySelector(".sidebar")!));
  expect(container.querySelector<HTMLElement>('[data-view="comparison"]')!.hidden).toBe(false);
  await click(container.querySelector('[aria-label="暂时收起对比"]')!);
  await act(async()=>pending.resolve({result:comparisonResult,requestId:id}));
  await click(button("查看上次对比",container.querySelector(".sidebar")!));
  expect(container.querySelector(".comparison-results")!.textContent).toContain("后台完成的对比结果");
  await click(button("编辑对比文字"));
  const draft=container.querySelector<HTMLTextAreaElement>(".comparison-dialog textarea")!;await setValue(draft,"尚未保存的对比修订");
  await click(container.querySelector('[aria-label="暂时收起对比"]')!);await navigate("全部论文");await click(button("查看上次对比",container.querySelector(".sidebar")!));
  expect(container.querySelector(".comparison-dialog textarea")).toBe(draft);expect(draft.value).toBe("尚未保存的对比修订");
  expect(compare).toHaveBeenCalledTimes(1);expect(cancel).not.toHaveBeenCalled();
});

it("cancels a comparison only on explicit request and retains a readable cancelled state",async()=>{
  const pending=deferred<{result:PaperComparisonResult;requestId:string}>();
  const compare=vi.spyOn(services.comparisonService,"compare").mockImplementation(()=>pending.promise);
  const cancel=vi.spyOn(services.comparisonService,"cancel").mockImplementation(async()=>{pending.reject(new Error("cancelled"));});
  await mount();const checkboxes=container.querySelectorAll<HTMLInputElement>(".compare-check input");await click(checkboxes[0]);await click(checkboxes[1]);
  await click(button("Compare (2)"));await click(container.querySelector('[aria-label="暂时收起对比"]')!);
  await navigate("Settings");await click(button("查看进行中的对比",container.querySelector(".sidebar")!));
  await click(button("取消",container.querySelector(".comparison-dialog")!));
  expect(cancel).toHaveBeenCalledTimes(1);expect(cancel).toHaveBeenCalledWith(compare.mock.calls[0][1]);
  await click(button("查看上次对比",container.querySelector(".sidebar")!));
  expect(container.querySelector(".comparison-error")!.textContent).toContain("论文对比已取消");
  expect(compare).toHaveBeenCalledTimes(1);
});
