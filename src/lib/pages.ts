/** 時計・ポモドーロ・タイマーのページを表示するか (Rust 側の `settings::Widgets` と同じ形)。 */
export type Widgets = { clock: boolean; pomodoro: boolean; timer: boolean };
export type WidgetKind = keyof Widgets;

/** ページ送りで並ぶ 1 ページ。ランチャーのページは何ページ目かを持つ。 */
export type Page = { kind: "launcher"; index: number } | { kind: WidgetKind };

const WIDGET_ORDER: WidgetKind[] = ["clock", "pomodoro", "timer"];

/** ランチャーのページを先に、表示する時計などのページをその後ろに並べる。 */
export function buildPages(launcherPages: number, widgets: Widgets | undefined): Page[] {
  const launcher: Page[] = Array.from({ length: launcherPages }, (_, index) => ({ kind: "launcher", index }));
  const extra: Page[] = WIDGET_ORDER.filter((kind) => widgets?.[kind]).map((kind) => ({ kind }));
  return [...launcher, ...extra];
}
