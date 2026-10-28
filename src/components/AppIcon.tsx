import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { initialOf, letterColor } from "../lib/link";

const iconCache = new Map<string, string>();

export function clearIconCache() {
  iconCache.clear();
}

type Props = { path: string; name: string; kind?: "app" | "url"; icon?: string | null };

/**
 * スロットのアイコン画像。
 * - アプリ: `.app` から抽出した画像。取得できない間・失敗時は絵文字で代替する。
 * - URL ショートカット: 登録時に取得した favicon。無い・読めない場合は頭文字のタイルで代替する。
 */
export function AppIcon({ path, name, kind = "app", icon }: Props) {
  const [src, setSrc] = useState<string | null>(iconCache.get(path) ?? null);
  const [iconFailed, setIconFailed] = useState(false);

  useEffect(() => {
    if (kind === "url") return;
    const cached = iconCache.get(path);
    if (cached) {
      setSrc(cached);
      return;
    }
    setSrc(null);
    let stale = false;
    invoke<string>("get_app_icon", { path })
      .then((dataUrl) => {
        if (stale || typeof dataUrl !== "string") return;
        iconCache.set(path, dataUrl);
        setSrc(dataUrl);
      })
      .catch(() => {});
    return () => {
      stale = true;
    };
  }, [path, kind]);

  if (kind === "url") {
    if (icon && !iconFailed) {
      return <img className="app-icon app-icon-favicon" src={icon} alt={name} draggable={false} onError={() => setIconFailed(true)} />;
    }
    return (
      <div className="app-icon app-icon-letter" style={{ background: letterColor(name) }}>
        {initialOf(name)}
      </div>
    );
  }

  if (!src) return <div className="app-icon app-icon-fallback">📱</div>;
  return <img className="app-icon" src={src} alt={name} draggable={false} />;
}
