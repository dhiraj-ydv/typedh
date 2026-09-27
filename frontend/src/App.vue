<script setup lang="ts">
import { ref, onMounted, onUnmounted } from 'vue';
import Keyboard from './components/Keyboard.vue';
import TypingArea from './components/TypingArea.vue';
import { getLessons, getProgress, saveProgress, restartApplication, stopApplication, appVersion } from './api';
import { check } from '@tauri-apps/plugin-updater';
import { checkForUpdates, friendlyUpdateError, progressFraction, isDesktop, type UpdateCheck } from './updates';

interface Lesson { id: number; title: string; content: string; level: number }
interface Progress { id: number; lesson_id: number; wpm: number; accuracy: number; completed_at: string }

const lessons = ref<Lesson[]>([]);
const currentLesson = ref<Lesson | null>(null);
const activeKey = ref('');
const targetKey = ref('');
const username = ref('User1');
const showStats = ref(false);
const finalStats = ref({ wpm: 0, accuracy: 0 });
const history = ref<Progress[]>([]);
const loading = ref(true);
const errorMessage = ref('');
const saveMessage = ref('');
const saving = ref(false);
const historyLoading = ref(false);
const hasMoreHistory = ref(false);
const isDrawerOpen = ref(false);
const currentView = ref('lesson');
const customText = ref('');
const MAX_CUSTOM_LENGTH = 10000;

// In-app updates (signed releases from GitHub Releases).
type UpdatePhase = 'idle' | 'checking' | 'downloading' | 'installing' | 'installed' | 'error';
const updateCheck = ref<UpdateCheck | null>(null);
const updatePhase = ref<UpdatePhase>('idle');
const updateProgress = ref<number | null>(null);
const updateStatusText = ref('');
const updateError = ref('');

async function openUpdatesView() {
  setView('updates');
  updateCheck.value = null;
  updatePhase.value = 'idle';
  updateProgress.value = null;
  updateStatusText.value = '';
  updateError.value = '';
  if (!isDesktop()) {
    updatePhase.value = 'error';
    updateError.value = 'Update checks are available in the desktop app.';
    return;
  }
  try {
    updateStatusText.value = 'Reading the installed version…';
    updateCheck.value = { kind: 'up-to-date', currentVersion: await appVersion() };
  } catch (error) {
    updatePhase.value = 'error';
    updateError.value = friendlyUpdateError(error);
  }
}

async function checkUpdates() {
  updatePhase.value = 'checking';
  updateProgress.value = null;
  updateStatusText.value = 'Checking GitHub Releases for a newer signed version…';
  updateError.value = '';
  try {
    updateCheck.value = await checkForUpdates({
      getAppVersion: appVersion,
      checkPlugin: async () => {
        const found = await check();
        if (!found) return null;
        return {
          version: found.version,
          notes: found.body ?? '',
          install: async (onProgress) => {
            let downloaded = 0;
            let total: number | undefined;
            await found.downloadAndInstall((event) => {
              if (event.event === 'Started') {
                downloaded = 0;
                total = event.data.contentLength;
                onProgress(0, total);
              } else if (event.event === 'Progress') {
                downloaded += event.data.chunkLength;
                onProgress(downloaded, total);
              } else if (event.event === 'Finished') {
                onProgress(downloaded, total ?? downloaded);
              }
            });
          },
        };
      },
    });
  } catch (error) {
    updatePhase.value = 'error';
    updateError.value = friendlyUpdateError(error);
    return;
  }
  const result = updateCheck.value;
  if (!result || result.kind === 'up-to-date') {
    updatePhase.value = 'idle';
    updateStatusText.value = 'You are on the latest version.';
  } else if (result.kind === 'unsupported' || result.kind === 'error') {
    updatePhase.value = 'error';
    updateError.value = result.kind === 'unsupported' ? result.reason : result.message;
  } else {
    updatePhase.value = 'idle';
    updateStatusText.value = `Version ${result.version} is available.`;
  }
}

