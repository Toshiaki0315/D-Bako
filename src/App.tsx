import { useState, useEffect, useRef } from "react";
import type { CSSProperties, FormEvent, MouseEvent as ReactMouseEvent } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { pickAppPath, wrapPage } from "./lib/launcher";
import { looksLikeUrl, pickShortcutFile } from "./lib/link";
import { tooltipPosition } from "./lib/tooltip";
import { contextMenuPosition } from "./lib/contextMenu";
import { LOAD_RETRY_DELAYS, retry } from "./lib/retry";
import { isDragGesture, isOutsideWindow, resolveItemDrop, slotIndexOf, toClientPoint } from "./lib/drag";
import type { Point } from "./lib/drag";
import { layoutWidth } from "./lib/layout";
import { buildPages } from "./lib/pages";
import type { WidgetKind, Widgets } from "./lib/pages";
import { AppIcon } from "./components/AppIcon";
import { ClockPage, PomodoroPage, TimerPage } from "./components/WidgetPages";

type AppItem = { path: string; name: string; kind?: "app" | "url"; icon?: string };
type LauncherState = { pages: (AppItem | null)[][]; max_pages: number; cols: number; rows: number };

type TipState = { name: string; x: number; y: number };
type MenuState = { slot: number; x: number; y: number };
/** ドラッグ中のアイテムのスロット、ポインタの位置、その下にあるスロット。 */
type DragState = { slot: number; x: number; y: number; over: number | null };

