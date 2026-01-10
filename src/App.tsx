import React, { useEffect, useMemo, useState, useRef } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";
import { invoke } from "@tauri-apps/api/core";
import { listen } from '@tauri-apps/api/event'; // ★追加

// =========================================================================================
//  Type Definitions (型定義)
// =========================================================================================

// --- Chat & Memory ---
interface AxisToken {
  id: string;
  text: string;
  timestamp: number;
}

interface InteractionLog {
  id: string;
  session_id: string;
  timestamp: number;
  user_tokens: AxisToken[];
  ai_response: string;
  provider_used: string;
}

// --- System Vitals ---
interface SystemStats {
  cpu_usage: number;      // 0-100
  memory_used: number;    // bytes
  memory_total: number;   // bytes
  battery_level: number;  // 0-100
  is_charging: boolean;
}

// --- Boot Sequence ---
type BootStatus = "pending" | "running" | "ok" | "failed";

interface BootStep {
  id: number;
  label: string;
  detail?: string;
}

interface RenderedStep extends BootStep {
  status: BootStatus;
  timestamp: string;
}

// =========================================================================================
//  Constants & Config (定数設定)
// =========================================================================================

const BOOT_STEPS: BootStep[] = [
  { id: 1, label: "AxisOS Core", detail: "Initializing meta-OS kernel overlay" },
  { id: 2, label: "Gemini Engine", detail: "Logic layer online (reasoning / planning)" },
  { id: 3, label: "GPT Engine", detail: "Execution layer online (code / text / tools)" },
  { id: 4, label: "Grok Engine", detail: "Monitoring layer online (web / anomaly)" },
  { id: 5, label: "Neural Link", detail: "Connecting to Memory Banks..." },
  { id: 6, label: "Local Node", detail: "System Ready." },
];

const STEP_INTERVAL_MS = 600;
const BLINK_INTERVAL_MS = 600;

// =========================================================================================
//  Helper Functions (ヘルパー関数)
// =========================================================================================

function formatBootTime(date: Date): string {
  const hh = date.getHours().toString().padStart(2, "0");
  const mm = date.getMinutes().toString().padStart(2, "0");
  const ss = date.getSeconds().toString().padStart(2, "0");
  return `${hh}:${mm}:${ss}`;
}

function bytesToGB(bytes: number): string {
  return (bytes / (1024 * 1024 * 1024)).toFixed(1);
}

const playSound = (filename: string, vol: number = 0.5) => {
  const audio = new Audio(`/sounds/${filename}`);
  audio.volume = vol;
  audio.play().catch(() => { });
};

// =========================================================================================
//  Sub Components (UI Parts)
// =========================================================================================

// Window Controls Component (分離しました)
// ★重要: ボタン自体はドラッグさせない (親のヘッダーがドラッグ領域になるため)
const WindowControls = ({ isMaximized, toggleMaximize }: { isMaximized: boolean, toggleMaximize: () => void }) => {
  return (
    <div className="axis-window-controls">
      <button className="axis-control-btn" onClick={() => getCurrentWindow().minimize()} title="Minimize">─</button>
      <button
        className="axis-control-btn"
        onClick={toggleMaximize}
        // ★Win+Zでスナップレイアウトが出ることを示唆
        title={isMaximized ? "Restore (Win+Z)" : "Maximize (Win+Z)"}
      >
        {isMaximized ? "❐" : "◻"}
      </button>
      <button className="axis-control-btn close" onClick={() => getCurrentWindow().close()} title="Close">✕</button>
    </div>
  );
};

// Status Panel Component
const StatusPanel = ({ stats }: { stats: SystemStats | null }) => (
  <div className="axis-status-card">
    <div className="axis-status-line">
      <span className="axis-status-label">CPU LOAD</span>
      <span className="axis-status-value" style={{ color: (stats?.cpu_usage || 0) > 80 ? 'var(--axis-danger)' : 'var(--axis-primary)' }}>
        {stats ? `${stats.cpu_usage}%` : "CALC..."}
      </span>
    </div>
    <div className="axis-status-line">
      <span className="axis-status-label">MEMORY</span>
      <span className="axis-status-value">
        {stats ? `${bytesToGB(stats.memory_used)} / ${bytesToGB(stats.memory_total)} GB` : "SCANNING..."}
      </span>
    </div>
    <div className="axis-status-line">
      <span className="axis-status-label">POWER</span>
      <span className="axis-status-value">
        {stats ? `${stats.battery_level}%` : "AC NET"}
      </span>
    </div>
  </div>
);

