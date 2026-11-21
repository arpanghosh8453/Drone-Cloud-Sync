import { useCallback, useEffect, useMemo, useRef, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { command, desktop } from "../lib/api";
import { errorMessage, syncActive, terminal } from "../lib/utils";
import type {
  Download,
  FlightRecord,
  InventoryProgress,
  ScanSummary,
  Session,
  SyncHistory,
  SyncProgress,
} from "../types";
const initial: Session = {
  state: "Checking",
  email: null,
  accountId: null,
  message: null,
};
export function useApp() {
  const [session, setSession] = useState(initial);
  const [settings, setSettings] = useState<Record<string, string>>({});
  const [records, setRecords] = useState<FlightRecord[]>([]);
  const [downloads, setDownloads] = useState<Download[]>([]);
  const [sync, setSync] = useState<SyncProgress | null>(null);
  const [inventory, setInventory] = useState<InventoryProgress | null>(null);
  const [history, setHistory] = useState<SyncHistory[]>([]);
  const [notice, setNotice] = useState<{
    message: string;
    danger: boolean;
  } | null>(null);
  const [operations, setOperations] = useState(0);
  const initialized = useRef(false);
  const connectedKey = useRef("");
  const preferenceTimer = useRef<ReturnType<typeof setTimeout> | undefined>(
    undefined,
  );
  const report = useCallback(
    (error: unknown) =>
      setNotice({ message: errorMessage(error), danger: true }),
    [],
  );
  const run = useCallback(
    async <T>(fn: () => Promise<T>): Promise<T | undefined> => {
      setOperations((n) => n + 1);
      try {
        return await fn();
      } catch (e) {
        report(e);
        return undefined;
      } finally {
        setOperations((n) => n - 1);
      }
    },
    [report],
  );
  const loadRecords = useCallback(async () => {
    setRecords(await command<FlightRecord[]>("get_cached_flight_records"));
  }, []);
  const loadHistory = useCallback(async () => {
    const [d, s] = await Promise.all([
      command<Download[]>("get_download_history"),
      command<SyncHistory[]>("get_sync_history"),
    ]);
    setDownloads(d);
    setHistory(s);
  }, []);
  const refresh = useCallback(
    () =>
      run(async () => {
        setInventory({ page: 1, records: 0, state: "Fetching", error: null });
        try {
          await command("fetch_flight_records");
        } finally {
          await loadRecords();
        }
      }),
    [loadRecords, run],
  );
  useEffect(() => {
    if (!desktop || initialized.current) return;
    initialized.current = true;
    let disposed = false;
    const unlisten: (() => void)[] = [];
    async function initialize() {
      const subscriptions: [string, (payload: never) => void][] = [
        ["session:changed", (p: Session) => setSession(p)],
        [
          "inventory:progress",
          (p: InventoryProgress) => {
            setInventory(p);
            if (p.state === "Completed" || p.state === "Partial failure")
              void loadRecords().catch(report);
          },
        ],
        ...[
          "queued",
          "started",
          "progress",
          "extracting",
          "verifying",
          "completed",
          "failed",
          "cancelled",
        ].map(
          (name) =>
            [
              `download:${name}`,
              (p: Download) => {
                setDownloads((ds) => {
                  const existing = ds.find((d) => d.id === p.id);
                  return existing
                    ? ds.map((d) => (d.id === p.id ? p : d))
                    : [p, ...ds];
                });
                if (terminal(p)) void loadRecords().catch(report);
              },
            ] as [string, (payload: never) => void],
        ),
        ...["started", "progress", "completed", "failed", "cancelled"].map(
          (name) =>
            [
              `sync:${name}`,
              (p: SyncProgress) => {
                setSync(p);
                if (!syncActive(p)) {
                  void loadRecords().catch(report);
                  void command<SyncHistory[]>("get_sync_history")
                    .then(setHistory)
                    .catch(report);
                }
              },
            ] as [string, (payload: never) => void],
        ),
      ];
      await Promise.all(
        subscriptions.map(async ([name, handler]) => {
          const off = await listen(name, (e) => {
            if (!disposed) handler(e.payload as never);
          });
          if (disposed) off();
          else unlisten.push(off);
        }),
      );
      if (disposed) return;
      const savedSettings =
        await command<Record<string, string>>("get_settings");
      setSettings(savedSettings);
      if (savedSettings.outputFolderWarning)
        setNotice({ message: savedSettings.outputFolderWarning, danger: true });
      const s = await command<Session>("validate_stored_session");
      if (!disposed) setSession(s);
      if (s.accountId) {
        await loadRecords();
        await loadHistory();
      }
    }
    void initialize().catch((e) => {
      if (!disposed) {
        report(e);
        setSession({
          ...initial,
          state: "Disconnected",
          message: errorMessage(e),
        });
      }
    });
    return () => {
      disposed = true;
      unlisten.forEach((off) => off());
    };
  }, [loadRecords, loadHistory, report]);
  useEffect(() => {
    if (session.state !== "Connected") {
      connectedKey.current = "";
      return;
    }
    if (connectedKey.current === session.accountId) return;
    connectedKey.current = session.accountId || "";
    void run(async () => {
      await loadRecords();
      await loadHistory();
      const s = await command<SyncProgress>("get_sync_status");
      setSync(s);
      if (!syncActive(s)) await refresh();
    });
  }, [
    session.state,
    session.accountId,
    loadRecords,
    loadHistory,
    refresh,
    run,
  ]);
  useEffect(() => {
    const media = window.matchMedia("(prefers-color-scheme: dark)");
    function apply() {
      document.documentElement.classList.toggle(
        "dark",
        settings.theme === "dark" ||
          ((settings.theme || "system") === "system" && media.matches),
      );
    }
    apply();
    media.addEventListener("change", apply);
    return () => media.removeEventListener("change", apply);
  }, [settings.theme]);
  useEffect(
    () => () => {
      if (preferenceTimer.current) clearTimeout(preferenceTimer.current);
    },
    [],
  );
  const update = (key: string, value: string) =>
    run(async () => {
      await command("update_setting", { key, value });
      setSettings((s) => ({ ...s, [key]: value }));
    });
  const savePreferences = (value: string) => {
    if (preferenceTimer.current) clearTimeout(preferenceTimer.current);
    preferenceTimer.current = setTimeout(() => {
      void update("tablePreferences", value);
    }, 450);
  };
  const chooseFolder = () =>
    run(async () => {
      const folder = await command<string | null>("select_output_folder");
      if (folder) {
        setSettings((s) => ({ ...s, outputFolder: folder }));
        await loadRecords();
        setNotice({
          message: "Output folder saved. Rescan to verify existing files.",
          danger: false,
        });
      }
    });
  const scan = () =>
    run(async () => {
      const s = await command<ScanSummary>("scan_output_folder");
      await loadRecords();
      setNotice({
        message: `Scanned ${s.scanned} files: ${s.present} records verified, ${s.missing} missing, ${s.ambiguous} ambiguous, ${s.duplicateContents} duplicate contents.${s.errors.length ? ` ${s.errors.length} files could not be verified.` : ""}`,
        danger: s.errors.length > 0,
      });
    });
  const download = (ids: string[], force = false) =>
    run(async () => {
      const queued = await command<string[]>("download_selected", {
        recordIds: ids,
        force,
      });
      if (!queued.length)
        setNotice({
          message:
            "All selected records are already verified in your output folder.",
          danger: false,
        });
      return queued;
    });
  const openFolder = (recordId?: string) =>
    run(() => command("open_output_folder", { recordId: recordId ?? null }));
  const retrySession = () =>
    run(async () =>
      setSession(await command<Session>("validate_stored_session")),
    );
  const logout = () =>
    run(async () => {
      await command("logout");
      setSession((s) => ({ ...s, state: "Disconnected", message: null }));
      setRecords([]);
      setDownloads([]);
      setSync(null);
      setInventory(null);
    });
  const call = (name: string, args?: Record<string, unknown>) =>
    run(() => command(name, args));
  const visibleRecords = useMemo(
    () =>
      records.map((r) => {
        const job = downloads.find((d) => d.recordId === r.id);
        return job && !terminal(job)
          ? { ...r, status: "Downloading" }
          : job?.state === "Failed" && r.status !== "Downloaded"
            ? { ...r, status: "Failed" }
            : r;
      }),
    [records, downloads],
  );
  return {
    session,
    setSession,
    settings,
    records: visibleRecords,
    downloads,
    sync,
    inventory,
    history,
    notice,
    setNotice,
    operations,
    run,
    loadHistory,
    refresh,
    update,
    savePreferences,
    chooseFolder,
    scan,
    download,
    openFolder,
    retrySession,
    logout,
    call,
  };
}
