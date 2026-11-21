import { useState } from "react";
import {
  Cloud,
  CloudOff,
  LoaderCircle,
  RefreshCw,
  X,
  ArrowRight,
  WifiOff,
} from "lucide-react";
import { useApp } from "./hooks/useApp";
import { desktop } from "./lib/api";
import { syncActive, terminal } from "./lib/utils";
import type { FlightRecord, Page } from "./types";
import { Brand } from "./components/Brand";
import { Sidebar } from "./components/Sidebar";
import { FlightDetails } from "./components/FlightDetails";
import { Button } from "./components/ui/button";
import { LoginPage } from "./pages/LoginPage";
import { FlightsPage } from "./pages/FlightsPage";
import { DownloadsPage } from "./pages/DownloadsPage";
import { SettingsPage } from "./pages/SettingsPage";
export default function App() {
  const app = useApp();
  const [page, setPage] = useState<Page>("flights");
  const [detail, setDetail] = useState<FlightRecord | null>(null);
  const [cached, setCached] = useState(false);
  const connected = app.session.state === "Connected";
  const busy =
    app.operations > 0 ||
    app.downloads.some((d) => !terminal(d)) ||
    syncActive(app.sync);
  async function download(ids: string[], force = false) {
    const result = await app.download(ids, force);
    if (result?.length) {
      setDetail(null);
      setPage("downloads");
    }
  }
  const notice = app.notice && (
    <div
      className={`global-notice notice ${app.notice.danger ? "danger" : ""}`}
      role={app.notice.danger ? "alert" : "status"}
    >
      <span>{app.notice.message}</span>
      <Button
        variant="ghost"
        size="icon"
        aria-label="Dismiss message"
        onClick={() => app.setNotice(null)}
      >
        <X size={15} />
      </Button>
    </div>
  );
  if (!desktop)
    return (
      <div className="startup-screen">
        <Brand />
        <div className="startup-card">
          <Cloud size={32} />
          <h1>Your desktop workspace is ready</h1>
          <p>
            Run the native application to connect your account and access local
            files.
          </p>
          <code>npm run tauri dev</code>
          <p className="hint">
            The browser preview has no access to account credentials or your
            filesystem.
          </p>
        </div>
      </div>
    );
  if (app.session.state === "Checking")
    return (
      <div className="startup-screen">
        <Brand />
        <div className="startup-card" role="status">
          <LoaderCircle size={28} className="spin" />
          <h2>Checking cloud session…</h2>
          <p>Validating your saved session securely.</p>
        </div>
      </div>
    );
  if (["Disconnected", "Expired", "Authenticating"].includes(app.session.state))
    return (
      <>
        {notice}
        <LoginPage
          session={app.session}
          onConnected={(s) => {
            app.setSession(s);
            setCached(false);
          }}
        />
      </>
    );
  if (app.session.state === "Offline" && !cached)
    return (
      <div className="startup-screen">
        <Brand />
        {notice}
        <div className="startup-card">
          <WifiOff size={30} />
          <h2>Cloud is temporarily unreachable</h2>
          <p>
            Your saved session has been retained. Retry when your connection is
            available, or browse your cached records.
          </p>
          {app.session.message && <p className="hint">{app.session.message}</p>}
          <div className="startup-actions">
            <Button onClick={() => void app.retrySession()}>
              <RefreshCw size={15} />
              Retry connection
            </Button>
            <Button variant="outline" onClick={() => setCached(true)}>
              View cached records
              <ArrowRight size={15} />
            </Button>
          </div>
          <Button variant="ghost" onClick={() => void app.logout()}>
            Sign out
          </Button>
        </div>
      </div>
    );
  return (
    <div className="app-layout">
      <Sidebar
        page={page}
        navigate={setPage}
        activeDownloads={app.downloads.filter((d) => !terminal(d)).length}
        folder={app.settings.outputFolder}
        openFolder={() => void app.openFolder()}
      />
      <div className="main-layout">
        <header className="topbar">
          <span className="topbar-title">
            drone-cloud-sync <span>/</span>{" "}
            {page === "flights"
              ? "Flight Logs"
              : page === "downloads"
                ? "Downloads"
                : "Settings"}
          </span>
          <div className="connection">
            <span
              className={`connection-status ${connected ? "connected" : ""}`}
            >
              {connected ? <Cloud size={14} /> : <CloudOff size={14} />}
              {app.session.state}
            </span>
            <span className="account-email" title={app.session.email || ""}>
              {app.session.email}
            </span>
            <Button
              variant="ghost"
              size="icon"
              aria-label={
                connected ? "Refresh cloud inventory" : "Retry cloud connection"
              }
              title={
                connected ? "Refresh cloud inventory" : "Retry cloud connection"
              }
              disabled={
                app.inventory?.state === "Fetching" || syncActive(app.sync)
              }
              onClick={() =>
                void (connected ? app.refresh() : app.retrySession())
              }
            >
              <RefreshCw size={15} />
            </Button>
          </div>
        </header>
        {notice}
        {!connected && (
          <div className="offline-banner">
            <WifiOff size={15} />
            Offline — showing cached records. Downloads will be available after
            reconnecting.
            <Button
              variant="ghost"
              size="sm"
              onClick={() => void app.retrySession()}
            >
              Retry
            </Button>
          </div>
        )}
        <main className="page-content">
          {page === "flights" && (
            <FlightsPage
              records={app.records}
              connected={connected}
              folder={app.settings.outputFolder}
              inventory={app.inventory}
              sync={app.sync}
              refresh={() => void app.refresh()}
              startSync={() => {
                void app.call("start_sync").then((id) => {
                  if (id) setPage("downloads");
                });
              }}
              cancelSync={() => void app.call("cancel_sync")}
              cancelInventory={() => void app.call("cancel_inventory")}
              openDetails={setDetail}
              download={(ids) => void download(ids)}
              chooseFolder={() => void app.chooseFolder()}
              preferences={app.settings.tablePreferences}
              savePreferences={app.savePreferences}
            />
          )}
          {page === "downloads" && (
            <DownloadsPage
              downloads={app.downloads}
              sync={app.sync}
              connected={connected}
              folder={app.settings.outputFolder}
              cancel={(id) =>
                void app.call("cancel_download", { downloadId: id })
              }
              cancelAll={() => void app.call("cancel_all_downloads")}
              cancelSync={() => void app.call("cancel_sync")}
              retry={() => void app.call("retry_failed_downloads")}
              clear={() =>
                void app.run(async () => {
                  await app.call("clear_completed_history");
                  await app.loadHistory();
                })
              }
              openFolder={() => void app.openFolder()}
            />
          )}
          {page === "settings" && (
            <SettingsPage
              settings={app.settings}
              session={app.session}
              busy={!!busy}
              chooseFolder={() => void app.chooseFolder()}
              scan={() => void app.scan()}
              openFolder={() => void app.openFolder()}
              update={(k, v) => void app.update(k, v)}
              logout={() => void app.logout()}
              history={app.history}
            />
          )}
        </main>
      </div>
      <FlightDetails
        record={detail}
        close={() => setDetail(null)}
        canDownload={
          connected && !!app.settings.outputFolder && !syncActive(app.sync)
        }
        download={(id, force) => void download([id], force)}
        openFolder={(id) => void app.openFolder(id)}
      />
    </div>
  );
}