// =========================================================================================
//  Main Component (App)
// =========================================================================================

const App: React.FC = () => {
  // ---------------------------------------------------------------------------------------
  //  State Management
  // ---------------------------------------------------------------------------------------

  // Boot & UI State
  const [bootIndex, setBootIndex] = useState<number>(-1);
  const [bootCompleted, setBootCompleted] = useState(false);
  const [bootStart] = useState<Date>(() => new Date());
  const [caretVisible, setCaretVisible] = useState(true);
  const [viewMode, setViewMode] = useState<'boot' | 'chat'>('boot');
  const [now, setNow] = useState<Date>(() => new Date());

  // Chat Data State
  const [logs, setLogs] = useState<InteractionLog[]>([]);
  const [sessionId, setSessionId] = useState<string>("");
  const [inputValue, setInputValue] = useState("");
  const [isThinking, setIsThinking] = useState(false);
  // ★追加: 日本語変換中かどうかを判定するフラグ
  const [isComposing, setIsComposing] = useState(false);
  // ★追加: テキストエリアの高さ制御用Ref
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  // System Vital State
  const [stats, setStats] = useState<SystemStats | null>(null);

  // Window Control State
  const [isMaximized, setIsMaximized] = useState(false);

  // Refs
  const chatEndRef = useRef<HTMLDivElement>(null);

  // ---------------------------------------------------------------------------------------
  //  Effects
  // ---------------------------------------------------------------------------------------

  // 1. Clock & Resize Listener
  useEffect(() => {
    const timer = setInterval(() => setNow(new Date()), 1000);

    // ウィンドウサイズ変更時に最大化状態をチェック
    const checkMaximized = async () => setIsMaximized(await getCurrentWindow().isMaximized());
    const unlisten = getCurrentWindow().onResized(checkMaximized);

    return () => {
      clearInterval(timer);
      unlisten.then(f => f());
    };
  }, []);

  // ★追加: 入力内容に応じてテキストエリアの高さを自動調整するEffect
  useEffect(() => {
    if (textareaRef.current) {
      // 一旦高さをリセットして縮める（削除時対応）
      textareaRef.current.style.height = "auto";
      // 内容に合わせて高さを設定 (最大200px程度で止めるなどの制限も可能)
      textareaRef.current.style.height = `${textareaRef.current.scrollHeight}px`;
    }
  }, [inputValue]);

  // 2. Boot Sequence
  useEffect(() => {
    if (bootCompleted) {
      const transitionTimer = setTimeout(() => setViewMode('chat'), 1500);
      return () => clearTimeout(transitionTimer);
    }

    const timer = setInterval(() => {
      setBootIndex((prev) => {
        const next = prev + 1;
        if (next >= BOOT_STEPS.length) {
          setBootCompleted(true);
          playSound('startup.mp3', 0.6);
          return prev;
        }
        playSound('beep.mp3', 0.3);
        return next;
      });
    }, STEP_INTERVAL_MS);

    return () => clearInterval(timer);
  }, [bootCompleted]);

  // 3. Caret Blink
  useEffect(() => {
    const timer = setInterval(() => setCaretVisible((v) => !v), BLINK_INTERVAL_MS);
    return () => clearInterval(timer);
  }, []);

  // 4. Keybindings (Esc to Minimize)
  useEffect(() => {
    const handleKeyDown = (event: KeyboardEvent) => {
      if (event.key === "Escape") {
        event.preventDefault();
        getCurrentWindow().minimize();
      }
    };
    window.addEventListener("keydown", handleKeyDown);
    return () => window.removeEventListener("keydown", handleKeyDown);
  }, []);

  // 5. Vital Monitor Polling
  useEffect(() => {
    if (!bootCompleted) return;
    const fetchStats = async () => {
      try {
        const data = await invoke<SystemStats>("get_vital_stats");
        setStats(data);
      } catch (e) { console.error("Vital Err:", e); }
    };
    fetchStats();
    const timer = setInterval(fetchStats, 2000);
    return () => clearInterval(timer);
  }, [bootCompleted]);

  // 6. History Loading
  useEffect(() => {
    const init = async () => {
      try {
        const history = await invoke<InteractionLog[]>("fetch_history");
        setLogs(history);
        if (history.length > 0) {
          setSessionId(history[history.length - 1].session_id);
        } else {
          setSessionId(crypto.randomUUID());
        }
      } catch (e) { console.error(e); }
    };
    init();
  }, []);

  // 7. Auto Scroll
  useEffect(() => {
    if (viewMode === 'chat') {
      chatEndRef.current?.scrollIntoView({ behavior: "smooth" });
    }
  }, [logs, viewMode, isThinking]);

  useEffect(() => {
    let unlisten: () => void;

    const setupListener = async () => {
      // "axis-observer-event" を監視
      unlisten = await listen<string>('axis-observer-event', (event) => {
        // 通知が来たらチャットログに追加
        const newMessage: InteractionLog = {
          id: crypto.randomUUID(),
          session_id: sessionId, // 現在のセッションに割り込み
          timestamp: Date.now(),
          user_tokens: [], // ユーザーの発言ではないので空
          ai_response: event.payload, // AIからの能動的な発言
          provider_used: "Observer" // 送信者名
        };

        setLogs(prev => [...prev, newMessage]);
        playSound('beep.mp3', 0.5); // 気づくように音を鳴らす
      });
    };

    setupListener();

    // クリーンアップ
    return () => {
      if (unlisten) unlisten();
    };
  }, [sessionId]); // sessionIDが変わっても追従するように

  // ---------------------------------------------------------------------------------------
  //  Logic & Handlers
  // ---------------------------------------------------------------------------------------

  const renderedSteps: RenderedStep[] = useMemo(() => {
    return BOOT_STEPS.map((step, index) => {
      let status: BootStatus = "pending";
      if (index < bootIndex) status = "ok";
      else if (index === bootIndex && !bootCompleted) status = "running";
      else if (bootCompleted && index === BOOT_STEPS.length - 1) status = "ok";

      const ts = new Date(bootStart.getTime() + Math.min(index, bootIndex) * STEP_INTERVAL_MS);
      return { ...step, status, timestamp: index <= bootIndex ? formatBootTime(ts) : "--:--:--" };
    });
  }, [bootIndex, bootCompleted, bootStart]);

  const sessions = useMemo(() => {
    const ids = Array.from(new Set(logs.map(l => l.session_id)));
    if (sessionId && !ids.includes(sessionId)) return [...ids, sessionId];
    return ids;
  }, [logs, sessionId]);

  const currentLogs = useMemo(() => logs.filter(l => l.session_id === sessionId), [logs, sessionId]);

  const handleNewChat = () => setSessionId(crypto.randomUUID());

  // 1. 送信の実行部（ボタンを押した時やEnterを押した時に呼ばれる実働部隊）
  const executeSend = async () => {
    if (inputValue.trim() && !isThinking) {
      const text = inputValue.trim();
      setInputValue("");
      setIsThinking(true);

      // ★「Thinkingが一瞬も出ない」対策（即時エラー/即時完了でレンダリングが飛ぶのを防ぐ）
      await new Promise(requestAnimationFrame);

      try {
        console.log("payload:", { input: text, sessionId });
        await invoke("ask_axis", { input: text, sessionId }); // ★ここだけにする
        const updated = await invoke<InteractionLog[]>("fetch_history");
        setLogs(updated);
      } catch (err) {
        console.error(err);
      } finally {
        setIsThinking(false);
      }
    }
  };

  // 2. キー入力の監視部（Shift+Enterか、ただのEnterかを仕分ける門番）
  const handleKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if (e.key === "Enter") {
      // 変換中(isComposing) または Shiftキー押下時 は送信せず改行を許可
      if (isComposing || e.shiftKey) {
        return;
      }
      // それ以外(確定後のEnter単体)なら送信実行
      e.preventDefault();
      executeSend();
    }
  };

  const handleDeleteSession = async (e: React.MouseEvent, targetSid: string) => {
    e.stopPropagation();
    if (!confirm("Purge this memory sector?")) return;
    try {
      console.log("payload:", { session_id: targetSid });
      await invoke("delete_history", { sessionId: targetSid });
      const updated = await invoke<InteractionLog[]>("fetch_history");
      setLogs(updated);
      if (targetSid === sessionId) setSessionId(crypto.randomUUID());
    } catch (err) { console.error("Delete Error:", err); }
  };

  // ★最大化トグルロジック
  const toggleMaximize = async () => {
    const win = getCurrentWindow();
    const max = await win.isMaximized();
    if (max) {
      await win.unmaximize();
      setIsMaximized(false);
    } else {
      await win.maximize();
      setIsMaximized(true);
    }
  };

  // ---------------------------------------------------------------------------------------
  //  Render
  // ---------------------------------------------------------------------------------------

  // === BOOT MODE ===
  if (viewMode === 'boot') {
    return (
      <div className="axis-root">
        <div className="axis-grid-overlay" />
        <div className="axis-shell">
          {/* Header (Drag Region Enabled) */}
          <header className="axis-header" data-tauri-drag-region>
            <div className="axis-header-left" style={{ pointerEvents: 'none' }}>
              <span className="axis-logo">AxisOS</span>
              <span className="axis-tag">META-OS OVERLAY</span>
              <span className="axis-version">v0.4</span>
            </div>
            <div className="axis-header-right" data-tauri-drag-region>
              <span className="axis-header-label">BOOT_SEQ</span>
              <span className="axis-header-separator">·</span>
              <span className="axis-header-value">{formatBootTime(now)}</span>
              <WindowControls isMaximized={isMaximized} toggleMaximize={toggleMaximize} />
            </div>
          </header>

          {/* Main Content */}
          <main className="axis-main">
            <section className="axis-boot-panel">
              <div className="axis-panel-title">BOOT SEQUENCE / LOG</div>
              <div className="axis-log-window">
                {renderedSteps.map((step) => (
                  <div key={step.id} className={`axis-log-line axis-log-${step.status}`}>
                    <span className="axis-log-time">[{step.timestamp}]</span>
                    <span className="axis-log-status">
                      {step.status === "ok" && "[OK]"}
                      {step.status === "running" && "[..]"}
                      {step.status === "failed" && "[ERR]"}
                      {step.status === "pending" && "    "}
                    </span>
                    <span className="axis-log-label">{step.label}</span>
                    {step.detail && <span className="axis-log-detail">– {step.detail}</span>}
                  </div>
                ))}
              </div>
            </section>

            <section className="axis-status-panel">
              <div className="axis-panel-title">SYSTEM STATUS</div>
              <StatusPanel stats={stats} />

              <div className="axis-panel-title axis-panel-title-mt">AXIS HINT</div>
              <div className="axis-hint-card">
                <p className="axis-hint-line">Initializing Neural Bridges...</p>
                <p className="axis-hint-line">Modules: [Nemotron-Brain] [Grok-Vision] [Shell-Hand]</p>
              </div>
            </section>
          </main>

          <footer className="axis-footer">
            <div className="axis-footer-line">
              <span className="axis-footer-prefix">AxisOS&gt;</span>
              <span className="axis-footer-text">
                {bootCompleted ? "SYSTEM READY. INITIALIZING UI..." : "LOADING KERNEL MODULES..."}
              </span>
            </div>
            <div className="axis-footer-console">
              <span className="axis-console-prompt">$</span>
              <span className="axis-console-input"></span>
              <span className={`axis-console-caret ${caretVisible ? "axis-console-caret-visible" : ""}`}>█</span>
            </div>
          </footer>
        </div>
      </div>
    );
  }

  // === CHAT MODE ===
  return (
    <div className="axis-root">
      <div className="axis-grid-overlay" />
      <div className="axis-shell">

        {/* Header: つまんで動かせる領域 */}
        <header className="axis-header" data-tauri-drag-region>
          <div className="axis-header-left" style={{ pointerEvents: 'none' }}>
            <span className="axis-logo">AxisOS</span>
            <span className="axis-tag">ONLINE</span>
            <span className="axis-version">v0.4</span>
          </div>
          <div className="axis-header-right" data-tauri-drag-region>
            <span className="axis-header-label">SESSION:</span>
            <span className="axis-header-value">{sessionId.substring(0, 8)}...</span>
            <span className="axis-header-separator">·</span>
            <span className="axis-header-value">{formatBootTime(now)}</span>

            {/* ボタン操作 */}
            <WindowControls isMaximized={isMaximized} toggleMaximize={toggleMaximize} />
          </div>
        </header>

        {/* Chat Layout */}
        <div className="axis-main axis-chat-layout">

          <aside className="axis-sidebar">
            <div className="axis-sidebar-header">MEMORY BANKS</div>
            <button className="axis-new-chat-btn" onClick={handleNewChat}>
              + Initialize New Thread
            </button>
            <div className="axis-thread-list">
              {sessions.map((sid, idx) => (
                <div
                  key={sid}
                  className={`axis-thread-item ${sid === sessionId ? 'active' : ''}`}
                  onClick={() => setSessionId(sid)}
                >
                  <div className="axis-thread-info">
                    Sector-{String(idx + 1).padStart(2, '0')}
                    <br />
                    <span className="axis-thread-preview">{sid.substring(0, 18)}...</span>
                  </div>
                  <button
                    className="axis-thread-delete"
                    onClick={(e) => handleDeleteSession(e, sid)}
                    title="Purge Memory"
                  >
                    ×
                  </button>
                </div>
              ))}
            </div>

            <div style={{ marginTop: 'auto' }}>
              <div className="axis-sidebar-header">VITAL MONITOR</div>
              <StatusPanel stats={stats} />
            </div>
          </aside>

          <section className="axis-chat-main">
            <div className="axis-chat-history">
              {currentLogs.length === 0 && (
                <div style={{ textAlign: 'center', marginTop: '40px', color: 'var(--axis-muted)' }}>
                  <p>Awaiting Input Protocol...</p>
                </div>
              )}

              {currentLogs.map((log) => (
                <React.Fragment key={log.id}>
                  <div className="axis-msg user">
                    <span className="axis-msg-sender">OPERATOR</span>
                    <div className="axis-msg-bubble">
                      {log.user_tokens.map(t => t.text).join(" ")}
                    </div>
                  </div>
                  <div className="axis-msg ai">
                    <span className="axis-msg-sender">{log.provider_used}</span>
                    <div className="axis-msg-bubble">
                      {log.ai_response}
                    </div>
                  </div>
                </React.Fragment>
              ))}

              {isThinking && (
                <div className="axis-msg ai">
                  <span className="axis-msg-sender">SYSTEM</span>
                  <div className="axis-msg-bubble">Thinking...</div>
                </div>
              )}
              <div ref={chatEndRef} />
            </div>

            <footer className="axis-footer" style={{ borderTop: '1px solid var(--axis-border)', borderRadius: '0' }}>
              <div className="axis-footer-console">
                <span className="axis-console-prompt">$</span>
                <textarea
                  ref={textareaRef}
                  className="axis-console-input"
                  value={inputValue}
                  onChange={(e) => setInputValue(e.target.value)}
                  onKeyDown={handleKeyDown}
                  // ★重要: IME変換開始・終了を検知
                  onCompositionStart={() => setIsComposing(true)}
                  onCompositionEnd={() => setIsComposing(false)}
                  placeholder="Execute command... (Shift+Enter for new line)"
                  autoFocus
                  disabled={isThinking}
                  rows={1} // 初期行数
                />
              </div>
            </footer>
          </section>

        </div>
      </div>
    </div>
  );
};

export default App;