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

export interface LlmStatus {
  reachable: boolean;
  endpoint: string;
  model: string;
  provider: string;
  available_models: string[];
  error: string | null;
  latency_ms: number;
}

export type StoryBlock =
  | { kind: "narrator"; turn: number; text: string }
  | { kind: "player"; turn: number; text: string }
  | { kind: "opening"; text: string };

// ---------------------------------------------------------------------------
// Debug inspector events — discriminated union on "type" field.
// All events arrive on the single "workshop:debug" Tauri event channel.
// ---------------------------------------------------------------------------

export const DEBUG_EVENT_CHANNEL = "workshop:debug";

export interface PhaseStartedEvent {
  type: "phase_started";
  turn: number;
  phase: string;
}

export interface PredictionCompleteEvent {
  type: "prediction_complete";
  turn: number;
  resolver_output: {
    sequenced_actions: unknown[];
    original_predictions: unknown[];
    scene_dynamics: string;
    conflicts: unknown[];
  };
  timing_ms: number;
  model_loaded: boolean;
}

export interface ContextAssembledEvent {
  type: "context_assembled";
  turn: number;
  preamble_text: string;
  journal_text: string;
  retrieved_text: string;
  token_counts: {
    preamble: number;
    journal: number;
    retrieved: number;
    total: number;
  };
  timing_ms: number;
}

export interface CharactersUpdatedEvent {
  type: "characters_updated";
  turn: number;
  characters: unknown[];
  emotional_markers: string[];
}

export interface EventsClassifiedEvent {
  type: "events_classified";
  turn: number;
  classifications: string[];
  classifier_loaded: boolean;
}

export interface NarratorCompleteEvent {
  type: "narrator_complete";
  turn: number;
  system_prompt: string;
  user_message: string;
  raw_response: string;
  model: string;
  temperature: number;
  max_tokens: number;
  tokens_used: number;
  timing_ms: number;
}

export interface ErrorEvent {
  type: "error";
  turn: number;
  phase: string;
  message: string;
}

export type DebugEvent =
  | PhaseStartedEvent
  | PredictionCompleteEvent
  | ContextAssembledEvent
  | CharactersUpdatedEvent
  | EventsClassifiedEvent
  | NarratorCompleteEvent
  | ErrorEvent;

export type PhaseStatus = "pending" | "processing" | "complete" | "skipped" | "error";

export interface DebugState {
  turn: number;
  phases: Record<string, PhaseStatus>;
  prediction: PredictionCompleteEvent | null;
  context: ContextAssembledEvent | null;
  characters: CharactersUpdatedEvent | null;
  events: EventsClassifiedEvent | null;
  narrator: NarratorCompleteEvent | null;
  error: ErrorEvent | null;
}
