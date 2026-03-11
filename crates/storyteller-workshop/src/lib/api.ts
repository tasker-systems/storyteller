import { invoke } from "@tauri-apps/api/core";
import type {
  SceneInfo,
  TurnResult,
  LogEntry,
  LlmStatus,
  GenreSummary,
  GenreOptions,
  SceneSelections,
  SessionSummary,
  ResumeResult,
} from "./types";

export async function checkLlm(): Promise<LlmStatus> {
  return invoke<LlmStatus>("check_llm");
}

export async function startScene(): Promise<SceneInfo> {
  return invoke<SceneInfo>("start_scene");
}

export async function submitInput(text: string): Promise<TurnResult> {
  return invoke<TurnResult>("submit_input", { input: text });
}

export async function getSessionLog(): Promise<LogEntry[]> {
  return invoke<LogEntry[]>("get_session_log");
}

// ---------------------------------------------------------------------------
// Scene template commands
// ---------------------------------------------------------------------------

export async function loadCatalog(): Promise<GenreSummary[]> {
  return invoke<GenreSummary[]>("load_catalog");
}

export async function getGenreOptions(
  genreId: string,
  selectedArchetypes: string[] = [],
): Promise<GenreOptions> {
  return invoke<GenreOptions>("get_genre_options", {
    genreId,
    selectedArchetypes,
  });
}

export async function composeScene(
  selections: SceneSelections,
): Promise<SceneInfo> {
  return invoke<SceneInfo>("compose_scene", { selections });
}

export async function listSessions(): Promise<SessionSummary[]> {
  return invoke<SessionSummary[]>("list_sessions");
}

export async function resumeSession(sessionId: string): Promise<ResumeResult> {
  return invoke<ResumeResult>("resume_session", { sessionId });
}
