import { useEffect, useState } from "react";
import type { ReactNode } from "react";
import { invoke } from "@tauri-apps/api/core";
import { formatClock } from "../lib/clock";
import { addTime, createCountdown, formatDuration, isFinished, pause, remaining, reset, start } from "../lib/countdown";
import type { Countdown } from "../lib/countdown";
import { createPomodoro, nextPhase, skipPhase } from "../lib/pomodoro";

const MIN = 60_000;
/** 表示を更新する間隔。秒の切り替わりから大きく遅れないよう、1 秒より短くする。 */
const TICK_MS = 250;

/** 今の時刻 (ミリ秒) を、一定間隔で更新して返す。 */
function useNow(): number {
  const [now, setNow] = useState(Date.now);
  useEffect(() => {
    const id = setInterval(() => setNow(Date.now()), TICK_MS);
    return () => clearInterval(id);
  }, []);
  return now;
}

/** 終了を知らせる音。鳴らせなくても画面の表示で分かるので、失敗は無視する。 */
function playAlert() {
  invoke("play_alert").catch(() => {});
}

function WidgetPage({ label, children }: { label: string; children: ReactNode }) {
  return <section className="widget-page" aria-label={label}>{children}</section>;
}

/** 開始 / 一時停止と、その他の操作ボタン。 */
function Controls({ countdown, now, onStart, onPause, children }: { countdown: Countdown; now: number; onStart: () => void; onPause: () => void; children: ReactNode }) {
  const running = countdown.endsAt !== null;
  return (
    <div className="widget-controls">
      {running
        ? <button type="button" onClick={onPause}>一時停止</button>
        : <button type="button" onClick={onStart} disabled={remaining(countdown, now) <= 0}>開始</button>}
      {children}
    </div>
  );
}

export function ClockPage() {
  const { time, seconds, date } = formatClock(new Date(useNow()));
  return (
    <WidgetPage label="時計">
      <div className="widget-display" role="timer">
        {time}<span className="widget-seconds">{seconds}</span>
      </div>
      <p className="widget-sub">{date}</p>
    </WidgetPage>
  );
}

export function TimerPage() {
  const now = useNow();
  const [countdown, setCountdown] = useState(() => createCountdown(0));
  const [done, setDone] = useState(false);

  useEffect(() => {
    if (!isFinished(countdown, now)) return;
    setCountdown(reset(countdown));
    setDone(true);
    playAlert();
  }, [now]);

  /** 操作したら「時間になりました」は消す。 */
  function change(next: Countdown) {
    setCountdown(next);
    setDone(false);
  }

  /** 設定した時間に戻す。すでに戻っていれば 0 にする (時間を設定し直せるように)。 */
  function resetOrClear() {
    const atSetTime = countdown.endsAt === null && countdown.remainingMs === countdown.durationMs;
    change(atSetTime ? createCountdown(0) : reset(countdown));
  }

  return (
    <WidgetPage label="タイマー">
      <div className="widget-display" role="timer">{formatDuration(remaining(countdown, now))}</div>
      <div className="widget-controls">
        {[1, 5, 10].map((m) => (
          <button key={m} type="button" onClick={() => change(addTime(countdown, m * MIN, Date.now()))}>+{m}分</button>
        ))}
      </div>
      <Controls countdown={countdown} now={now} onStart={() => change(start(countdown, Date.now()))} onPause={() => change(pause(countdown, Date.now()))}>
        <button type="button" onClick={resetOrClear}>リセット</button>
      </Controls>
      {done && <p role="status" className="widget-sub widget-done">時間になりました</p>}
    </WidgetPage>
  );
}

export function PomodoroPage() {
  const now = useNow();
  const [pomodoro, setPomodoro] = useState(createPomodoro);
  const [message, setMessage] = useState<string | null>(null);
  const { phase, countdown, completed } = pomodoro;

  // 終わったら音を鳴らして次の段階へ移る。次の段階は自分で始める。
  useEffect(() => {
    if (!isFinished(countdown, now)) return;
    setPomodoro(nextPhase(pomodoro));
    setMessage(phase === "work" ? "お疲れさまでした。休憩しましょう" : "休憩終了。作業に戻りましょう");
    playAlert();
  }, [now]);

  function change(next: Countdown) {
    setPomodoro({ ...pomodoro, countdown: next });
    setMessage(null);
  }

  return (
    <WidgetPage label="ポモドーロ">
      <p className={`widget-phase ${phase}`}>{phase === "work" ? "作業" : "休憩"}</p>
      <div className="widget-display" role="timer">{formatDuration(remaining(countdown, now))}</div>
      <Controls countdown={countdown} now={now} onStart={() => change(start(countdown, Date.now()))} onPause={() => change(pause(countdown, Date.now()))}>
        <button type="button" onClick={() => change(reset(countdown))}>リセット</button>
        <button type="button" onClick={() => { setPomodoro(skipPhase(pomodoro)); setMessage(null); }}>スキップ</button>
      </Controls>
      <p className="widget-sub">完了 {completed} 回</p>
      {message && <p role="status" className="widget-sub widget-done">{message}</p>}
    </WidgetPage>
  );
}