async function installStockUpdate() {
  const result = updateCheck.value;
  if (!result || result.kind !== 'available-stock') return;
  if (!confirm(`Download and install version ${result.version}?`)) return;
  updatePhase.value = 'downloading';
  updateProgress.value = null;
  updateStatusText.value = `Downloading version ${result.version}…`;
  updateError.value = '';
  try {
    await result.install((downloaded, total) => {
      updateProgress.value = progressFraction(downloaded, total);
      if (total) {
        updateStatusText.value = `Downloading version ${result.version}… ${Math.round((updateProgress.value ?? 0) * 100)}%`;
      }
    });
    updatePhase.value = 'installed';
    updateProgress.value = 1;
    updateStatusText.value = `Version ${result.version} is installed.`;
  } catch (error) {
    updatePhase.value = 'error';
    updateError.value = friendlyUpdateError(error);
  }
}

async function restartAfterUpdate() {
  try { await restartApplication(); }
  catch { updateError.value = 'The app could not restart. Close and reopen it to finish updating.'; }
}

function readPreference(key: string) {
  try { return localStorage.getItem(key); } catch { return null; }
}
function writePreference(key: string, value: string) {
  try { localStorage.setItem(key, value); } catch { /* Preferences are optional. */ }
}
const toggleDrawer = () => { isDrawerOpen.value = !isDrawerOpen.value; };
const setView = (view: string) => {
  currentView.value = view;
  isDrawerOpen.value = false;
  showStats.value = false;
};

async function fetchHistory(append = false) {
  if (historyLoading.value) return;
  historyLoading.value = true;
  try {
    const last = append ? history.value.at(-1) : undefined;
    const entries = await getProgress(username.value, last?.id);
    history.value = append ? [...history.value, ...entries] : entries;
    hasMoreHistory.value = entries.length === 100;
  } catch {
    errorMessage.value = 'Progress history could not be loaded. Check free disk space, then retry.';
  } finally {
    historyLoading.value = false;
  }
}

async function loadApp() {
  loading.value = true;
  errorMessage.value = '';
  try {
    username.value = readPreference('username') || 'User1';
    lessons.value = await getLessons();
    if (!lessons.value.length) throw new Error('No lessons available.');
    const savedId = Number(readPreference('currentLessonId'));
    currentLesson.value = lessons.value.find(lesson => lesson.id === savedId) || lessons.value[0]!;
    targetKey.value = currentLesson.value.content[0] || '';
    await fetchHistory();
  } catch {
    errorMessage.value = 'The app could not load your lessons. Check free disk space, then retry or restart the app.';
  } finally {
    loading.value = false;
  }
}

async function handleComplete(stats: { wpm: number; accuracy: number }) {
  if (saving.value || !currentLesson.value) return;
  finalStats.value = stats;
  showStats.value = true;
  saving.value = true;
  saveMessage.value = 'Saving result…';
  const lessonId = currentLesson.value.id;
  try {
    await saveProgress({ username: username.value, lesson_id: lessonId, ...stats });
    saveMessage.value = 'Result saved.';
    await fetchHistory();
  } catch {
    saveMessage.value = 'Save could not be confirmed. Check Progress History before repeating this lesson.';
  } finally {
    saving.value = false;
  }
}

function switchLesson(lesson: Lesson) {
  currentLesson.value = lesson;
  targetKey.value = lesson.content[0] || '';
  setView('lesson');
  if (lesson.id !== 999) writePreference('currentLessonId', lesson.id.toString());
}
function nextLesson() {
  const index = lessons.value.findIndex(lesson => lesson.id === currentLesson.value?.id);
  const next = lessons.value[index + 1];
  if (next) switchLesson(next);
  else { showStats.value = false; setView('history'); }
}
function handleCustomPractice() {
  const content = customText.value.trim().toLowerCase().replace(/\s+/g, ' ');
  if (!content || content.length > MAX_CUSTOM_LENGTH) {
    errorMessage.value = 'Enter between 1 and 10,000 characters for custom practice.';
    return;
  }
  errorMessage.value = '';
  switchLesson({ id: 999, title: 'Custom Practice', content, level: 99 });
}
async function handleFileUpload(event: Event) {
  const input = event.target as HTMLInputElement;
  const file = input.files?.[0];
  if (!file) return;
  if (file.size > 100000) {
    errorMessage.value = 'Choose a text file smaller than 100 KB.';
    input.value = '';
    return;
  }
  try {
    const text = await file.text();
    if (text.length > MAX_CUSTOM_LENGTH) throw new Error('File too long');
    customText.value = text;
    errorMessage.value = '';
  } catch {
    errorMessage.value = 'The file could not be loaded. Use a text file with at most 10,000 characters.';
  }
  input.value = '';
}
async function stopApp() {
  if (!confirm('Close TypeDH?')) return;
  try { await stopApplication(); }
  catch { errorMessage.value = 'The app could not close. Use the window close button.'; }
}
async function restartApp() {
  if (!confirm('Restart TypeDH?')) return;
  try { await restartApplication(); }
  catch { errorMessage.value = 'The app could not restart. Close and reopen it.'; }
}
const keyDown = (event: KeyboardEvent) => { activeKey.value = event.key; };
const keyUp = () => { activeKey.value = ''; };
onMounted(() => {
  void loadApp();
  window.addEventListener('keydown', keyDown);
  window.addEventListener('keyup', keyUp);
  window.addEventListener('blur', keyUp);
});
onUnmounted(() => {
  window.removeEventListener('keydown', keyDown);
  window.removeEventListener('keyup', keyUp);
  window.removeEventListener('blur', keyUp);
});
</script>

