import { useState, useEffect, useRef } from "react";
import type { FormEvent, MouseEvent as ReactMouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { clampPage, pickAppPath } from "./lib/launcher";
import { looksLikeUrl, pickShortcutFile } from "./lib/link";
import { tooltipPosition } from "./lib/tooltip";
import { contextMenuPosition } from "./lib/contextMenu";
import { isDragGesture, isOutsideWindow, resolveItemDrop, slotIndexOf, toClientPoint } from "./lib/drag";
import type { Point } from "./lib/drag";
import { AppIcon } from "./components/AppIcon";

type AppItem = { path: string; name: string; kind?: "app" | "url"; icon?: string };
type LauncherState = { pages: (AppItem | null)[][]; max_pages: number };

type TipState = { name: string; x: number; y: number };
type MenuState = { slot: number; x: number; y: number };
/** ドラッグ中のアイテムのスロット、ポインタの位置、その下にあるスロット。 */
type DragState = { slot: number; x: number; y: number; over: number | null };

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
  const [drag, setDrag] = useState<DragState | null>(null);
  // ドラッグを終えた直後に届くクリックで起動しないようにする
  const suppressClick = useRef(false);

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
      const position: Point | undefined = event.payload.position;
      // スロットの上に落とされたらそこへ差し込み (埋まっていれば後ろへずらす)、それ以外は最初の空きへ
      const slotIdx = position ? slotAt(toClientPoint(position, window.devicePixelRatio)) : null;
      const appPath = pickAppPath(paths);
      const shortcut = pickShortcutFile(paths);
      if (appPath) {
        await register(() => slotIdx === null
          ? invoke("add_app_to_first_empty", { pageIdx: currentPage, path: appPath })
          : invoke("insert_app", { pageIdx: currentPage, slotIdx, path: appPath }));
      } else if (shortcut) {
        await register(() => slotIdx === null
          ? invoke("add_url_file_to_first_empty", { pageIdx: currentPage, path: shortcut })
          : invoke("insert_url_file", { pageIdx: currentPage, slotIdx, path: shortcut }));
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

  /** 画面上の座標にあるスロット番号。スロットの上でなければ null。 */
  function slotAt({ x, y }: Point): number | null {
    return slotIndexOf(document.elementFromPoint(x, y));
  }

  /**
   * 登録済みアイテムのドラッグを始める。しきい値を超えて動くまではクリックとして扱う。
   * 離した位置がウィンドウ外なら解除、別のスロットなら移動 (埋まっていれば入れ替え)。
   * ウィンドウ外でもボタンを押している間はマウスイベントが届くので、window で受ける。
   */
  function startDrag(e: ReactMouseEvent, slot: number) {
    if (e.button !== 0) return;
    suppressClick.current = false;
    const start = { x: e.clientX, y: e.clientY };
    const pageIdx = currentPage;
    let dragging = false;

    const onMove = (ev: MouseEvent) => {
      const p = { x: ev.clientX, y: ev.clientY };
      if (!dragging && !isDragGesture(start, p)) return;
      dragging = true;
      setTip(null);
      setDrag({ slot, ...p, over: slotAt(p) });
    };
    const onUp = (ev: MouseEvent) => {
      window.removeEventListener("mousemove", onMove);
      window.removeEventListener("mouseup", onUp);
      if (!dragging) return;
      suppressClick.current = true;
      setDrag(null);
      const p = { x: ev.clientX, y: ev.clientY };
      const outside = isOutsideWindow(p, window.innerWidth, window.innerHeight);
      const result = resolveItemDrop(slot, { outside, slot: outside ? null : slotAt(p) });
      if (result?.type === "remove") {
        register(() => invoke("remove_app", { pageIdx, slotIdx: slot }), "解除できません");
      } else if (result?.type === "move") {
        register(() => invoke("move_app", { pageIdx, fromSlot: slot, toSlot: result.to }), "移動できません");
      }
    };
    window.addEventListener("mousemove", onMove);
    window.addEventListener("mouseup", onUp);
  }

  function handleSlotClick(app: AppItem | null, slot: number) {
    if (suppressClick.current) {
      suppressClick.current = false;
      return;
    }
    if (app) handleLaunch(app);
    else setDialogSlot(slot);
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
  const dragged = drag ? currentSlots[drag.slot] : null;

  return (
    <div className="container">
      <div className="drag-region" data-tauri-drag-region />
      <div className="grid-area">
        <div className="grid">
          {currentSlots.map((app, i) => (
            <div
              key={i}
              data-slot={i}
              className={`slot ${app ? 'filled' : ''} ${drag?.slot === i ? 'dragging' : ''} ${drag && drag.over === i && drag.slot !== i ? 'drop-target' : ''}`}
              role={app ? "button" : undefined}
              aria-label={app?.name}
              onClick={() => handleSlotClick(app, i)}
              onMouseDown={app ? (e) => startDrag(e, i) : undefined}
              onMouseMove={app && !drag ? (e) => setTip({ name: app.name, x: e.clientX, y: e.clientY }) : undefined}
              onMouseLeave={app ? () => setTip(null) : undefined}
              onContextMenu={app ? (e) => { e.preventDefault(); setTip(null); setMenu({ slot: i, x: e.clientX, y: e.clientY }); } : undefined}
            >
              {app ? <AppIcon key={app.path} path={app.path} name={app.name} kind={app.kind} icon={app.icon} /> : <div className="slot-number">{i + 1}</div>}
            </div>
          ))}
        </div>
      </div>
      {tip && <Tooltip {...tip} />}
      {drag && dragged && (
        <div className="drag-ghost" style={{ left: drag.x, top: drag.y }} aria-hidden="true">
          <AppIcon path={dragged.path} name={dragged.name} kind={dragged.kind} icon={dragged.icon} />
        </div>
      )}
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
