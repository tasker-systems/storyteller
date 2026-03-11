import { describe, it, expect } from "vitest";
import {
  hydrateBlocks,
  canAdvance,
  nextStep,
  prevStep,
  usedNames,
  nextUnusedName,
  castPairs,
  phaseStatus,
  freshDebugState,
  applyDebugEvent,
} from "./logic";
import type {
  ResumeResult,
  CastSelection,
  DebugState,
  DebugEvent,
  LlmStatus,
  PhaseStartedEvent,
  EventDecomposedEvent,
} from "./types";

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

function makeSceneInfo(opening = "Once upon a time...") {
  return {
    title: "Test Scene",
    setting_description: "A dark room",
    cast: ["Alice", "Bob"],
    opening_prose: opening,
  };
}

function makeCast(...names: string[]): CastSelection[] {
  return names.map((name, i) => ({
    archetype_id: `arch_${i}`,
    name,
    role: i === 0 ? "protagonist" : "cast",
  }));
}

// ---------------------------------------------------------------------------
// hydrateBlocks
// ---------------------------------------------------------------------------

describe("hydrateBlocks", () => {
  it("returns opening from scene_info when turns are empty", () => {
    const result: ResumeResult = {
      scene_info: makeSceneInfo("The door creaks open."),
      turns: [],
    };
    const { blocks, turnCount } = hydrateBlocks(result);
    expect(blocks).toEqual([{ kind: "opening", text: "The door creaks open." }]);
    expect(turnCount).toBe(0);
  });

  it("renders turn 0 as opening block", () => {
    const result: ResumeResult = {
      scene_info: makeSceneInfo(),
      turns: [{ turn: 0, player_input: null, narrator_output: "The scene begins." }],
    };
    const { blocks, turnCount } = hydrateBlocks(result);
    expect(blocks).toEqual([{ kind: "opening", text: "The scene begins." }]);
    expect(turnCount).toBe(0);
  });

  it("renders multiple turns with player + narrator blocks", () => {
    const result: ResumeResult = {
      scene_info: makeSceneInfo(),
      turns: [
        { turn: 0, player_input: null, narrator_output: "Opening." },
        { turn: 1, player_input: "I look around.", narrator_output: "You see a room." },
        { turn: 2, player_input: "I open the door.", narrator_output: "The door opens." },
      ],
    };
    const { blocks, turnCount } = hydrateBlocks(result);
    expect(blocks).toEqual([
      { kind: "opening", text: "Opening." },
      { kind: "player", turn: 1, text: "I look around." },
      { kind: "narrator", turn: 1, text: "You see a room." },
      { kind: "player", turn: 2, text: "I open the door." },
      { kind: "narrator", turn: 2, text: "The door opens." },
    ]);
    expect(turnCount).toBe(2);
  });

  it("skips player block when player_input is null on non-zero turn", () => {
    const result: ResumeResult = {
      scene_info: makeSceneInfo(),
      turns: [
        { turn: 0, player_input: null, narrator_output: "Opening." },
        { turn: 1, player_input: null, narrator_output: "Something happens." },
      ],
    };
    const { blocks } = hydrateBlocks(result);
    // Turn 1 has no player_input — only narrator block
    expect(blocks).toEqual([
      { kind: "opening", text: "Opening." },
      { kind: "narrator", turn: 1, text: "Something happens." },
    ]);
  });

  it("sets turnCount to last turn number", () => {
    const result: ResumeResult = {
      scene_info: makeSceneInfo(),
      turns: [
        { turn: 0, player_input: null, narrator_output: "A." },
        { turn: 1, player_input: "B.", narrator_output: "C." },
        { turn: 5, player_input: "D.", narrator_output: "E." },
      ],
    };
    const { turnCount } = hydrateBlocks(result);
    expect(turnCount).toBe(5);
  });
});

// ---------------------------------------------------------------------------
// canAdvance
// ---------------------------------------------------------------------------

