export interface Lesson { id: number; title: string; content: string; level: number }
export interface Progress { id: number; lesson_id: number; wpm: number; accuracy: number; completed_at: string }
export interface ProgressInput { username: string; lesson_id: number; wpm: number; accuracy: number }
export interface LinuxTargetInfo { os: string; managed: boolean; prefix: string | null; reason: string }
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
    linuxInstallInfo: () => invoke<LinuxTargetInfo>('linux_install_info'),
    installLinuxUpdate: (expectedVersion: string) =>
      invoke<void>('install_linux_update', { expectedVersion }),
  };
}
