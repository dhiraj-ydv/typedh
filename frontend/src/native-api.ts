export interface Lesson { id: number; title: string; content: string; level: number }
export interface Progress { id: number; lesson_id: number; wpm: number; accuracy: number; completed_at: string }
export interface ProgressInput { username: string; lesson_id: number; wpm: number; accuracy: number }
type Invoke = <T>(command: string, args?: Record<string, unknown>) => Promise<T>;

export function createNativeApi(invoke: Invoke) {
  return {
    getLessons: () => invoke<Lesson[]>('get_lessons'),
    getProgress: (username: string, before?: number) =>
      invoke<Progress[]>('get_progress', { username, limit: 100, before: before ?? null }),
    saveProgress: (input: ProgressInput) => invoke<void>('save_progress', { input }),
    stopApplication: () => invoke<void>('stop_application'),
    restartApplication: () => invoke<void>('restart_application'),
    appVersion: () => invoke<string>('app_version'),
  };
}
