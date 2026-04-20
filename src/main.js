// ── Water Intake Tracker — Main App Logic ────────────────────
const { invoke } = window.__TAURI__.core;
const { listen } = window.__TAURI__.event;

// ── State ────────────────────────────────────────────────────
let settings = { daily_target: 2000, reminder_interval: 60 };
let reminderTimer = null;

// ── DOM References ───────────────────────────────────────────
const $ = (sel) => document.querySelector(sel);
const $$ = (sel) => document.querySelectorAll(sel);

// ── Initialize ───────────────────────────────────────────────
window.addEventListener("DOMContentLoaded", async () => {
  setCurrentDate();
  await loadSettings();
  await refreshData();
  setupEventListeners();
  setupReminder();

  // Listen for tray quick-add events
  await listen("refresh-data", async () => {
    await refreshData();
  });
});

// ── Date Display ─────────────────────────────────────────────
function setCurrentDate() {
  const now = new Date();
  const formatted = now.toLocaleDateString("en-US", {
    weekday: "long",
    month: "short",
    day: "numeric",
  });
  $("#current-date").textContent = formatted;
}

// ── Event Listeners ──────────────────────────────────────────
function setupEventListeners() {
  // Quick add buttons
  $$(".add-btn[data-amount]").forEach((btn) => {
    btn.addEventListener("click", async () => {
      const amount = parseInt(btn.dataset.amount);
      await addWater(amount);
      pulseButton(btn);
    });
  });

  // Custom amount add
  $("#custom-add-btn").addEventListener("click", async () => {
    const input = $("#custom-amount");
    const amount = parseInt(input.value);
    if (amount && amount > 0 && amount <= 2000) {
      await addWater(amount);
      input.value = "";
    }
  });

  $("#custom-amount").addEventListener("keydown", async (e) => {
    if (e.key === "Enter") {
      const input = e.target;
      const amount = parseInt(input.value);
      if (amount && amount > 0 && amount <= 2000) {
        await addWater(amount);
        input.value = "";
      }
    }
  });

  // Settings toggle
  $("#settings-toggle").addEventListener("click", () => {
    $("#settings-panel").classList.toggle("active");
  });

  // Save settings
  $("#save-settings").addEventListener("click", async () => {
    const target = parseInt($("#target-input").value);
    const interval = parseInt($("#reminder-input").value);
    if (target >= 500 && target <= 5000 && interval >= 15 && interval <= 180) {
      try {
        settings = await invoke("update_settings", {
          dailyTarget: target,
          reminderInterval: interval,
        });
        $("#settings-panel").classList.remove("active");
        setupReminder();
        await refreshData();
        showToast("Settings saved ✨");
      } catch (err) {
        console.error("Failed to save settings:", err);
      }
    }
  });

  // Undo last entry
  $("#undo-btn").addEventListener("click", async () => {
    try {
      const data = await invoke("remove_last_entry");
      updateUI(data);
      await refreshWeeklyChart();
      showToast("Entry removed ↩");
    } catch (err) {
      console.error("Failed to undo:", err);
    }
  });
}

// ── Core Actions ─────────────────────────────────────────────
async function addWater(amount) {
  try {
    const data = await invoke("add_water", { amount });
    updateUI(data);
    await refreshWeeklyChart();
    triggerRipple();

    if (data.total >= data.target && data.total - amount < data.target) {
      showToast("🎉 Daily target reached!");
    }
  } catch (err) {
    console.error("Failed to add water:", err);
  }
}

async function refreshData() {
  try {
    const data = await invoke("get_today_data");
    updateUI(data);
    await refreshWeeklyChart();
  } catch (err) {
    console.error("Failed to load data:", err);
  }
}

async function refreshWeeklyChart() {
  try {
    const weekData = await invoke("get_weekly_data");
    renderWeeklyChart(weekData);
  } catch (err) {
    console.error("Failed to load weekly data:", err);
  }
}

