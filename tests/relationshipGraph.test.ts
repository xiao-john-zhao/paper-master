import { describe,expect,it } from "vitest";
import { buildRelationshipGraph,layoutGraph,graphSvg,graphPoint,zoomGraphAt,graphLabelLines } from "../src/domain/paper/RelationshipGraph";
import type { Paper } from "../src/domain/paper/Paper";

const paper=(id:string,tags:string[],authors:string[]=[]):Paper=>({id,title:`Paper ${id}`,authors,tags,file_name:`${id}.pdf`,file_path:`/${id}.pdf`,created_at:`2026-10-0${id}`,updated_at:"now",analysis_status:"analyzed"});
describe("source-backed relationship graph",()=>{
  it("keeps a panned world point anchored under the cursor and reveals complete labels",()=>{
    const pan={x:140,y:-85},zoom=1.7,point={x:700,y:280};
    const anchor=graphPoint(point,zoom,pan);
    const next=zoomGraphAt(zoom,pan,4.2,anchor);
    const projected=graphPoint(point,next.zoom,next.pan);
    expect(projected.x).toBeCloseTo(anchor.x);expect(projected.y).toBeCloseTo(anchor.y);
    expect(zoomGraphAt(1,pan,100).zoom).toBe(12);
    const label="多无人机通信与感知资源分配关联关系中的完整研究方法与实验标题";
    const lines=graphLabelLines(label,12);expect(lines.length).toBeGreaterThan(1);expect(lines.join("")).toBe(label);
  });
  it("zooms graph coordinates independently of glyph sizes",()=>{
    const a=graphPoint({x:600,y:400},1,{x:0,y:0});
    const b=graphPoint({x:600,y:400},2,{x:0,y:0});
    expect(b.x-550).toBe((a.x-550)*2);expect(b.y-340).toBe((a.y-340)*2);
    expect(graphPoint({x:550,y:340},.2,{x:10,y:15})).toEqual({x:560,y:355});
  });
  it("counts a keyword and co-occurrence at most once per paper, including duplicate records",()=>{
    const a=paper("1",["ISAC"," isac ","UAV"]),b=paper("2",["ISAC","UAV"]),c=paper("3",["ISAC","Fairness"]);
    const graph=buildRelationshipGraph([a,a,b,c],{mode:"keywords"});
    expect(graph.totalPapers).toBe(3);expect(graph.nodes.find(node=>node.id==="tag:isac")?.value).toBe(3);
    const link=graph.edges.find(edge=>edge.source==="tag:isac"&&edge.target==="tag:uav"||edge.target==="tag:isac"&&edge.source==="tag:uav");
    expect(link?.weight).toBe(2);expect(link?.paperIds).toEqual(["1","2"]);
    expect(graph.edges.some(edge=>edge.source===edge.target)).toBe(false);
    const strict=buildRelationshipGraph([a,b,c],{mode:"keywords",minShared:2});expect(strict.edges).toHaveLength(1);
  });
  it("connects papers by shared keywords or exact normalized author names and supports disabling author links",()=>{
    const papers=[paper("1",["ISAC","UAV"],["Ada Lovelace"]),paper("2",["ISAC"],["Alan Turing"]),paper("3",[],["ada lovelace"])];
    const graph=buildRelationshipGraph(papers,{mode:"papers"});expect(graph.edges).toHaveLength(2);
    expect(graph.edges.find(edge=>edge.sharedAuthors.length)?.sharedAuthors[0].toLowerCase()).toBe("ada lovelace");
    expect(buildRelationshipGraph(papers,{mode:"papers",includeAuthors:false}).edges).toHaveLength(1);
    expect(buildRelationshipGraph(papers,{mode:"papers",minShared:2,includeAuthors:false}).edges).toHaveLength(0);
  });
  it("bounds a dense graph and produces deterministic finite coordinates",()=>{
    const papers=[paper("1",Array.from({length:70},(_,i)=>`Topic ${i}`))];
    const graph=buildRelationshipGraph(papers,{mode:"keywords",maxNodes:40});expect(graph.nodes).toHaveLength(40);expect(graph.edges).toHaveLength(240);expect(graph.omittedNodes).toBe(30);expect(graph.omittedEdges).toBeGreaterThan(0);
    const layout=layoutGraph(graph);expect(layout).toEqual(layoutGraph(graph));
    expect(layout.nodes.every(node=>Number.isFinite(node.x)&&Number.isFinite(node.y)&&node.x>=0&&node.x<=1100&&node.y>=0&&node.y<=680)).toBe(true);
  });
  it("exports complete labels safely and handles empty libraries",()=>{
    const graph=layoutGraph(buildRelationshipGraph([{...paper("1",["A <script> & B"]),title:"A <script> & B"}],{mode:"papers"}));
    const svg=graphSvg(graph);expect(svg).toContain("A &lt;script&gt; &amp; B");expect(svg).not.toContain("<script>");expect(svg).toContain("非引文网络");expect(svg).toContain("P1");
    expect(buildRelationshipGraph([],{mode:"keywords"}).nodes).toHaveLength(0);
  });
  it("renders potential-team evidence independently of keyword and author toggles",()=>{
    const papers=[paper("1",[]),paper("2",[]),paper("3",["Other"])];
    const teamLinks=[{source:"1",target:"2",evidence:["共同通信作者：Ada；待核实"]}];
    const graph=buildRelationshipGraph(papers,{mode:"teams",teamLinks,includeAuthors:false});
    expect(graph.nodes).toHaveLength(2);expect(graph.edges).toHaveLength(1);
    expect(graph.edges[0].teamEvidence).toEqual(teamLinks[0].evidence);
    expect(graphSvg(layoutGraph(graph))).toContain('stroke-dasharray="7 5"');
    expect(buildRelationshipGraph(papers,{mode:"papers",teamLinks,includeTeams:false,includeAuthors:false}).edges).toHaveLength(0);
  });
});
