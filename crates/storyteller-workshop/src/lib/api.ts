import { invoke } from "@tauri-apps/api/core";
import type {
  HealthReport,
  SceneInfo,
  TurnResult,
  GenreSummary,
  ProfileSummary,
  ArchetypeSummary,
  DynamicSummary,
  SettingSummary,
  SessionInfo,
  ResumeResult,
  SceneSelections,
} from "./generated";

export async function checkHealth(): Promise<HealthReport> {
  return invoke<HealthReport>("check_health");
}

export async function loadCatalog(): Promise<GenreSummary[]> {
  return invoke<GenreSummary[]>("load_catalog");
}

export async function getProfilesForGenre(genreId: string): Promise<ProfileSummary[]> {
  return invoke<ProfileSummary[]>("get_profiles_for_genre", { genreId });
}

export async function getArchetypesForGenre(genreId: string): Promise<ArchetypeSummary[]> {
  return invoke<ArchetypeSummary[]>("get_archetypes_for_genre", { genreId });
}

export async function getDynamicsForGenre(
  genreId: string,
  selectedArchetypeIds: string[] = [],
): Promise<DynamicSummary[]> {
  return invoke<DynamicSummary[]>("get_dynamics_for_genre", { genreId, selectedArchetypeIds });
}

export async function getNamesForGenre(genreId: string): Promise<string[]> {
  return invoke<string[]>("get_names_for_genre", { genreId });
}

export async function getSettingsForGenre(genreId: string): Promise<SettingSummary[]> {
  return invoke<SettingSummary[]>("get_settings_for_genre", { genreId });
}

export async function composeScene(
  selections: SceneSelections,
): Promise<SceneInfo> {
  return invoke<SceneInfo>("compose_scene", { selections });
}

export async function submitInput(sessionId: string, input: string): Promise<TurnResult> {
  return invoke<TurnResult>("submit_input", { sessionId, input });
}

export async function listSessions(): Promise<SessionInfo[]> {
  return invoke<SessionInfo[]>("list_sessions");
}

export async function resumeSession(sessionId: string): Promise<ResumeResult> {
  return invoke<ResumeResult>("resume_session", { sessionId });
}

export async function getSceneState(sessionId: string): Promise<unknown> {
  return invoke<unknown>("get_scene_state", { sessionId });
}

export async function getPredictionHistory(sessionId: string): Promise<unknown> {
  return invoke<unknown>("get_prediction_history", { sessionId });
}
