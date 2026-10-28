import { useState, useEffect } from "react";
import type { FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { clampPage, pickAppPath } from "./lib/launcher";
import { looksLikeUrl, pickShortcutFile } from "./lib/link";
import { tooltipPosition } from "./lib/tooltip";
import { AppIcon } from "./components/AppIcon";

type AppItem = { path: string; name: string; kind?: "app" | "url"; icon?: string };
type LauncherState = { pages: (AppItem | null)[][]; max_pages: number };

type TipState = { name: string; x: number; y: number };

/** マウスの近くに出るアプリ名の吹き出し。 */
function Tooltip({ name, x, y }: TipState) {
  const { left, top, alignRight } = tooltipPosition(x, y, window.innerWidth);
  return (
    <div role="tooltip" className="tooltip" style={{ left, top, transform: alignRight ? "translateX(-100%)" : undefined }}>
      {name}
    </div>
  );
}

function Chevron({ direction }: { direction: "left" | "right" }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={direction === "left" ? "M10 3 5 8l5 5" : "M6 3l5 5-5 5"} />
    </svg>
  );
}

/** 空きスロットから URL ショートカットを登録する小さなダイアログ。成功したら親が閉じる。 */
function UrlDialog({ onSubmit, onClose }: { onSubmit: (input: string) => Promise<void>; onClose: () => void }) {
  const [value, setValue] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  async function submit(e: FormEvent) {
    e.preventDefault();
    const input = value.trim();
    if (!input || busy) return;
    setBusy(true);
    setError(null);
    try {
      await onSubmit(input);
    } catch (err) {
      setError(String(err));
      setBusy(false);
    }
  }

  return (
    <div className="backdrop" onClick={onClose} onKeyDown={(e) => e.key === "Escape" && onClose()}>
      <form role="dialog" aria-label="Add URL shortcut" className="dialog" onClick={(e) => e.stopPropagation()} onSubmit={submit}>
        <input
          type="text"
          aria-label="URL"
          placeholder="https://example.com"
          autoFocus
          value={value}
          disabled={busy}
          onChange={(e) => setValue(e.target.value)}
        />
        {error && <p className="dialog-error">{error}</p>}
      </form>
    </div>
  );
}

export default function App() {
  const [state, setState] = useState<LauncherState | null>(null);
  const [currentPage, setCurrentPage] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [tip, setTip] = useState<TipState | null>(null);
  const [dialogSlot, setDialogSlot] = useState<number | null>(null);

  /** 登録系の操作を実行し、失敗したら理由を表示する。どちらの場合も最新の状態を読み直す。 */
  async function register(action: () => Promise<unknown>) {
    try {
      await action();
      setError(null);
    } catch (e) {
      setError(String(e));
    }
    await fetchState();
  }

  useEffect(() => {
    fetchState();

    const unlisten = listen('tauri://drop', async (event: any) => {
      const paths: string[] = event.payload.paths ?? [];
      const appPath = pickAppPath(paths);
      const shortcut = pickShortcutFile(paths);
      if (appPath) {
        await register(() => invoke("add_app_to_first_empty", { pageIdx: currentPage, path: appPath }));
      } else if (shortcut) {
        await register(() => invoke("add_url_file_to_first_empty", { pageIdx: currentPage, path: shortcut }));
      }
    });
    return () => { unlisten.then(f => f()); };
  }, [currentPage]);

  // URL を貼り付け (Cmd+V) ると、現在のページの最初の空きスロットに登録する。入力欄への貼り付けは対象外。
  useEffect(() => {
    const onPaste = (e: ClipboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;
      const text = e.clipboardData?.getData("text") ?? "";
      if (!looksLikeUrl(text)) return;
      register(() => invoke("add_url_to_first_empty", { pageIdx: currentPage, input: text.trim() }));
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [currentPage]);

  async function fetchState() {
    const s = await invoke<LauncherState>("get_state");
    setState(s);
  }

  async function handleLaunch(app: AppItem) {
    await invoke("launch_app", { path: app.path });
  }

  async function submitUrl(slotIdx: number, input: string) {
    await invoke("add_url_to_slot", { pageIdx: currentPage, slotIdx, input });
    setError(null);
    await fetchState();
    setDialogSlot(null);
  }

  if (!state) return <div>Loading...</div>;

  const currentSlots = state.pages[currentPage];

  return (
    <div className="container">
      <div className="drag-region" data-tauri-drag-region />
      <div className="grid">
        {currentSlots.map((app, i) => (
          <div
            key={i}
            className={`slot ${app ? 'filled' : ''}`}
            role={app ? "button" : undefined}
            aria-label={app?.name}
            onClick={() => (app ? handleLaunch(app) : setDialogSlot(i))}
            onMouseMove={app ? (e) => setTip({ name: app.name, x: e.clientX, y: e.clientY }) : undefined}
            onMouseLeave={app ? () => setTip(null) : undefined}
          >
            {app ? <AppIcon key={app.path} path={app.path} name={app.name} kind={app.kind} icon={app.icon} /> : <div className="slot-number">{i + 1}</div>}
          </div>
        ))}
      </div>
      {tip && <Tooltip {...tip} />}
      {error && <p role="alert" className="error">登録できません: {error}</p>}
      <div className="pager">
        <button aria-label="Previous page" onClick={() => setCurrentPage(p => clampPage(p - 1, state.max_pages))} disabled={currentPage === 0}><Chevron direction="left" /></button>
        <span>{currentPage + 1} / {state.max_pages}</span>
        <button aria-label="Next page" onClick={() => setCurrentPage(p => clampPage(p + 1, state.max_pages))} disabled={currentPage >= state.max_pages - 1}><Chevron direction="right" /></button>
      </div>
      {dialogSlot !== null && <UrlDialog onSubmit={(input) => submitUrl(dialogSlot, input)} onClose={() => setDialogSlot(null)} />}
    </div>
  );
}