async function loadSettings() {
  try {
    settings = await invoke("get_settings");
    $("#target-input").value = settings.daily_target;
    $("#reminder-input").value = settings.reminder_interval;
  } catch (err) {
    console.error("Failed to load settings:", err);
  }
}

// ── UI Updates ───────────────────────────────────────────────
function updateUI(data) {
  const progress = Math.min(data.total / data.target, 1);
  const circumference = 2 * Math.PI * 75; // radius = 75
  const offset = circumference * (1 - progress);
  const ringFill = $("#ring-fill");
  const progressRing = $(".progress-ring");
  const currentAmount = $("#current-amount");

  // Update progress ring
  ringFill.style.strokeDashoffset = offset;

  // Toggle completed state
  const isCompleted = progress >= 1;
  ringFill.classList.toggle("completed", isCompleted);
  progressRing.classList.toggle("completed", isCompleted);
  currentAmount.classList.toggle("completed", isCompleted);

  // Update text
  currentAmount.textContent = data.total.toLocaleString();
  $("#target-display").textContent = data.target.toLocaleString();
  $("#percentage").textContent = Math.round(progress * 100) + "%";

  // Render log
  renderLog(data.entries);
}

function renderLog(entries) {
  const logList = $("#log-list");

  if (!entries || entries.length === 0) {
    logList.innerHTML =
      '<div class="empty-state">No entries yet. Start drinking! 💧</div>';
    return;
  }

  // Show newest first
  const html = [...entries]
    .reverse()
    .map(
      (entry, i) => `
      <div class="log-entry" style="animation-delay: ${i * 0.04}s">
        <span class="log-time">${entry.time}</span>
        <span class="log-amount">+${entry.amount} ml</span>
      </div>
    `
    )
    .join("");

  logList.innerHTML = html;
}

function renderWeeklyChart(weekData) {
  const chart = $("#weekly-chart");
  const maxVal = Math.max(
    ...weekData.map((d) => d.total),
    weekData[0]?.target || 2000
  );
  const todayIdx = weekData.length - 1;

  const html = weekData
    .map((day, i) => {
      const pct = maxVal > 0 ? Math.max((day.total / maxVal) * 100, 3) : 3;
      const isToday = i === todayIdx;
      const isDone = day.total >= day.target;

      const barClasses = ["bar"];
      if (isToday) barClasses.push("today");
      if (isDone) barClasses.push("completed");

      const labelClasses = ["bar-label"];
      if (isToday) labelClasses.push("today");

      // Format value label
      let valLabel = "";
      if (day.total > 0) {
        valLabel =
          day.total >= 1000
            ? (day.total / 1000).toFixed(1) + "L"
            : day.total + "";
      }

      return `
        <div class="bar-wrapper">
          <span class="bar-value">${valLabel}</span>
          <div class="${barClasses.join(" ")}" style="height: ${pct}%"></div>
          <span class="${labelClasses.join(" ")}">${day.day_name}</span>
        </div>
      `;
    })
    .join("");

  chart.innerHTML = html;
}

// ── Reminder ─────────────────────────────────────────────────
function setupReminder() {
  if (reminderTimer) clearInterval(reminderTimer);

  const intervalMs = settings.reminder_interval * 60 * 1000;
  reminderTimer = setInterval(async () => {
    try {
      await invoke("send_reminder");
    } catch (err) {
      console.error("Reminder failed:", err);
    }
  }, intervalMs);
}

// ── Visual Effects ───────────────────────────────────────────
function triggerRipple() {
  const container = $(".progress-ring-container");
  const ripple = document.createElement("div");
  ripple.className = "ripple";
  container.appendChild(ripple);
  ripple.addEventListener("animationend", () => ripple.remove());
}

function pulseButton(btn) {
  btn.style.transform = "scale(0.92)";
  setTimeout(() => {
    btn.style.transform = "";
  }, 150);
}

function showToast(message) {
  const toast = $("#toast");
  toast.textContent = message;
  toast.classList.add("show");

  setTimeout(() => {
    toast.classList.remove("show");
  }, 2200);
}