describe("canAdvance", () => {
  const profile = {
    id: "p1",
    display_name: "Test",
    description: "",
    scene_type: "Gravitational",
    tension_min: 1,
    tension_max: 5,
    cast_size_min: 2,
    cast_size_max: 4,
  };

  it("step 0: requires genre selection", () => {
    expect(canAdvance(0, { selectedGenreId: null, selectedProfileId: null, cast: [], launching: false }, null)).toBe(false);
    expect(canAdvance(0, { selectedGenreId: "fantasy", selectedProfileId: null, cast: [], launching: false }, null)).toBe(true);
  });

  it("step 1: requires profile selection", () => {
    expect(canAdvance(1, { selectedGenreId: "fantasy", selectedProfileId: null, cast: [], launching: false }, null)).toBe(false);
    expect(canAdvance(1, { selectedGenreId: "fantasy", selectedProfileId: "p1", cast: [], launching: false }, null)).toBe(true);
  });

  it("step 2: validates cast size, archetypes, names, and protagonist", () => {
    const state = { selectedGenreId: "f", selectedProfileId: "p1", cast: makeCast("Alice", "Bob"), launching: false };
    expect(canAdvance(2, state, profile)).toBe(true);
  });

  it("step 2: rejects cast below minimum", () => {
    const state = { selectedGenreId: "f", selectedProfileId: "p1", cast: makeCast("Alice"), launching: false };
    expect(canAdvance(2, state, profile)).toBe(false);
  });

  it("step 2: rejects empty archetype_id", () => {
    const cast: CastSelection[] = [
      { archetype_id: "hero", name: "Alice", role: "protagonist" },
      { archetype_id: "", name: "Bob", role: "cast" },
    ];
    expect(canAdvance(2, { selectedGenreId: "f", selectedProfileId: "p1", cast, launching: false }, profile)).toBe(false);
  });

  it("step 2: rejects empty name", () => {
    const cast: CastSelection[] = [
      { archetype_id: "hero", name: "Alice", role: "protagonist" },
      { archetype_id: "sidekick", name: "", role: "cast" },
    ];
    expect(canAdvance(2, { selectedGenreId: "f", selectedProfileId: "p1", cast, launching: false }, profile)).toBe(false);
  });

  it("step 2: rejects whitespace-only name", () => {
    const cast: CastSelection[] = [
      { archetype_id: "hero", name: "Alice", role: "protagonist" },
      { archetype_id: "sidekick", name: "   ", role: "cast" },
    ];
    expect(canAdvance(2, { selectedGenreId: "f", selectedProfileId: "p1", cast, launching: false }, profile)).toBe(false);
  });

  it("step 2: rejects null name", () => {
    const cast: CastSelection[] = [
      { archetype_id: "hero", name: "Alice", role: "protagonist" },
      { archetype_id: "sidekick", name: null, role: "cast" },
    ];
    expect(canAdvance(2, { selectedGenreId: "f", selectedProfileId: "p1", cast, launching: false }, profile)).toBe(false);
  });

  it("step 2: rejects zero protagonists", () => {
    const cast: CastSelection[] = [
      { archetype_id: "hero", name: "Alice", role: "cast" },
      { archetype_id: "sidekick", name: "Bob", role: "cast" },
    ];
    expect(canAdvance(2, { selectedGenreId: "f", selectedProfileId: "p1", cast, launching: false }, profile)).toBe(false);
  });

  it("step 2: rejects multiple protagonists", () => {
    const cast: CastSelection[] = [
      { archetype_id: "hero", name: "Alice", role: "protagonist" },
      { archetype_id: "sidekick", name: "Bob", role: "protagonist" },
    ];
    expect(canAdvance(2, { selectedGenreId: "f", selectedProfileId: "p1", cast, launching: false }, profile)).toBe(false);
  });

  it("steps 3 and 4 always advance", () => {
    const state = { selectedGenreId: "f", selectedProfileId: "p1", cast: [], launching: false };
    expect(canAdvance(3, state, null)).toBe(true);
    expect(canAdvance(4, state, null)).toBe(true);
  });

  it("step 5: blocked while launching", () => {
    expect(canAdvance(5, { selectedGenreId: "f", selectedProfileId: "p1", cast: [], launching: true }, null)).toBe(false);
    expect(canAdvance(5, { selectedGenreId: "f", selectedProfileId: "p1", cast: [], launching: false }, null)).toBe(true);
  });

  it("unknown step returns false", () => {
    expect(canAdvance(99, { selectedGenreId: "f", selectedProfileId: "p1", cast: [], launching: false }, null)).toBe(false);
  });
});

// ---------------------------------------------------------------------------
// nextStep / prevStep (dynamics skip logic)
// ---------------------------------------------------------------------------

