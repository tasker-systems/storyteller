/**
 * Pure functions extracted from Svelte components for testability.
 *
 * These functions contain the non-trivial logic that could drift if
 * surrounding code changes. Each is a pure transformation with no
 * Svelte reactivity or side effects.
 */

import type {
  StoryBlock,
  ResumeResult,
  CastSelection,
  ProfileSummary,
  PhaseStatus,
  DebugState,
  DebugEvent,
  LlmStatus,
  EventDecomposedEvent,
  ActionArbitratedEvent,
  GoalsGeneratedEvent,
} from "./types";

// ---------------------------------------------------------------------------
// Session resume hydration
// ---------------------------------------------------------------------------

export interface HydrationResult {
  blocks: StoryBlock[];
  turnCount: number;
}

/**
 * Convert a ResumeResult (from the backend) into StoryBlocks for the chat UI.
 *
 * Turn 0 becomes an "opening" block. Subsequent turns produce a "player"
 * block (if player_input is present) followed by a "narrator" block.
 */
export function hydrateBlocks(result: ResumeResult): HydrationResult {
  const blocks: StoryBlock[] = [];

  if (result.turns.length === 0) {
    blocks.push({ kind: "opening", text: result.scene_info.opening_prose });
    return { blocks, turnCount: 0 };
  }

  for (const turn of result.turns) {
    if (turn.turn === 0) {
      blocks.push({ kind: "opening", text: turn.narrator_output });
    } else {
      if (turn.player_input != null) {
        blocks.push({ kind: "player", turn: turn.turn, text: turn.player_input });
      }
      blocks.push({ kind: "narrator", turn: turn.turn, text: turn.narrator_output });
    }
  }

  const turnCount = result.turns[result.turns.length - 1].turn;
  return { blocks, turnCount };
}

// ---------------------------------------------------------------------------
// Wizard step validation
// ---------------------------------------------------------------------------

export interface WizardState {
  selectedGenreId: string | null;
  selectedProfileId: string | null;
  cast: CastSelection[];
  launching: boolean;
}

/**
 * Determine whether the wizard can advance from the given step.
 */
export function canAdvance(
  step: number,
  state: WizardState,
  selectedProfile: ProfileSummary | null,
): boolean {
  switch (step) {
    case 0:
      return state.selectedGenreId !== null;
    case 1:
      return state.selectedProfileId !== null;
    case 2:
      return (
        state.cast.length >= (selectedProfile?.cast_size_min ?? 1) &&
        state.cast.every((c) => c.archetype_id !== "" && (c.name ?? "").trim() !== "") &&
        state.cast.filter((c) => c.role === "protagonist").length === 1
      );
    case 3:
      return true; // dynamics are optional
    case 4:
      return true; // setting override is optional
    case 5:
      return !state.launching;
    default:
      return false;
  }
}

/**
 * Compute the next step when advancing, handling the dynamics-skip rule.
 * Returns the new step number.
 */
export function nextStep(currentStep: number, castLength: number): number {
  // Skip dynamics step if fewer than 2 cast members
  if (currentStep === 2 && castLength < 2) {
    return 4;
  }
  return Math.min(currentStep + 1, 5);
}

/**
 * Compute the previous step when going back, handling the dynamics-skip rule.
 * Returns the new step number.
 */
export function prevStep(currentStep: number, castLength: number): number {
  if (currentStep === 4 && castLength < 2) {
    return 2;
  }
  return Math.max(currentStep - 1, 0);
}

// ---------------------------------------------------------------------------
// Cast management
// ---------------------------------------------------------------------------

/**
 * Collect the set of names already used by cast members.
 */
export function usedNames(cast: CastSelection[]): Set<string> {
  return new Set(cast.map((c) => c.name ?? "").filter((n) => n !== ""));
}

/**
 * Find the next unused name from the pool.
 */
export function nextUnusedName(cast: CastSelection[], namePool: string[]): string {
  const used = usedNames(cast);
  for (const name of namePool) {
    if (!used.has(name)) return name;
  }
  return "";
}

