import { useEffect, useState, type ReactNode } from "react";

/** Mount on first visit, then hide without destroying drafts or pending operations. */
export function KeepAliveView({ active, name, children }: { active: boolean; name: string; children: ReactNode }) {
  const [visited, setVisited] = useState(active);
  useEffect(() => { if (active) setVisited(true); }, [active]);
  if (!active && !visited) return null;
  return <div className="preserved-view" data-view={name} hidden={!active}>{children}</div>;
}