<template>
  <div class="app-container">
    <!-- Overlay for Drawer -->
    <div v-if="isDrawerOpen" class="drawer-overlay" @click="toggleDrawer"></div>

    <!-- SIDE DRAWER (Hamburger Menu) -->
    <aside class="drawer" :class="{ open: isDrawerOpen }">
      <div class="drawer-header">
        <h2>Menu</h2>
        <button class="hamburger-btn" @click="toggleDrawer">✕</button>
      </div>

      <div class="drawer-content">
        <nav class="drawer-nav">
          <button class="nav-item" :class="{ active: currentView === 'lesson' }" @click="setView('lesson')">
            📖 Lessons
          </button>
          <button class="nav-item" :class="{ active: currentView === 'custom' }" @click="setView('custom')">
            ✍️ Custom Practice
          </button>
          <button class="nav-item" :class="{ active: currentView === 'history' }" @click="setView('history')">
            📊 Progress History
          </button>
          <button class="nav-item" :class="{ active: currentView === 'updates' }" @click="openUpdatesView">
            🔄 Updates
          </button>
        </nav>

        <div v-if="currentView === 'lesson'" class="drawer-section">
          <h3>Choose Lesson</h3>
          <ul class="lesson-list">
            <li 
              v-for="lesson in lessons" 
              :key="lesson.id"
              :class="{ active: currentLesson?.id === lesson.id }"
              @click="switchLesson(lesson)"
            >
              <span class="level">Lv. {{ lesson.level }}</span>
              <span class="title">{{ lesson.title }}</span>
            </li>
          </ul>
        </div>
      </div>

      <div class="drawer-footer">
        <button class="system-btn restart" @click="restartApp">🔄 Restart App</button>
        <button class="system-btn stop" @click="stopApp">🛑 Stop App</button>
      </div>
    </aside>

    <!-- HEADER -->
    <header class="app-header">
      <div class="header-left">
        <button class="hamburger-btn" @click="toggleDrawer">☰</button>
        <div class="logo">
          <h1>TypeDH</h1>
        </div>
      </div>
      <div class="current-lesson-name">
        {{ currentLesson?.title || 'No Lesson Selected' }}
      </div>
    </header>

    <!-- MAIN VIEW -->
    <p v-if="loading" role="status">Loading your lessons…</p>
    <div v-if="errorMessage" class="error-banner" role="alert">
      <p>{{ errorMessage }}</p>
      <button :disabled="loading || saving" @click="loadApp">Retry loading</button>
    </div>
    <main v-if="!loading" class="main-view">
      
      <!-- Lesson Practice View -->
      <div v-if="currentView === 'lesson' && !showStats" class="typing-section">
        <TypingArea 
          v-if="currentLesson"
          :targetText="currentLesson.content" 
          @complete="handleComplete"
          @targetChange="(key) => targetKey = key"
        />
      </div>

      <!-- Custom Practice Setup -->
      <div v-if="currentView === 'custom'" class="custom-view-content">
        <div class="card full-height-card">
          <h2>Custom Practice</h2>
          <p>Paste text or upload a file to begin.</p>
          <textarea 
            v-model="customText" 
            :maxlength="MAX_CUSTOM_LENGTH"
            aria-label="Custom practice text"
            placeholder="Paste your text here..." 
            class="custom-textarea flex-grow"
          ></textarea>
          <div class="upload-actions">
            <label class="file-label">
              📁 Upload .txt
              <input type="file" accept=".txt" @change="handleFileUpload" />
            </label>
            <button @click="handleCustomPractice" :disabled="!customText.trim()">Start Practice</button>
          </div>
        </div>
      </div>

      <!-- History View -->
      <div v-if="currentView === 'history'" class="history-view-content">
        <div class="card">
          <h2>Your History</h2>
          <div class="history-table-wrapper">
            <table class="history-table">
              <thead>
                <tr>
                  <th>Date</th>
                  <th>WPM</th>
                  <th>Accuracy</th>
                </tr>
              </thead>
              <tbody>
                <tr v-for="entry in history" :key="entry.id">
                  <td>{{ new Date(entry.completed_at).toLocaleDateString() }}</td>
                  <td class="wpm-val">{{ entry.wpm }}</td>
                  <td class="acc-val">{{ entry.accuracy }}%</td>
                </tr>
              </tbody>
            </table>
            <p v-if="!history.length && !historyLoading">No completed lessons yet.</p>
            <button v-if="hasMoreHistory" :disabled="historyLoading" @click="fetchHistory(true)">Load older results</button>
          </div>
        </div>
      </div>

      <!-- Updates View -->
      <div v-if="currentView === 'updates'" class="history-view-content">
        <div class="card">
          <h2>Updates</h2>
          <p v-if="updateCheck && 'currentVersion' in updateCheck && updateCheck.currentVersion">
            Installed version: {{ updateCheck.currentVersion }}
          </p>
          <p v-if="updateStatusText" role="status">{{ updateStatusText }}</p>
          <div v-if="updatePhase === 'downloading' || updatePhase === 'installing'" class="update-progress">
            <progress :value="updateProgress ?? undefined" max="1" aria-label="Update progress">
              {{ updateProgress === null ? 'Working…' : `${Math.round(updateProgress * 100)}%` }}
            </progress>
          </div>
          <p v-if="updateCheck && updateCheck.kind !== 'up-to-date' && 'version' in updateCheck && updateCheck.version">
            Available version: {{ updateCheck.version }}
          </p>
          <p v-if="updateCheck && 'notes' in updateCheck && updateCheck.notes">{{ updateCheck.notes }}</p>
          <p v-if="updateError" class="update-error" role="alert">{{ updateError }}</p>
          <div class="upload-actions">
            <button
              :disabled="updatePhase === 'checking' || updatePhase === 'downloading' || updatePhase === 'installing'"
              @click="checkUpdates"
            >
              Check for updates
            </button>
            <button
              v-if="updateCheck && updateCheck.kind === 'available-stock'"
              :disabled="updatePhase !== 'idle'"
              @click="installStockUpdate"
            >
              Download &amp; install
            </button>
            <button
              v-if="updatePhase === 'installed'"
              @click="restartAfterUpdate"
            >
              Restart now
            </button>
          </div>
          <p class="update-note">
            Updates are signed and verified before installing. On Windows and macOS the app closes while the
            installer finishes — reopen it if it does not restart by itself. On Linux, update with
            sudo pacman -U using the newer typedh .pkg.tar.zst from GitHub Releases. Your lessons and
            progress are kept.
          </p>
        </div>
      </div>

      <!-- Stats Modal (Overlay on Top) -->
      <div v-if="showStats" class="stats-overlay">
        <div class="stats-modal card">
          <div class="success-icon">🎉</div>
          <h2>Well Done!</h2>
          <p role="status">{{ saveMessage }}</p>
          <div class="final-stats-grid">
            <div class="stat-item">
              <label>Speed</label>
              <div class="value">{{ finalStats.wpm }} <small>WPM</small></div>
            </div>
            <div class="stat-item">
              <label>Accuracy</label>
              <div class="value">{{ finalStats.accuracy }}%</div>
            </div>
          </div>
          <div class="modal-actions">
            <button :disabled="saving" @click="nextLesson">Next Lesson</button>
            <button :disabled="saving" class="secondary" @click="showStats = false; setView('lesson')">Practice Again</button>
          </div>
        </div>
      </div>

      <!-- FIXED VISUALIZER SECTION -->
      <div v-if="currentView === 'lesson' && !showStats" class="visualizer-section">
        <Keyboard :activeKey="activeKey" :targetKey="targetKey" />
      </div>

    </main>
  </div>