/**
 * Compute all unique character pairs (combinations) for dynamics assignment.
 */
export function castPairs(
  cast: CastSelection[],
): { a: number; b: number; labelA: string; labelB: string }[] {
  const pairs: { a: number; b: number; labelA: string; labelB: string }[] = [];
  for (let i = 0; i < cast.length; i++) {
    for (let j = i + 1; j < cast.length; j++) {
      pairs.push({
        a: i,
        b: j,
        labelA: cast[i].name ?? `Character ${i + 1}`,
        labelB: cast[j].name ?? `Character ${j + 1}`,
      });
    }
  }
  return pairs;
}

// ---------------------------------------------------------------------------
// Debug panel phase status
// ---------------------------------------------------------------------------

/**
 * Compute the phase status indicator for a debug panel tab.
 *
 * The Events tab is special: it combines classification + decomposition
 * phases with error > processing > complete > pending precedence.
 */
export function phaseStatus(
  tab: string,
  debugState: DebugState,
  llmStatus: LlmStatus | null,
  llmChecking: boolean,
): PhaseStatus {
  const TAB_PHASE_MAP: Record<string, string> = {
    LLM: "llm",
    Context: "context",
    "ML Predictions": "prediction",
    Characters: "characters",
    Events: "events",
    Arbitration: "arbitration",
    Goals: "goals",
    Narrator: "narrator",
    Logs: "logs",
  };

  if (tab === "LLM") {
    if (llmChecking) return "processing";
    if (!llmStatus) return "pending";
    return llmStatus.reachable ? "complete" : "error";
  }

  if (tab === "Events") {
    const evtStatus = debugState.phases["events"] ?? "pending";
    const decStatus = debugState.phases["decomposition"] ?? "pending";
    if (evtStatus === "error" || decStatus === "error") return "error";
    if (evtStatus === "processing" || decStatus === "processing") return "processing";
    if (evtStatus === "complete" && decStatus === "complete") return "complete";
    if (evtStatus === "complete" || decStatus === "complete") return "processing";
    return "pending";
  }

  const phase = TAB_PHASE_MAP[tab];
  return debugState.phases[phase] ?? "pending";
}

/**
 * Create a fresh DebugState for a new turn.
 */
export function freshDebugState(turn: number): DebugState {
  return {
    turn,
    phases: {},
    prediction: null,
    context: null,
    characters: null,
    events: null,
    decomposition: null,
    arbitration: null,
    intent_synthesis: null,
    goals: null,
    narrator: null,
    error: null,
  };
}

/**
 * Apply a debug event to the current state, returning a new state.
 * Resets state if the event's turn differs from the current turn.
 */
export function applyDebugEvent(state: DebugState, event: DebugEvent): DebugState {
  let next = event.turn !== state.turn ? freshDebugState(event.turn) : { ...state, phases: { ...state.phases } };

  switch (event.type) {
    case "phase_started":
      next.phases[event.phase] = "processing";
      break;
    case "prediction_complete":
      next.prediction = event;
      next.phases["prediction"] = "complete";
      break;
    case "context_assembled":
      next.context = event;
      next.phases["context"] = "complete";
      break;
    case "characters_updated":
      next.characters = event;
      next.phases["characters"] = "complete";
      break;
    case "events_classified":
      next.events = event;
      next.phases["events"] = "complete";
      break;
    case "event_decomposed":
      next.decomposition = event as EventDecomposedEvent;
      next.phases["decomposition"] = event.error ? "error" : "complete";
      break;
    case "action_arbitrated":
      next.arbitration = event as ActionArbitratedEvent;
      next.phases["arbitration"] = "complete";
      break;
    case "intent_synthesized":
      next.intent_synthesis = event;
      next.phases["intent_synthesis"] = "complete";
      break;
    case "goals_generated":
      next.goals = event as GoalsGeneratedEvent;
      next.phases["goals"] = "complete";
      break;
    case "narrator_complete":
      next.narrator = event;
      next.phases["narrator"] = "complete";
      break;
    case "error":
      next.error = event;
      next.phases[event.phase] = "error";
      break;
  }

  return next;
}
