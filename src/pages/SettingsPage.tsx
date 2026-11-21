import {
  FolderOpen,
  FolderSearch,
  LogOut,
  Monitor,
  Moon,
  Sun,
  ShieldCheck,
} from "lucide-react";
import type { Session, SyncHistory } from "../types";
import { Button } from "../components/ui/button";
import { Badge } from "../components/ui/badge";
export function SettingsPage({
  settings,
  session,
  busy,
  chooseFolder,
  scan,
  openFolder,
  update,
  logout,
  history,
}: {
  settings: Record<string, string>;
  session: Session;
  busy: boolean;
  chooseFolder: () => void;
  scan: () => void;
  openFolder: () => void;
  update: (key: string, value: string) => void;
  logout: () => void;
  history: SyncHistory[];
}) {
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>Settings</h1>
          <p>Your folder, appearance, and download preferences.</p>
        </div>
      </div>
      <div className="settings-stack">
        <section className="settings-card">
          <div className="settings-section-title">
            <FolderOpen size={19} />
            <div>
              <h2>Output folder</h2>
              <p>Flight logs are saved directly in this folder as TXT files.</p>
            </div>
          </div>
          <div className="folder-path">
            {settings.outputFolder || "No folder selected"}
          </div>
          <div className="settings-buttons">
            <Button
              variant="outline"
              size="sm"
              disabled={busy}
              onClick={chooseFolder}
            >
              {settings.outputFolder ? "Change folder" : "Browse"}
            </Button>
            <Button
              variant="outline"
              size="sm"
              disabled={!settings.outputFolder}
              onClick={openFolder}
            >
              <FolderOpen size={14} />
              Open folder
            </Button>
            <Button
              variant="ghost"
              size="sm"
              disabled={busy || !settings.outputFolder}
              onClick={scan}
            >
              <FolderSearch size={14} />
              Rescan folder
            </Button>
          </div>
          <p className="hint">
            Renamed logs are recognized by their contents, so existing copies
            can be kept.
          </p>
        </section>
        <section className="settings-card">
          <h2>Appearance</h2>
          <p className="muted">Use light, dark, or your system appearance.</p>
          <div className="theme-options">
            {(
              [
                { value: "light", label: "Light", icon: Sun },
                { value: "dark", label: "Dark", icon: Moon },
                { value: "system", label: "System", icon: Monitor },
              ] as const
            ).map((t) => (
              <button
                key={t.value}
                aria-pressed={(settings.theme || "system") === t.value}
                className={
                  (settings.theme || "system") === t.value ? "selected" : ""
                }
                onClick={() => update("theme", t.value)}
              >
                <t.icon size={18} />
                {t.label}
              </button>
            ))}
          </div>
        </section>
        <section className="settings-card">
          <h2>Transfers</h2>
          <div className="setting-row">
            <div>
              <label htmlFor="concurrency">Concurrent downloads</label>
              <p>Keep a small queue for a reliable connection.</p>
            </div>
            <select
              id="concurrency"
              disabled={busy}
              value={settings.downloadConcurrency || "2"}
              onChange={(e) => update("downloadConcurrency", e.target.value)}
            >
              {[1, 2, 3, 4].map((n) => (
                <option key={n} value={n}>
                  {n} {n === 1 ? "download" : "downloads"}
                </option>
              ))}
            </select>
          </div>
          <div className="setting-row">
            <div>
              <label htmlFor="timeout">Network timeout</label>
              <p>Applies after restarting the application.</p>
            </div>
            <select
              id="timeout"
              value={settings.networkTimeout || "30"}
              onChange={(e) => update("networkTimeout", e.target.value)}
            >
              {[15, 30, 60, 120, 300].map((n) => (
                <option key={n} value={n}>
                  {n} seconds
                </option>
              ))}
            </select>
          </div>
        </section>
        <section className="settings-card">
          <div className="settings-section-title">
            <ShieldCheck size={19} />
            <div>
              <h2>Cloud account</h2>
              <p>{session.email || "No account connected"}</p>
            </div>
            <Badge tone={session.state === "Connected" ? "success" : "warning"}>
              {session.state}
            </Badge>
          </div>
          <div className="setting-row">
            <p>Your password is never saved.</p>
            <Button variant="outline" size="sm" onClick={logout}>
              <LogOut size={14} />
              Sign out
            </Button>
          </div>
        </section>
        <section className="settings-card">
          <h2>Recent synchronization</h2>
          {history.length === 0 ? (
            <p className="muted">No synchronization history yet.</p>
          ) : (
            history.slice(0, 8).map((h) => (
              <div key={h.id} className="sync-history-row">
                <div>
                  <strong>{h.status}</strong>
                  <small>{h.startedAt} UTC</small>
                </div>
                <span>
                  {h.downloaded} downloaded · {h.failed} failed
                </span>
              </div>
            ))
          )}
        </section>
      </div>
    </>
  );
}
