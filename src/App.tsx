import { useState, useEffect } from "react";
import type { FormEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { clampPage, pickAppPath } from "./lib/launcher";
import { looksLikeUrl, pickShortcutFile } from "./lib/link";
import { tooltipPosition } from "./lib/tooltip";
import { contextMenuPosition } from "./lib/contextMenu";
import { AppIcon } from "./components/AppIcon";

type AppItem = { path: string; name: string; kind?: "app" | "url"; icon?: string };
type LauncherState = { pages: (AppItem | null)[][]; max_pages: number };

type TipState = { name: string; x: number; y: number };
type MenuState = { slot: number; x: number; y: number };

/** 右クリックメニューの大きさ (px)。App.css の .context-menu と合わせる。 */
const CONTEXT_MENU_SIZE = { width: 150, height: 62 };

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

type TextDialogProps = {
  title: string;
  inputLabel: string;
  placeholder?: string;
  initialValue?: string;
  onSubmit: (input: string) => Promise<void>;
  onClose: () => void;
};

/** 1 行だけ入力して Enter で確定する小さなダイアログ (URL の登録、名前の変更)。成功したら親が閉じる。 */
function TextDialog({ title, inputLabel, placeholder, initialValue = "", onSubmit, onClose }: TextDialogProps) {
  const [value, setValue] = useState(initialValue);
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
      <form role="dialog" aria-label={title} className="dialog" onClick={(e) => e.stopPropagation()} onSubmit={submit}>
        <input
          type="text"
          aria-label={inputLabel}
          placeholder={placeholder}
          autoFocus
          onFocus={(e) => e.target.select()}
          value={value}
          disabled={busy}
          onChange={(e) => setValue(e.target.value)}
        />
        {error && <p className="dialog-error">{error}</p>}
      </form>
    </div>
  );
}

/** 登録済みアイテムの右クリックメニュー。メニューの外をクリックするか Escape で閉じる。 */
function ContextMenu({ x, y, onRename, onRemove, onClose }: { x: number; y: number; onRename: () => void; onRemove: () => void; onClose: () => void }) {
  const { left, top } = contextMenuPosition(x, y, window.innerWidth, window.innerHeight, CONTEXT_MENU_SIZE);
  return (
    <div
      className="context-menu-layer"
      onClick={onClose}
      onContextMenu={(e) => { e.preventDefault(); onClose(); }}
      onKeyDown={(e) => e.key === "Escape" && onClose()}
    >
      <div role="menu" className="context-menu" style={{ left, top }} onClick={(e) => e.stopPropagation()}>
        <button type="button" role="menuitem" autoFocus onClick={onRename}>名前を変更…</button>
        <button type="button" role="menuitem" onClick={onRemove}>登録を解除</button>
      </div>
    </div>
  );
}

export default function App() {
  const [state, setState] = useState<LauncherState | null>(null);
  const [currentPage, setCurrentPage] = useState(0);
  const [error, setError] = useState<string | null>(null);
  const [tip, setTip] = useState<TipState | null>(null);
  const [dialogSlot, setDialogSlot] = useState<number | null>(null);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [renameSlot, setRenameSlot] = useState<number | null>(null);

  /** 登録系の操作を実行し、失敗したら理由を表示する。どちらの場合も最新の状態を読み直す。 */
  async function register(action: () => Promise<unknown>, failure = "登録できません") {
    try {
      await action();
      setError(null);
    } catch (e) {
      setError(`${failure}: ${e}`);
    }
    await fetchState();
  }

  useEffect(() => {
    fetchState();

    // Tauri v2 のファイルドロップは "tauri://drag-drop" で届く (旧名 "tauri://drop" では届かない)
    const unlisten = listen('tauri://drag-drop', async (event: any) => {
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

  async function submitRename(slotIdx: number, name: string) {
    await invoke("rename_app", { pageIdx: currentPage, slotIdx, name });
    await fetchState();
    setRenameSlot(null);
  }

  if (!state) return <div>Loading...</div>;

  const currentSlots = state.pages[currentPage];

  return (
    <div className="container">
      <div className="drag-region" data-tauri-drag-region />
      <div className="grid-area">
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
              onContextMenu={app ? (e) => { e.preventDefault(); setTip(null); setMenu({ slot: i, x: e.clientX, y: e.clientY }); } : undefined}
            >
              {app ? <AppIcon key={app.path} path={app.path} name={app.name} kind={app.kind} icon={app.icon} /> : <div className="slot-number">{i + 1}</div>}
            </div>
          ))}
        </div>
      </div>
      {tip && <Tooltip {...tip} />}
      {error && <p role="alert" className="error">{error}</p>}
      <div className="pager">
        <button aria-label="Previous page" onClick={() => setCurrentPage(p => clampPage(p - 1, state.max_pages))} disabled={currentPage === 0}><Chevron direction="left" /></button>
        <span>{currentPage + 1} / {state.max_pages}</span>
        <button aria-label="Next page" onClick={() => setCurrentPage(p => clampPage(p + 1, state.max_pages))} disabled={currentPage >= state.max_pages - 1}><Chevron direction="right" /></button>
      </div>
      {dialogSlot !== null && (
        <TextDialog
          title="Add URL shortcut"
          inputLabel="URL"
          placeholder="https://example.com"
          onSubmit={(input) => submitUrl(dialogSlot, input)}
          onClose={() => setDialogSlot(null)}
        />
      )}
      {menu && (
        <ContextMenu
          x={menu.x}
          y={menu.y}
          onRename={() => { setRenameSlot(menu.slot); setMenu(null); }}
          onRemove={() => { setMenu(null); register(() => invoke("remove_app", { pageIdx: currentPage, slotIdx: menu.slot }), "解除できません"); }}
          onClose={() => setMenu(null)}
        />
      )}
      {renameSlot !== null && (
        <TextDialog
          title="名前を変更"
          inputLabel="名前"
          initialValue={currentSlots[renameSlot]?.name}
          onSubmit={(name) => submitRename(renameSlot, name)}
          onClose={() => setRenameSlot(null)}
        />
      )}
    </div>
  );
}