</template>

<style scoped>
.error-banner { padding: 12px 20px; border: 1px solid var(--error-color); border-radius: 8px; }
.header-left {
  display: flex;
  align-items: center;
  gap: 15px;
}

.app-header h1 {
  font-size: 1.2rem;
  margin: 0;
}

.current-lesson-name {
  font-weight: 600;
  color: var(--text-muted);
  font-size: 0.9rem;
}

/* Side Drawer Styles */
.drawer-nav {
  display: flex;
  flex-direction: column;
  gap: 10px;
  margin-bottom: 30px;
}

.nav-item {
  text-align: left;
  background: transparent;
  padding: 12px;
  width: 100%;
  color: var(--text-muted);
}

.nav-item.active {
  background: rgba(66, 184, 131, 0.1);
  color: var(--primary-color);
}

.drawer-section h3 {
  font-size: 0.75rem;
  text-transform: uppercase;
  color: var(--text-muted);
  margin-bottom: 10px;
}

.lesson-list {
  list-style: none;
  padding: 0;
}

.lesson-list li {
  padding: 10px;
  cursor: pointer;
  border-radius: 6px;
  display: flex;
  flex-direction: column;
  margin-bottom: 5px;
}

.lesson-list li:hover { background: rgba(255,255,255,0.05); }
.lesson-list li.active { border: 1px solid var(--primary-color); }
.lesson-list li .level { font-size: 0.6rem; color: var(--primary-color); font-weight: 800; }
.lesson-list li .title { font-size: 0.9rem; font-weight: 600; }

