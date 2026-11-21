import { useState } from "react";
import { Download, FolderOpen, RotateCcw, Trash2, X } from "lucide-react";
import type {
  Download as DownloadType,
  SyncProgress as SyncStatus,
} from "../types";
import { terminal } from "../lib/utils";
import { Button } from "../components/ui/button";
import { DownloadItem } from "../components/DownloadItem";
import { DownloadProgress } from "../components/DownloadProgress";
import { SyncProgress } from "../components/SyncProgress";
export function DownloadsPage({
  downloads,
  sync,
  cancel,
  cancelAll,
  retry,
  clear,
  openFolder,
  cancelSync,
  connected,
  folder,
}: {
  downloads: DownloadType[];
  sync: SyncStatus | null;
  cancel: (id: string) => void;
  cancelAll: () => void;
  retry: () => void;
  clear: () => void;
  openFolder: () => void;
  cancelSync: () => void;
  connected: boolean;
  folder?: string;
}) {
  const [tab, setTab] = useState("All");
  const active = downloads.filter((d) => !terminal(d));
  const tabs = ["All", "Active", "Queued", "Completed", "Failed"];
  const visible = downloads.filter(
    (d) =>
      tab === "All" ||
      (tab === "Active"
        ? !terminal(d) && d.state !== "Queued"
        : tab === "Failed"
          ? d.state === "Failed" || d.state === "Cancelled"
          : d.state === tab),
  );
  const batch = active.length
    ? downloads.filter((d) => active.some((a) => a.batchId === d.batchId))
    : [];
  return (
    <>
      <div className="page-heading">
        <div>
          <span className="eyebrow">TRANSFER CENTER</span>
          <h1>Downloads</h1>
          <p>Your files, from the first byte to the final verification.</p>
        </div>
        <Button variant="outline" disabled={!folder} onClick={openFolder}>
          <FolderOpen size={16} />
          Open output folder
        </Button>
      </div>
      <SyncProgress status={sync} cancel={cancelSync} />
      {batch.length > 0 && <DownloadProgress downloads={batch} />}
      <div className="downloads-toolbar">
        <div className="tabs" aria-label="Filter downloads">
          {tabs.map((t) => (
            <button
              aria-pressed={tab === t}
              className={t === tab ? "active" : ""}
              key={t}
              onClick={() => setTab(t)}
            >
              {t}
              {t === "Active" && active.length > 0 && (
                <span>{active.length}</span>
              )}
            </button>
          ))}
        </div>
        <div className="download-actions">
          <Button
            variant="ghost"
            size="sm"
            disabled={active.length === 0}
            onClick={cancelAll}
          >
            <X size={14} />
            Cancel all
          </Button>
          <Button
            variant="ghost"
            size="sm"
            disabled={
              !connected ||
              !folder ||
              !downloads.some(
                (d) => d.state === "Failed" || d.state === "Cancelled",
              )
            }
            onClick={retry}
          >
            <RotateCcw size={14} />
            Retry failed
          </Button>
          <Button
            variant="ghost"
            size="icon"
            disabled={
              active.length > 0 ||
              !downloads.some((d) => d.state === "Completed")
            }
            aria-label="Clear completed history"
            title="Clear completed history"
            onClick={clear}
          >
            <Trash2 size={15} />
          </Button>
        </div>
      </div>
      <div className="download-list">
        {visible.length ? (
          visible.map((d) => (
            <DownloadItem key={d.id} download={d} cancel={cancel} />
          ))
        ) : (
          <div className="empty-table">
            <Download size={30} />
            <h3>
              {tab === "All"
                ? "A home for your downloads"
                : `No ${tab.toLowerCase()} downloads`}
            </h3>
            <p>
              Select flight records to download, or sync your entire library.
            </p>
          </div>
        )}
      </div>
    </>
  );
}
