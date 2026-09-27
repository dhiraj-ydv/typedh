import { invoke } from '@tauri-apps/api/core';
import { createNativeApi } from './native-api';

export const { getLessons, getProgress, saveProgress, stopApplication, restartApplication, appVersion } = createNativeApi(invoke);
