import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";

const iconCache = new Map<string, string>();

export function clearIconCache() {
  iconCache.clear();
}

type Props = { path: string; name: string };

/** アプリのアイコン画像。取得できない間・失敗時は絵文字で代替表示する。 */
export function AppIcon({ path, name }: Props) {
  const [src, setSrc] = useState<string | null>(iconCache.get(path) ?? null);

  useEffect(() => {
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
  }, [path]);

  if (!src) return <div style={{ fontSize: "32px" }}>📱</div>;
  return <img src={src} alt={name} width={48} height={48} />;
}
