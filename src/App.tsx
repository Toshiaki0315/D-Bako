import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { clampPage, pickAppPath } from "./lib/launcher";
import { AppIcon } from "./components/AppIcon";

type AppItem = { path: string; name: string };
type LauncherState = { pages: (AppItem | null)[][]; max_pages: number };

function Chevron({ direction }: { direction: "left" | "right" }) {
  return (
    <svg width="16" height="16" viewBox="0 0 16 16" fill="none" stroke="currentColor" strokeWidth="2" strokeLinecap="round" strokeLinejoin="round" aria-hidden="true">
      <path d={direction === "left" ? "M10 3 5 8l5 5" : "M6 3l5 5-5 5"} />
    </svg>
  );
}

export default function App() {
  const [state, setState] = useState<LauncherState | null>(null);
  const [currentPage, setCurrentPage] = useState(0);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    fetchState();
    
    const unlisten = listen('tauri://drop', async (event: any) => {
      const appPath = pickAppPath(event.payload.paths ?? []);
      if (appPath) {
        try {
          await invoke("add_app_to_first_empty", { pageIdx: currentPage, path: appPath });
          setError(null);
        } catch (e) {
          setError(String(e));
        }
        fetchState();
      }
    });
    return () => { unlisten.then(f => f()); };
  }, [currentPage]);

  async function fetchState() {
    const s = await invoke<LauncherState>("get_state");
    setState(s);
  }

  async function handleLaunch(app: AppItem | null) {
    if (!app) return;
    await invoke("launch_app", { path: app.path });
  }

  if (!state) return <div>Loading...</div>;

  const currentSlots = state.pages[currentPage];

  return (
    <div className="container">
      <div className="drag-region" data-tauri-drag-region />
      <div className="grid">
        {currentSlots.map((app, i) => (
          <div key={i} className={`slot ${app ? 'filled' : ''}`} onClick={() => handleLaunch(app)}>
            {app ? (<><AppIcon path={app.path} name={app.name} /><div role="tooltip" className="tooltip">{app.name}</div></>) : (<div className="slot-number">{i + 1}</div>)}
          </div>
        ))}
      </div>
      {error && <p role="alert" className="error">登録できません: {error}</p>}
      <div className="pager">
        <button aria-label="Previous page" onClick={() => setCurrentPage(p => clampPage(p - 1, state.max_pages))} disabled={currentPage === 0}><Chevron direction="left" /></button>
        <span>{currentPage + 1} / {state.max_pages}</span>
        <button aria-label="Next page" onClick={() => setCurrentPage(p => clampPage(p + 1, state.max_pages))} disabled={currentPage >= state.max_pages - 1}><Chevron direction="right" /></button>
      </div>
    </div>
  );
}