describe("nextStep", () => {
  it("advances normally", () => {
    expect(nextStep(0, 3)).toBe(1);
    expect(nextStep(1, 3)).toBe(2);
    expect(nextStep(3, 3)).toBe(4);
    expect(nextStep(4, 3)).toBe(5);
  });

  it("skips dynamics (step 3) when cast < 2", () => {
    expect(nextStep(2, 1)).toBe(4);
    expect(nextStep(2, 0)).toBe(4);
  });

  it("does not skip dynamics when cast >= 2", () => {
    expect(nextStep(2, 2)).toBe(3);
    expect(nextStep(2, 5)).toBe(3);
  });

  it("clamps at step 5", () => {
    expect(nextStep(5, 3)).toBe(5);
  });
});

describe("prevStep", () => {
  it("goes back normally", () => {
    expect(prevStep(5, 3)).toBe(4);
    expect(prevStep(3, 3)).toBe(2);
    expect(prevStep(1, 3)).toBe(0);
  });

  it("skips dynamics backwards when cast < 2", () => {
    expect(prevStep(4, 1)).toBe(2);
    expect(prevStep(4, 0)).toBe(2);
  });

  it("does not skip dynamics backwards when cast >= 2", () => {
    expect(prevStep(4, 2)).toBe(3);
  });

  it("clamps at step 0", () => {
    expect(prevStep(0, 3)).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Cast helpers
// ---------------------------------------------------------------------------

describe("usedNames", () => {
  it("collects non-empty names", () => {
    const cast = makeCast("Alice", "Bob");
    expect(usedNames(cast)).toEqual(new Set(["Alice", "Bob"]));
  });

  it("excludes null names", () => {
    const cast: CastSelection[] = [
      { archetype_id: "a", name: "Alice", role: "cast" },
      { archetype_id: "b", name: null, role: "cast" },
    ];
    expect(usedNames(cast)).toEqual(new Set(["Alice"]));
  });

  it("excludes empty string names", () => {
    const cast: CastSelection[] = [
      { archetype_id: "a", name: "", role: "cast" },
    ];
    expect(usedNames(cast)).toEqual(new Set());
  });

  it("returns empty set for empty cast", () => {
    expect(usedNames([])).toEqual(new Set());
  });
});

describe("nextUnusedName", () => {
  it("returns first unused name", () => {
    const cast = makeCast("Alice");
    expect(nextUnusedName(cast, ["Alice", "Bob", "Carol"])).toBe("Bob");
  });

  it("returns empty string when all names used", () => {
    const cast = makeCast("Alice", "Bob");
    expect(nextUnusedName(cast, ["Alice", "Bob"])).toBe("");
  });

  it("returns first name when cast is empty", () => {
    expect(nextUnusedName([], ["Alice", "Bob"])).toBe("Alice");
  });

  it("returns empty string when pool is empty", () => {
    expect(nextUnusedName([], [])).toBe("");
  });
});

describe("castPairs", () => {
  it("returns empty for 0 or 1 cast members", () => {
    expect(castPairs([])).toEqual([]);
    expect(castPairs(makeCast("Alice"))).toEqual([]);
  });

  it("returns 1 pair for 2 members", () => {
    const pairs = castPairs(makeCast("Alice", "Bob"));
    expect(pairs).toEqual([{ a: 0, b: 1, labelA: "Alice", labelB: "Bob" }]);
  });

  it("returns 3 pairs for 3 members", () => {
    const pairs = castPairs(makeCast("Alice", "Bob", "Carol"));
    expect(pairs).toHaveLength(3);
    expect(pairs.map((p) => `${p.labelA}-${p.labelB}`)).toEqual([
      "Alice-Bob",
      "Alice-Carol",
      "Bob-Carol",
    ]);
  });

  it("uses fallback labels for null names", () => {
    const cast: CastSelection[] = [
      { archetype_id: "a", name: null, role: "cast" },
      { archetype_id: "b", name: "Bob", role: "cast" },
    ];
    const pairs = castPairs(cast);
    expect(pairs[0].labelA).toBe("Character 1");
    expect(pairs[0].labelB).toBe("Bob");
  });
});

// ---------------------------------------------------------------------------
// phaseStatus
// ---------------------------------------------------------------------------

describe("phaseStatus", () => {
  const emptyState = freshDebugState(1);

  describe("LLM tab", () => {
    it("returns processing when checking", () => {
      expect(phaseStatus("LLM", emptyState, null, true)).toBe("processing");
    });

    it("returns pending when no status", () => {
      expect(phaseStatus("LLM", emptyState, null, false)).toBe("pending");
    });

    it("returns complete when reachable", () => {
      const llm: LlmStatus = {
        reachable: true, endpoint: "http://localhost:11434",
        model: "llama3", provider: "Ollama", available_models: [], error: null, latency_ms: 50,
      };
      expect(phaseStatus("LLM", emptyState, llm, false)).toBe("complete");
    });

    it("returns error when unreachable", () => {
      const llm: LlmStatus = {
        reachable: false, endpoint: "http://localhost:11434",
        model: "llama3", provider: "Ollama", available_models: [], error: "refused", latency_ms: 0,
      };
      expect(phaseStatus("LLM", emptyState, llm, false)).toBe("error");
    });
  });

  describe("Events tab (compound status)", () => {
    it("returns pending when both pending", () => {
      expect(phaseStatus("Events", emptyState, null, false)).toBe("pending");
    });

    it("returns error if either is error", () => {
      const state = { ...emptyState, phases: { events: "complete" as const, decomposition: "error" as const } };
      expect(phaseStatus("Events", state, null, false)).toBe("error");
    });

    it("returns processing if either is processing", () => {
      const state = { ...emptyState, phases: { events: "complete" as const, decomposition: "processing" as const } };
      expect(phaseStatus("Events", state, null, false)).toBe("processing");
    });

    it("returns complete only when both complete", () => {
      const state = { ...emptyState, phases: { events: "complete" as const, decomposition: "complete" as const } };
      expect(phaseStatus("Events", state, null, false)).toBe("complete");
    });

    it("returns processing when one complete and other pending", () => {
      const state = { ...emptyState, phases: { events: "complete" as const } };
      expect(phaseStatus("Events", state, null, false)).toBe("processing");
    });
  });

  describe("simple tabs", () => {
    it("returns phase status from phases map", () => {
      const state = { ...emptyState, phases: { prediction: "complete" as const } };
      expect(phaseStatus("ML Predictions", state, null, false)).toBe("complete");
    });

    it("defaults to pending for unknown phase", () => {
      expect(phaseStatus("Narrator", emptyState, null, false)).toBe("pending");
    });
  });
});

// ---------------------------------------------------------------------------
// applyDebugEvent
// ---------------------------------------------------------------------------

describe("applyDebugEvent", () => {
  it("resets state when turn changes", () => {
    const state = freshDebugState(1);
    state.phases["prediction"] = "complete";

    const event: PhaseStartedEvent = { type: "phase_started", turn: 2, phase: "prediction" };
    const next = applyDebugEvent(state, event);

    expect(next.turn).toBe(2);
    expect(next.phases["prediction"]).toBe("processing");
    // Old state fields should be null (fresh)
    expect(next.prediction).toBeNull();
  });

  it("preserves state for same turn", () => {
    let state = freshDebugState(1);
    state.phases["prediction"] = "complete";

    const event: PhaseStartedEvent = { type: "phase_started", turn: 1, phase: "context" };
    const next = applyDebugEvent(state, event);

    expect(next.phases["prediction"]).toBe("complete");
    expect(next.phases["context"]).toBe("processing");
  });

  it("handles event_decomposed with error", () => {
    const state = freshDebugState(1);
    const event: EventDecomposedEvent = {
      type: "event_decomposed", turn: 1,
      decomposition: null, raw_llm_json: null,
      timing_ms: 100, model: "qwen", error: "timeout",
    };
    const next = applyDebugEvent(state, event);
    expect(next.phases["decomposition"]).toBe("error");
    expect(next.decomposition?.error).toBe("timeout");
  });

  it("handles event_decomposed without error", () => {
    const state = freshDebugState(1);
    const event: EventDecomposedEvent = {
      type: "event_decomposed", turn: 1,
      decomposition: { events: [], entities: [] }, raw_llm_json: null,
      timing_ms: 100, model: "qwen", error: null,
    };
    const next = applyDebugEvent(state, event);
    expect(next.phases["decomposition"]).toBe("complete");
  });

  it("handles error event", () => {
    const state = freshDebugState(1);
    const event: DebugEvent = {
      type: "error", turn: 1, phase: "narrator", message: "LLM timeout",
    };
    const next = applyDebugEvent(state, event);
    expect(next.phases["narrator"]).toBe("error");
    expect(next.error?.message).toBe("LLM timeout");
  });

  it("handles intent_synthesized event", () => {
    const state = freshDebugState(1);
    const event: DebugEvent = {
      type: "intent_synthesized", turn: 1,
      intent_statements: "**Arthur** should respond reluctantly.",
      timing_ms: 2300,
    };
    const next = applyDebugEvent(state, event);
    expect(next.phases["intent_synthesis"]).toBe("complete");
    expect(next.intent_synthesis?.intent_statements).toBe("**Arthur** should respond reluctantly.");
  });
});
