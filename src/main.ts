import { invoke } from "@tauri-apps/api/core";
import { listen } from "@tauri-apps/api/event";

interface ProcessInfo {
  name: string;
  pid: number;
  status: string;
}

interface AppState {
  cliProcesses: ProcessInfo[];
  sittingMinutes: number;
  isUserPresent: boolean;
  lastActivity: Date;
}

const state: AppState = {
  cliProcesses: [],
  sittingMinutes: 0,
  isUserPresent: true,
  lastActivity: new Date(),
};

function formatTime(minutes: number): string {
  const hrs = Math.floor(minutes / 60);
  const mins = minutes % 60;
  if (hrs > 0) {
    return `${hrs}小时 ${mins}分钟`;
  }
  return `${mins}分钟`;
}

function render() {
  const app = document.getElementById("app");
  if (!app) return;

  app.innerHTML = `
    <div class="header">
      <h1>Focus Guard</h1>
      <p>开发者健康助手</p>
    </div>

    <div class="status-card">
      <h2>CLI 监控状态</h2>
      <div id="cli-status">
        ${
          state.cliProcesses.length === 0
            ? '<div class="status-item"><span>未检测到CLI进程</span></div>'
            : state.cliProcesses
                .map(
                  (p) => `
              <div class="status-item">
                <div class="status-indicator">
                  <span class="status-dot ${p.status === "waiting" ? "waiting" : "active"}"></span>
                  <span>${p.name}</span>
                </div>
                <span>${p.status === "waiting" ? "等待输入" : "运行中"}</span>
              </div>
            `
                )
                .join("")
        }
      </div>
      <button class="btn" id="refresh-btn" style="margin-top: 12px; width: 100%;">
        刷新状态
      </button>
    </div>

    <div class="status-card">
      <h2>久坐提醒</h2>
      <div class="timer">${formatTime(state.sittingMinutes)}</div>
      <div class="status-item">
        <span>用户状态</span>
        <div class="status-indicator">
          <span class="status-dot ${state.isUserPresent ? "active" : ""}"></span>
          <span>${state.isUserPresent ? "在屏幕前" : "已离开"}</span>
        </div>
      </div>
    </div>

    <div class="status-card">
      <h2>快捷操作</h2>
      <div style="display: flex; gap: 8px; flex-wrap: wrap;">
        <button class="btn" id="test-notify">测试通知</button>
        <button class="btn" id="reset-timer">重置计时</button>
      </div>
    </div>
  `;

  document.getElementById("refresh-btn")?.addEventListener("click", refreshCliStatus);
  document.getElementById("test-notify")?.addEventListener("click", testNotification);
  document.getElementById("reset-timer")?.addEventListener("click", resetTimer);
}

async function refreshCliStatus() {
  try {
    const processes = await invoke<ProcessInfo[]>("get_cli_processes");
    state.cliProcesses = processes;
    render();
  } catch (e) {
    console.error("Failed to get CLI processes:", e);
  }
}

async function testNotification() {
  try {
    await invoke("send_notification", {
      title: "Focus Guard",
      body: "CLI需要你的注意！请查看终端。",
    });
  } catch (e) {
    console.error("Failed to send notification:", e);
  }
}

function resetTimer() {
  state.sittingMinutes = 0;
  render();
}

async function init() {
  render();

  // 监听来自Rust后端的事件
  await listen<{ minutes: number }>("sitting-time-update", (event) => {
    state.sittingMinutes = event.payload.minutes;
    render();
  });

  await listen<{ present: boolean }>("user-presence-update", (event) => {
    state.isUserPresent = event.payload.present;
    render();
  });

  await listen<ProcessInfo[]>("cli-status-update", (event) => {
    state.cliProcesses = event.payload;
    render();
  });

  // 初始刷新
  refreshCliStatus();

  // 定时刷新CLI状态
  setInterval(refreshCliStatus, 5000);

  // 本地久坐计时（后续会由Rust后端接管）
  setInterval(() => {
    if (state.isUserPresent) {
      state.sittingMinutes++;
      render();
    }
  }, 60000);
}

init();
