import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { initialOf, letterColor } from "../lib/link";

/** 取り出したアイコン画像と、そのときのアイコンファイルの目印 (パスごと)。 */
const iconCache = new Map<string, { stamp: string; src: string }>();

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
  const [src, setSrc] = useState<string | null>(iconCache.get(path)?.src ?? null);
  const [iconFailed, setIconFailed] = useState(false);

  // 覚えた画像があればすぐに出し、アイコンファイルの目印が変わっていたときだけ取り出し直す。
  // アプリのアップデートは道具箱が裏にいる間に起きるので、ウィンドウが前面に来るたびに確かめる。
  useEffect(() => {
    if (kind === "url") return;
    setSrc(iconCache.get(path)?.src ?? null);
    let stale = false;
    const refresh = async () => {
      try {
        const stamp = await invoke<string>("get_app_icon_stamp", { path });
        const cached = iconCache.get(path);
        if (cached && cached.stamp === stamp) return;
        const dataUrl = await invoke<string>("get_app_icon", { path });
        if (typeof dataUrl !== "string") return;
        iconCache.set(path, { stamp, src: dataUrl });
        if (!stale) setSrc(dataUrl);
      } catch {
        // 確かめられなければ今の表示のままにする
      }
    };
    refresh();
    window.addEventListener("focus", refresh);
    return () => {
      stale = true;
      window.removeEventListener("focus", refresh);
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
