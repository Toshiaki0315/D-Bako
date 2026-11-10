const WEEKDAYS = ["日", "月", "火", "水", "木", "金", "土"];
const pad = (n: number) => String(n).padStart(2, "0");

/** 時計のページに出す文字 (時:分、秒、日付と曜日)。 */
export function formatClock(d: Date): { time: string; seconds: string; date: string } {
  return {
    time: `${pad(d.getHours())}:${pad(d.getMinutes())}`,
    seconds: pad(d.getSeconds()),
    date: `${d.getFullYear()}年${d.getMonth() + 1}月${d.getDate()}日 (${WEEKDAYS[d.getDay()]})`,
  };
}
