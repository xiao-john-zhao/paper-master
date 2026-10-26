// @vitest-environment jsdom
import { act } from "react";
import { createRoot } from "react-dom/client";
import { describe, expect, it, vi } from "vitest";
import { PaperDetail } from "../src/features/paper-detail/PaperDetail";
import { InMemoryPaperRepository } from "../src/repositories/InMemoryPaperRepository";
import { AnalysisVersionsService } from "../src/services/AnalysisVersionsService";

(globalThis as typeof globalThis & { IS_REACT_ACT_ENVIRONMENT: boolean }).IS_REACT_ACT_ENVIRONMENT = true;

describe("paper detail when switching papers", () => {
  it("edits every AI field, saves the selected version and restores the generated original", async()=>{
    const repository=new InMemoryPaperRepository();const paper=(await repository.findById("demo-section-aware"))!;
    const service=new AnalysisVersionsService(repository);const container=document.createElement("div");const root=createRoot(container);const noop=()=>{};
    try {
      await act(async()=>{root.render(<PaperDetail paper={paper} analyzing={false} versionsService={service} onChanged={noop} onSaveMetadata={async()=>{}} onAnalyze={noop} onCancel={noop} onOpen={noop} onDelete={noop} onRelink={noop} onSelectRelated={noop}/>);});
      await act(async()=>{[...container.querySelectorAll("button")].find(b=>b.textContent==="编辑 AI 分析内容")!.click();});
      expect(container.querySelectorAll(".analysis-editor textarea")).toHaveLength(9);
      const textarea=container.querySelector<HTMLTextAreaElement>('textarea[aria-label="核心方法"]')!;
      await act(async()=>{Object.getOwnPropertyDescriptor(HTMLTextAreaElement.prototype,"value")!.set!.call(textarea,"人工校正的核心方法");textarea.dispatchEvent(new Event("input",{bubbles:true}));});
      await act(async()=>{container.querySelector(".analysis-editor")!.dispatchEvent(new Event("submit",{bubbles:true,cancelable:true}));});
      expect((await repository.findById(paper.id))!.analysis!.core_method).toBe("人工校正的核心方法");
      const edited=(await service.list((await repository.findById(paper.id))!))[0];expect(edited.edited).toBe(true);
      await service.restore(edited);
      expect((await repository.findById(paper.id))!.analysis!.core_method).toBe(paper.analysis!.core_method);
    } finally {await act(async()=>root.unmount());}
  });
  it("keeps exactly one metadata editor and one analysis editor across selections and refreshes", async () => {
    const repository = new InMemoryPaperRepository();
    const first = (await repository.findById("demo-section-aware"))!;
    const second = (await repository.findById("demo-retrieval"))!;
    const third = (await repository.findById("demo-parser"))!;
    const service = new AnalysisVersionsService(repository);
    const container = document.createElement("div");
    const root = createRoot(container);
    const onSave = vi.fn(async () => {});
    const noop = () => {};
    const buttons = (text: string) => [...container.querySelectorAll("button")].filter(button => button.textContent === text);
    try {
      for (const paper of [first, second, third, first, second, first, third, second, { ...second }]) {
        await act(async () => {
          root.render(<PaperDetail paper={paper} analyzing={false} versionsService={service} onChanged={noop} onSaveMetadata={onSave} onAnalyze={noop} onCancel={noop} onOpen={noop} onDelete={noop} onRelink={noop} onSelectRelated={noop}/>);
        });
        expect(container.querySelectorAll(".metadata-editor")).toHaveLength(1);
        expect(buttons("编辑题名、作者与摘要")).toHaveLength(1);
        expect(container.querySelectorAll(".analysis-version-panel")).toHaveLength(1);
        expect(buttons("编辑 AI 分析内容")).toHaveLength(1);
      }
      await act(async () => { buttons("编辑题名、作者与摘要")[0].click(); });
      expect(container.querySelector<HTMLInputElement>(".metadata-editor input")?.value).toBe(second.title);
      await act(async () => {
        root.render(<PaperDetail paper={first} analyzing={false} versionsService={service} onChanged={noop} onSaveMetadata={onSave} onAnalyze={noop} onCancel={noop} onOpen={noop} onDelete={noop} onRelink={noop} onSelectRelated={noop}/>);
      });
      expect(container.querySelectorAll(".metadata-editor form")).toHaveLength(0);
      expect(buttons("编辑题名、作者与摘要")).toHaveLength(1);
      await act(async () => { buttons("编辑题名、作者与摘要")[0].click(); });
      expect(container.querySelector<HTMLInputElement>(".metadata-editor input")?.value).toBe(first.title);
      expect(onSave).not.toHaveBeenCalled();
    } finally {
      await act(async () => root.unmount());
    }
  });
});
