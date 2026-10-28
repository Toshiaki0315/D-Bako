import { useState, useEffect } from "react";
import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";
import { clampPage, pickAppPath } from "./lib/launcher";
import { AppIcon } from "./components/AppIcon";

type AppItem = { path: string; name: string };
type LauncherState = { pages: (AppItem | null)[][]; max_pages: number };

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
      <div className="header">
        <button onClick={() => setCurrentPage(p => clampPage(p - 1, state.max_pages))} disabled={currentPage === 0}>◀ Prev</button>
        <span>Page {currentPage + 1} / {state.max_pages}</span>
        <button onClick={() => setCurrentPage(p => clampPage(p + 1, state.max_pages))} disabled={currentPage >= state.max_pages - 1}>Next ▶</button>
      </div>
      <div className="grid">
        {currentSlots.map((app, i) => (
          <div key={i} className={`slot ${app ? 'filled' : ''}`} onClick={() => handleLaunch(app)}>
            {app ? (<><AppIcon path={app.path} name={app.name} /><div className="app-name">{app.name}</div></>) : (<div style={{color: '#555'}}>{i + 1}</div>)}
          </div>
        ))}
      </div>
      {error && <p role="alert" style={{marginTop: '12px', color: '#ff8a80', fontSize: '12px'}}>登録できません: {error}</p>}
      <p style={{marginTop: '20px', color: '#888', fontSize: '12px'}}>※ .app ファイルをウィンドウにドラッグ＆ドロップして登録します。</p>
    </div>
  );
}
