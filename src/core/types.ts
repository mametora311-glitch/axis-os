// src/core/types.ts

// AIエンジンの種類
export type AIProvider = 'gpt' | 'gemini' | 'grok' | 'claude' | 'nemotron' | 'sd' | 'sora';

// ユーザー入力/AI応答の最小単位（単語単位管理用）
export interface AxisToken {
  id: string;
  text: string;
  timestamp: number;
  tags: string[]; // 検索用タグ (例: "important", "code", "intent:creation")
  vector?: number[]; // 将来的な埋め込みベクトル用
}

// 1回のやり取り（ログ）
export interface InteractionLog {
  id: string;
  sessionId: string;
  timestamp: number;
  userTokens: AxisToken[]; // 分解されたユーザー入力
  aiResponse: string;
  providerUsed: AIProvider;
}

// 司令塔の状態
export interface CommanderState {
  activeProvider: AIProvider;
  isProcessing: boolean;
  systemStatus: 'IDLE' | 'THINKING' | 'EXECUTING' | 'ERROR';
}