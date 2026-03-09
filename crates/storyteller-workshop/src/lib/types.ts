export interface SceneInfo {
  title: string;
  setting_description: string;
  cast: string[];
  opening_prose: string;
}

export interface TurnResult {
  turn: number;
  narrator_prose: string;
  timing: TurnTiming;
  context_tokens: ContextTokens;
}

export interface TurnTiming {
  prediction_ms: number;
  assembly_ms: number;
  narrator_ms: number;
}

export interface ContextTokens {
  preamble: number;
  journal: number;
  retrieved: number;
  total: number;
}

export interface LogEntry {
  turn: number;
  timestamp: string;
  player_input: string;
  narrator_output: string;
  context_assembly: {
    preamble_tokens: number;
    journal_tokens: number;
    retrieved_tokens: number;
    total_tokens: number;
  };
  timing: {
    prediction_ms: number;
    assembly_ms: number;
    narrator_ms: number;
  };
}

export type StoryBlock =
  | { kind: "narrator"; turn: number; text: string }
  | { kind: "player"; turn: number; text: string }
  | { kind: "opening"; text: string };
