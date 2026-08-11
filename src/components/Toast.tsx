import { CheckIcon } from "./Icons";

export interface ToastMessage {
  id: number;
  tone: "success" | "error" | "info";
  title: string;
  message?: string;
}

export function Toast({ toast, onClose }: { toast?: ToastMessage; onClose(): void }) {
  if (!toast) return null;
  return (
    <div className={`toast ${toast.tone}`} role="status">
      <div className="toast-icon">{toast.tone === "success" ? <CheckIcon/> : toast.tone === "error" ? "!" : "i"}</div>
      <div><strong>{toast.title}</strong>{toast.message && <p>{toast.message}</p>}</div>
      <button onClick={onClose} aria-label="Close notification">×</button>
    </div>
  );
}

