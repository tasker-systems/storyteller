import { invoke } from "@tauri-apps/api/core";
import type { SceneInfo, TurnResult, LogEntry } from "./types";

export async function startScene(): Promise<SceneInfo> {
  return invoke<SceneInfo>("start_scene");
}

export async function submitInput(text: string): Promise<TurnResult> {
  return invoke<TurnResult>("submit_input", { input: text });
}

export async function getSessionLog(): Promise<LogEntry[]> {
  return invoke<LogEntry[]>("get_session_log");
}
