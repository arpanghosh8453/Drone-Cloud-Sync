import { Download, FileText, Settings2, Cloud, CloudOff } from "lucide-react";
import type { Page, Session } from "../types";
import { Brand } from "./Brand";
import { Button } from "./ui/button";
export function Sidebar({
  page,
  navigate,
  activeDownloads,
  session,
  retry,
}: {
  page: Page;
  navigate: (p: Page) => void;
  activeDownloads: number;
  session: Session;
  retry: () => void;
}) {
  return (
    <aside className="sidebar">
      <Brand />
      <nav aria-label="Primary navigation">
        {(
          [
            { id: "flights", icon: FileText, label: "Flight logs" },
            { id: "downloads", icon: Download, label: "Downloads" },
            { id: "settings", icon: Settings2, label: "Settings" },
          ] as const
        ).map(({ id, icon: Icon, label }) => (
          <button
            key={id}
            className={`nav-item ${page === id ? "active" : ""}`}
            aria-current={page === id ? "page" : undefined}
            onClick={() => navigate(id)}
          >
            <Icon size={18} strokeWidth={1.7} />
            <span>{label}</span>
            {id === "downloads" && activeDownloads > 0 && (
              <span className="nav-count">{activeDownloads}</span>
            )}
          </button>
        ))}
      </nav>
      <div className="sidebar-bottom">
        <div className="rail-connection">
          {session.state === "Connected" ? (
            <Cloud size={16} />
          ) : (
            <CloudOff size={16} />
          )}
          <span>
            {session.state === "Connected"
              ? "DJI account connected"
              : "Offline"}
          </span>
        </div>
        <p className="rail-email" title={session.email || ""}>
          {session.email}
        </p>
        <Button
          variant="ghost"
          size="sm"
          onClick={
            session.state === "Connected" ? () => navigate("settings") : retry
          }
        >
          {session.state === "Connected"
            ? "Account settings"
            : "Retry connection"}
        </Button>
      </div>
    </aside>
  );
}
