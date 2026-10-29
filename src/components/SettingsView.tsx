import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

type Settings = { always_on_top: boolean; launch_at_login: boolean };
type Layout = { cols: number; rows: number; pages: number };

/** グリッドの横・縦の数とページ数の選択肢 (Rust 側の MAX_COLS / MAX_ROWS / MAX_PAGES と合わせる)。 */
const LAYOUT_FIELDS: { key: keyof Layout; label: string; max: number }[] = [
  { key: "cols", label: "横の数", max: 8 },
  { key: "rows", label: "縦の数", max: 6 },
  { key: "pages", label: "ページ数", max: 10 },
];

/** チェックボックスで切り替える項目と、変更を反映するコマンド。 */
const TOGGLES: { key: keyof Settings; label: string; command: string }[] = [
  { key: "always_on_top", label: "常に前に表示", command: "set_always_on_top" },
  { key: "launch_at_login", label: "起動時に実行する", command: "set_launch_at_login" },
];

/** 設定ウィンドウの中身。変更はその場で反映・保存される。 */
export function SettingsView() {
  const [settings, setSettings] = useState<Settings | null>(null);
  const [layout, setLayout] = useState<Layout | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<Settings>("get_settings").then(setSettings);
    invoke<Layout>("get_layout").then(setLayout);
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

  /** 先に見た目を変え、反映できなければ (アイテムが収まらないなど) 元に戻す。 */
  async function changeLayout(next: Layout) {
    const before = layout;
    setLayout(next);
    setError(null);
    try {
      await invoke("set_layout", next);
    } catch (err) {
      setLayout(before);
      setError(String(err));
    }
  }

  if (!settings) return null;

  return (
    <div className="settings-view">
      <fieldset className="settings-group">
        <legend>一般</legend>
        <div className="settings-box">
          {TOGGLES.map(({ key, label, command }) => (
            <label key={key} className="settings-row">
              <span>{label}</span>
              <input type="checkbox" checked={settings[key]} onChange={(e) => toggle(key, command, e.target.checked)} />
            </label>
          ))}
        </div>
      </fieldset>
      {layout && (
        <fieldset className="settings-group">
          <legend>グリッド</legend>
          <div className="settings-box">
            {LAYOUT_FIELDS.map(({ key, label, max }) => (
              <label key={key} className="settings-row">
                <span>{label}</span>
                <select value={layout[key]} onChange={(e) => changeLayout({ ...layout, [key]: Number(e.target.value) })}>
                  {Array.from({ length: max }, (_, i) => i + 1).map((n) => <option key={n} value={n}>{n}</option>)}
                </select>
              </label>
            ))}
          </div>
        </fieldset>
      )}
      {error && <p role="alert" className="settings-error">変更できません: {error}</p>}
    </div>
  );
}
