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

// ---------------------------------------------------------------------------
// Scene Template Types
// ---------------------------------------------------------------------------

export interface GenreSummary {
  id: string;
  display_name: string;
  description: string;
  archetype_count: number;
  profile_count: number;
  dynamic_count: number;
}

export interface ProfileSummary {
  id: string;
  display_name: string;
  description: string;
  scene_type: string;
  tension_min: number;
  tension_max: number;
  cast_size_min: number;
  cast_size_max: number;
}

export interface ArchetypeSummary {
  id: string;
  display_name: string;
  description: string;
}

export interface DynamicSummary {
  id: string;
  display_name: string;
  description: string;
  role_a: string;
  role_b: string;
}

export interface GenreOptions {
  profiles: ProfileSummary[];
  archetypes: ArchetypeSummary[];
  dynamics: DynamicSummary[];
  names: string[];
}

export interface CastSelection {
  archetype_id: string;
  name: string | null;
  role: string;
}

export interface DynamicSelection {
  dynamic_id: string;
  cast_index_a: number;
  cast_index_b: number;
}

export interface SceneSelections {
  genre_id: string;
  profile_id: string;
  cast: CastSelection[];
  dynamics: DynamicSelection[];
  setting_override: string | null;
  seed: number | null;
}

export interface SessionSummary {
  session_id: string;
  genre: string;
  profile: string;
  title: string;
  cast_names: string[];
  turn_count: number;
}

export interface TurnSummary {
  turn: number;
  player_input: string | null;
  narrator_output: string;
}

export interface ResumeResult {
  scene_info: SceneInfo;
  turns: TurnSummary[];
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

export interface DecomposedEntity {
  mention: string;
  category: string;
}

export interface DecomposedEvent {
  kind: string;
  actor: DecomposedEntity | null;
  action: string;
  target: DecomposedEntity | null;
  relational_direction: string;
  confidence_note: string | null;
}

export interface EventDecomposition {
  events: DecomposedEvent[];
  entities: DecomposedEntity[];
}

export interface EventDecomposedEvent {
  type: "event_decomposed";
  turn: number;
  decomposition: EventDecomposition | null;
  raw_llm_json: unknown | null;
  timing_ms: number;
  model: string;
  error: string | null;
}

export interface ActionArbitratedEvent {
  type: "action_arbitrated";
  turn: number;
  result: {
    verdict: "Permitted" | "Impossible" | "Ambiguous";
    conditions?: unknown[];
    reason?: { constraint_name: string; description: string };
    known_constraints?: unknown[];
    uncertainty?: string;
  };
  player_input: string;
  timing_ms: number;
}

export interface IntentSynthesizedEvent {
  type: "intent_synthesized";
  turn: number;
  intent_statements: string;
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
  | EventDecomposedEvent
  | ActionArbitratedEvent
  | IntentSynthesizedEvent
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
  decomposition: EventDecomposedEvent | null;
  arbitration: ActionArbitratedEvent | null;
  intent_synthesis: IntentSynthesizedEvent | null;
  narrator: NarratorCompleteEvent | null;
  error: ErrorEvent | null;
}

// ---------------------------------------------------------------------------
// Structured log streaming — events arrive on "workshop:logs" channel.
// ---------------------------------------------------------------------------

export const LOG_EVENT_CHANNEL = "workshop:logs";

export interface TracingLogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
  fields: Record<string, unknown>;
}
