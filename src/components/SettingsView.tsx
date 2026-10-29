import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Settings = { always_on_top: boolean };

/** 設定ウィンドウの中身。変更はその場でメインウィンドウに反映・保存される。 */
export function SettingsView() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
  }, []);

  /** 先に見た目を切り替え、保存・反映に失敗したら元に戻す。 */
  async function toggleAlwaysOnTop(enabled: boolean) {
    setSettings({ always_on_top: enabled });
    setError(null);
    try {
      await invoke("set_always_on_top", { enabled });
    } catch (err) {
      setSettings({ always_on_top: !enabled });
      setError(String(err));
    }
  }

  if (!settings) return null;

  return (
    <div className="settings-view">
      <label className="setting-row">
        <input type="checkbox" checked={settings.always_on_top} onChange={(e) => toggleAlwaysOnTop(e.target.checked)} />
        常に前に表示
      </label>
      {error && <p role="alert" className="settings-error">変更できません: {error}</p>}
    </div>
  );
}
