import {
  Download,
  FileText,
  Settings2,
  FolderOpen,
  HardDrive,
} from "lucide-react";
import type { Page } from "../types";
import { Brand } from "./Brand";
import { Button } from "./ui/button";
export function Sidebar({
  page,
  navigate,
  activeDownloads,
  folder,
  openFolder,
}: {
  page: Page;
  navigate: (p: Page) => void;
  activeDownloads: number;
  folder?: string;
  openFolder: () => void;
}) {
  return (
    <aside className="sidebar">
      <Brand />
      <div className="workspace-label">WORKSPACE</div>
      <nav aria-label="Primary navigation">
        {(
          [
            { id: "flights", icon: FileText, label: "Flight Logs" },
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
        <div className="folder-card">
          <HardDrive size={18} />
          <strong>Your local workspace</strong>
          <p title={folder}>{folder || "Choose a folder to get started."}</p>
          <Button
            variant="outline"
            size="sm"
            onClick={folder ? openFolder : () => navigate("settings")}
          >
            <FolderOpen size={14} />
            {folder ? "Open folder" : "Choose folder"}
          </Button>
        </div>
      </div>
    </aside>
  );
}
