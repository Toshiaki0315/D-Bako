import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Settings = { always_on_top: boolean; launch_at_login: boolean };

/** チェックボックスで切り替える項目と、変更を反映するコマンド。 */
const TOGGLES: { key: keyof Settings; label: string; command: string }[] = [
  { key: "always_on_top", label: "常に前に表示", command: "set_always_on_top" },
  { key: "launch_at_login", label: "起動時に実行する", command: "set_launch_at_login" },
];

/** 設定ウィンドウの中身。変更はその場で反映・保存される。 */
export function SettingsView() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
  }, []);

  /** 先に見た目を切り替え、保存・反映に失敗したら元に戻す。 */
  async function toggle(key: keyof Settings, command: string, enabled: boolean) {
    setSettings((s) => s && { ...s, [key]: enabled });
    setError(null);
    try {
      await invoke(command, { enabled });
    } catch (err) {
      setSettings((s) => s && { ...s, [key]: !enabled });
      setError(String(err));
    }
  }

  if (!settings) return null;

  return (
    <div className="settings-view">
      {TOGGLES.map(({ key, label, command }) => (
        <label key={key} className="setting-row">
          <input type="checkbox" checked={settings[key]} onChange={(e) => toggle(key, command, e.target.checked)} />
          {label}
        </label>
      ))}
      {error && <p role="alert" className="settings-error">変更できません: {error}</p>}
    </div>
  );
}
