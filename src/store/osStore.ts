import { create } from 'zustand';
import { CommanderState, InteractionLog } from '../core/types';

interface OSState extends CommanderState {
  logs: InteractionLog[];
  
  // Actions
  setStatus: (status: CommanderState['systemStatus']) => void;
  setProvider: (provider: CommanderState['activeProvider']) => void;
  addLog: (log: InteractionLog) => void;
  clearLogs: () => void;
}

export const useOSStore = create<OSState>((set) => ({
  // 初期状態
  activeProvider: 'gemini', // デフォルト司令塔
  isProcessing: false,
  systemStatus: 'IDLE',
  logs: [],

  // アクション実装
  setStatus: (status) => set({ 
    systemStatus: status, 
    isProcessing: status === 'THINKING' || status === 'EXECUTING' 
  }),
  setProvider: (provider) => set({ activeProvider: provider }),
  
  addLog: (log) => set((state) => ({ 
    logs: [...state.logs, log] 
  })),
  
  clearLogs: () => set({ logs: [] }),
}));
