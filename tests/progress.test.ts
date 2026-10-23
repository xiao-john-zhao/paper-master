import { describe,expect,it } from "vitest";
import { ProgressService } from "../src/services/ProgressService";
describe("task progress lifecycle",()=>{
  it("ignores late running events after cancellation and permits explicit retries",()=>{
    const service=new ProgressService();service.begin("1","analysis");service.finish("1","cancelled","用户取消");service.update({id:"1",kind:"analysis",stage:"late response",completed:3,total:4,unit:"steps"});
    expect(service.snapshot()[0].status).toBe("cancelled");service.begin("1","analysis");expect(service.snapshot()[0].status).toBe("running");
  });
  it("keeps unrelated jobs separate and clears only finished jobs",()=>{
    const service=new ProgressService();let changes=0;const unsubscribe=service.subscribe(()=>changes++);
    service.begin("a","import");service.begin("b","research");service.update({id:"a",kind:"import",stage:"2 files",completed:2,total:5,unit:"files"});service.finish("b","failed","网络失败");service.clearFinished();
    expect(service.snapshot()).toHaveLength(1);expect(service.snapshot()[0].completed).toBe(2);expect(service.snapshot()[0].total).toBe(5);expect(changes).toBe(5);unsubscribe();
  });
  it("does not invent percentages when totals are unavailable and clamps invalid counts",()=>{
    const service=new ProgressService();service.update({id:"d",kind:"download",stage:"bytes",completed:1000,total:0,unit:"bytes"});expect(service.snapshot()[0].total).toBeUndefined();expect(service.snapshot()[0].completed).toBeUndefined();
    service.update({id:"d",kind:"download",stage:"bytes",completed:1000,total:500,unit:"bytes"});expect(service.snapshot()[0].completed).toBe(500);
    service.finish("d","failed","下载中断");expect(service.snapshot()[0].status).toBe("failed");
  });
});
