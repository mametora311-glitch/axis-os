// src/components/TitleBar.tsx
import { getCurrentWindow } from '@tauri-apps/api/window';
import { useState, useEffect } from 'react';

// SFチックなアイコン (Lucide-reactなどがあればそれを使いますが、今回はSVG手打ちで軽量化)
const MinimizeIcon = () => <svg width="10" height="10" viewBox="0 0 10 10"><line x1="1" y1="5" x2="9" y2="5" stroke="currentColor" strokeWidth="2"/></svg>;
const MaximizeIcon = () => <svg width="10" height="10" viewBox="0 0 10 10"><rect x="1.5" y="1.5" width="7" height="7" stroke="currentColor" strokeWidth="2" fill="none"/></svg>;
const RestoreIcon = () => <svg width="10" height="10" viewBox="0 0 10 10"><rect x="3.5" y="1.5" width="5" height="5" stroke="currentColor" strokeWidth="2" fill="none"/><polyline points="1.5,3.5 1.5,8.5 6.5,8.5" stroke="currentColor" strokeWidth="2" fill="none"/></svg>;
const CloseIcon = () => <svg width="10" height="10" viewBox="0 0 10 10"><line x1="1" y1="1" x2="9" y2="9" stroke="currentColor" strokeWidth="2"/><line x1="9" y1="1" x2="1" y2="9" stroke="currentColor" strokeWidth="2"/></svg>;

export default function TitleBar() {
  const [isMaximized, setIsMaximized] = useState(false);

  // ウィンドウ状態の監視
  useEffect(() => {
    const checkStatus = async () => {
      const win = getCurrentWindow();
      setIsMaximized(await win.isMaximized());
    };
    checkStatus();

    // リサイズイベントのリスナー（簡易的）
    window.addEventListener('resize', checkStatus);
    return () => window.removeEventListener('resize', checkStatus);
  }, []);

  const handleMinimize = async () => getCurrentWindow().minimize();
  const handleMaximize = async () => {
    const win = getCurrentWindow();
    const max = await win.isMaximized();
    if (max) {
      win.unmaximize();
      setIsMaximized(false);
    } else {
      win.maximize();
      setIsMaximized(true);
    }
  };
  const handleClose = async () => getCurrentWindow().close();

  return (
    <div 
      className="titlebar" 
      data-tauri-drag-region // ★ここが重要！これがある場所がつまめる
    >
      <div className="logo-area" data-tauri-drag-region>
        <span className="logo-text">AxisOS v0.4</span>
        <span className="status-badge">ONLINE</span>
      </div>

      <div className="window-controls">
        <button onClick={handleMinimize} className="control-btn minimize" title="Minimize">
          <MinimizeIcon />
        </button>
        <button onClick={handleMaximize} className="control-btn maximize" title="Maximize (Win+Z for Snap)">
          {isMaximized ? <RestoreIcon /> : <MaximizeIcon />}
        </button>
        <button onClick={handleClose} className="control-btn close" title="Close">
          <CloseIcon />
        </button>
      </div>
    </div>
  );
}