/** ランチャーの後ろに並ぶ時計などのページ。 */
const WIDGET_PAGES: Record<WidgetKind, () => JSX.Element> = { clock: ClockPage, pomodoro: PomodoroPage, timer: TimerPage };

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
  // 起動時の読み込みが失敗しきったときの理由。再読み込みのたびに loadAttempt を進めてやり直す。
  const [loadError, setLoadError] = useState<string | null>(null);
  const [loadAttempt, setLoadAttempt] = useState(0);
  const [tip, setTip] = useState<TipState | null>(null);
  const [dialogSlot, setDialogSlot] = useState<number | null>(null);
  const [menu, setMenu] = useState<MenuState | null>(null);
  const [renameSlot, setRenameSlot] = useState<number | null>(null);
  const [drag, setDrag] = useState<DragState | null>(null);
  // 時計などのページのうち表示するもの (設定ウィンドウで変わると Rust 側から知らされる)
  const [widgets, setWidgets] = useState<Widgets | undefined>(undefined);
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

  // 起動直後は裏側の準備が間に合わないことがあるので、最初の読み込みは失敗しても何度かやり直す。
  // それでも読めなければ、「Loading...」のまま止まらないよう理由と再読み込みの手段を出す。
  useEffect(() => {
    let cancelled = false;
    retry(fetchState, LOAD_RETRY_DELAYS).catch((e) => {
      if (!cancelled) setLoadError(String(e));
    });
    return () => { cancelled = true; };
  }, [loadAttempt]);

  function reload() {
    setLoadError(null);
    setLoadAttempt((n) => n + 1);
  }

  // ランチャーのページの後ろに時計などのページを並べる。ランチャー用の操作は、ランチャーのページでだけ受け付ける。
  const pages = state ? buildPages(state.max_pages, widgets) : [];
  const page = pages[Math.min(currentPage, pages.length - 1)];
  const launcherPage = page?.kind === "launcher" ? page.index : null;

  // UI の拡縮の基準 (App.css の html の font-size) を、今のグリッドの基準の幅に合わせる
  const baseWidth = state ? layoutWidth(state.cols, state.rows) : null;
  useEffect(() => {
    if (baseWidth !== null) document.documentElement.style.setProperty("--layout-width", String(baseWidth));
  }, [baseWidth]);

  // ページが減って今のページが無くなったら、最後のページへ移る
  useEffect(() => {
    if (pages.length > 0) setCurrentPage((p) => Math.min(p, pages.length - 1));
  }, [pages.length]);

  // 最初の読み込みは上の effect が受け持つ。ページを移ったときは読み直す。
  useEffect(() => {
    if (state) fetchState().catch(() => {});
  }, [currentPage]);

  useEffect(() => {
    const fetchWidgets = () => invoke<Widgets>("get_widgets").then(setWidgets).catch(() => {});
    fetchWidgets();
    const unlisten = listen("settings-changed", fetchWidgets);
    return () => { unlisten.then(f => f()); };
  }, []);

  useEffect(() => {
    if (launcherPage === null) return;
    const pageIdx = launcherPage;
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
          ? invoke("add_app_to_first_empty", { pageIdx, path: appPath })
          : invoke("insert_app", { pageIdx, slotIdx, path: appPath }));
      } else if (shortcut) {
        await register(() => slotIdx === null
          ? invoke("add_url_file_to_first_empty", { pageIdx, path: shortcut })
          : invoke("insert_url_file", { pageIdx, slotIdx, path: shortcut }));
      }
    });
    return () => { unlisten.then(f => f()); };
  }, [launcherPage]);

  // URL を貼り付け (Cmd+V) ると、現在のページの最初の空きスロットに登録する。入力欄への貼り付けは対象外。
  useEffect(() => {
    if (launcherPage === null) return;
    const pageIdx = launcherPage;
    const onPaste = (e: ClipboardEvent) => {
      if (e.target instanceof HTMLInputElement) return;
      const text = e.clipboardData?.getData("text") ?? "";
      if (!looksLikeUrl(text)) return;
      register(() => invoke("add_url_to_first_empty", { pageIdx, input: text.trim() }));
    };
    window.addEventListener("paste", onPaste);
    return () => window.removeEventListener("paste", onPaste);
  }, [launcherPage]);

  // 設定ウィンドウでグリッドの大きさやページ数が変わると、Rust 側から知らされる
  useEffect(() => {
    const unlisten = listen("launcher-changed", () => fetchState());
    return () => { unlisten.then(f => f()); };
  }, []);

  /** 最新の状態を読み直す。 */
  async function fetchState() {
    setState(await invoke<LauncherState>("get_state"));
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
    const pageIdx = launcherPage ?? 0;
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
    await invoke("add_url_to_slot", { pageIdx: launcherPage ?? 0, slotIdx, input });
    setError(null);
    await fetchState();
    setDialogSlot(null);
  }

  async function submitRename(slotIdx: number, name: string) {
    await invoke("rename_app", { pageIdx: launcherPage ?? 0, slotIdx, name });
    await fetchState();
    setRenameSlot(null);
  }

  if (!state && loadError) {
    return (
      <div className="container">
        <div className="drag-region" data-tauri-drag-region />
        <div className="load-error" role="alert">
          <p>起動に失敗しました: {loadError}</p>
          <button type="button" className="reload" onClick={reload}>再読み込み</button>
        </div>
      </div>
    );
  }
  if (!state) return <div>Loading...</div>;

  const currentSlots = launcherPage === null ? [] : state.pages[launcherPage];
  const dragged = drag ? currentSlots[drag.slot] : null;

  return (
    <div className="container">
      <div className="drag-region" data-tauri-drag-region />
      <div className="grid-area">
        {launcherPage !== null && <div className="grid" style={{ "--cols": state.cols, "--rows": state.rows } as CSSProperties}>
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
        </div>}
        {/* 時計などのページは、ほかのページを見ている間も隠すだけにして、タイマーを動かし続ける */}
        {pages.flatMap((p) => p.kind === "launcher" ? [] : [p.kind]).map((kind) => {
          const Widget = WIDGET_PAGES[kind];
          return <div key={kind} className="widget-holder" hidden={page?.kind !== kind}><Widget /></div>;
        })}
      </div>
      {tip && <Tooltip {...tip} />}
      {drag && dragged && (
        <div className="drag-ghost" style={{ left: drag.x, top: drag.y }} aria-hidden="true">
          <AppIcon path={dragged.path} name={dragged.name} kind={dragged.kind} icon={dragged.icon} />
        </div>
      )}
      {error && <p role="alert" className="error">{error}</p>}
      <div className="pager">
        <button aria-label="Previous page" onClick={() => setCurrentPage(p => wrapPage(p - 1, pages.length))} disabled={pages.length <= 1}><Chevron direction="left" /></button>
        <span>{Math.min(currentPage, pages.length - 1) + 1} / {pages.length}</span>
        <button aria-label="Next page" onClick={() => setCurrentPage(p => wrapPage(p + 1, pages.length))} disabled={pages.length <= 1}><Chevron direction="right" /></button>
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
          onRemove={() => { setMenu(null); register(() => invoke("remove_app", { pageIdx: launcherPage ?? 0, slotIdx: menu.slot }), "解除できません"); }}
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