/* Views Content */
.custom-view-content, .history-view-content {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
  gap: 20px;
}

.custom-textarea {
  width: 100%;
  background: var(--bg-color);
  color: white;
  border: 1px solid rgba(255,255,255,0.1);
  padding: 15px;
  border-radius: 8px;
  margin-bottom: 15px;
  min-height: 200px;
}

.full-height-card {
  flex: 1;
  min-height: 0;
  display: flex;
  flex-direction: column;
}

.flex-grow {
  flex: 1;
}

.upload-actions {
  display: flex;
  justify-content: space-between;
  align-items: center;
}

.file-label {
  cursor: pointer;
  color: var(--primary-color);
  font-weight: 600;
}

.file-label input { display: none; }

/* Stats Overlay */
.stats-overlay {
  position: absolute;
  inset: 0;
  background: rgba(15, 23, 42, 0.8);
  backdrop-filter: blur(10px);
  display: flex;
  align-items: center;
  justify-content: center;
  z-index: 50;
}

.stats-modal {
  text-align: center;
  width: 100%;
  max-width: 400px;
}

.final-stats-grid {
  display: grid;
  grid-template-columns: 1fr 1fr;
  gap: 15px;
  margin: 20px 0;
}

.stat-item {
  background: var(--bg-color);
  padding: 15px;
  border-radius: 10px;
}

.stat-item label { font-size: 0.7rem; color: var(--text-muted); }
.stat-item .value { font-size: 1.5rem; font-weight: 800; }

.modal-actions {
  display: flex;
  flex-direction: column;
  gap: 10px;
}

/* History Table */
.history-table-wrapper {
  max-height: 300px;
  overflow-y: auto;
}

.history-table {
  width: 100%;
  border-collapse: collapse;
}

.history-table th { text-align: left; padding: 10px; color: var(--text-muted); font-size: 0.8rem; }
.history-table td { padding: 10px; border-bottom: 1px solid rgba(255,255,255,0.05); }
.wpm-val { color: var(--primary-color); font-weight: 700; }
.acc-val { color: var(--accent-color); font-weight: 700; }

.drawer-footer {
  margin-top: auto;
  padding: 20px;
  border-top: 1px solid rgba(255,255,255,0.1);
  display: flex;
  flex-direction: column;
  gap: 10px;
}

.update-progress progress {
  width: 100%;
  margin: 10px 0;
}

.update-error {
  color: var(--error-color);
  font-weight: 600;
}

.update-note {
  font-size: 0.8rem;
  color: var(--text-muted);
  margin-top: 15px;
}

.system-btn {
  background: rgba(255,255,255,0.05);
  border: 1px solid rgba(255,255,255,0.1);
  color: var(--text-muted);
  font-size: 0.85rem;
  justify-content: center;
}

.system-btn.restart:hover {
  background: var(--accent-color);
  color: white;
  border-color: var(--accent-color);
}

.system-btn.stop:hover {
  background: var(--error-color);
  color: white;
  border-color: var(--error-color);
}
</style>
