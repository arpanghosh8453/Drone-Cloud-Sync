import { Check, FileText, LoaderCircle, X, CircleAlert } from "lucide-react";
import type { Download } from "../types";
import { bytes, eta, terminal } from "../lib/utils";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
import { ProgressBar } from "./DownloadProgress";
export function DownloadItem({
  download: d,
  cancel,
}: {
  download: Download;
  cancel: (id: string) => void;
}) {
  const active = !terminal(d);
  const waiting = d.state === "Queued" || d.state === "Paused";
  return (
    <article className="download-item">
      <div className={`file-icon ${d.state === "Completed" ? "success" : ""}`}>
        {d.state === "Completed" ? (
          <Check size={20} />
        ) : d.state === "Failed" ? (
          <CircleAlert size={20} />
        ) : (
          <FileText size={20} />
        )}
      </div>
      <div className="download-content">
        <div className="download-item-title">
          <strong title={d.filename}>{d.filename}</strong>
          <Badge
            tone={
              d.state === "Completed"
                ? "success"
                : d.state === "Failed"
                  ? "danger"
                  : d.state === "Paused"
                    ? "warning"
                    : "neutral"
            }
          >
            {d.state === "Downloading" && (
              <LoaderCircle size={11} className="spin" />
            )}
            {d.state}
          </Badge>
        </div>
        {active && (
          <ProgressBar
            value={d.percentage}
            indeterminate={
              !waiting &&
              (d.totalBytes === null ||
                ["Connecting", "Extracting", "Verifying", "Retrying"].includes(
                  d.state,
                ))
            }
            label={`${d.filename} progress`}
          />
        )}
        <div className="download-item-meta">
          <span>
            {bytes(d.downloadedBytes)}
            {d.totalBytes !== null && ` / ${bytes(d.totalBytes)}`}
            {d.percentage !== null &&
              active &&
              ` · ${Math.floor(d.percentage)}%`}
          </span>
          {active && (
            <span>
              {d.speed > 0 && `${bytes(d.speed)}/s`}
              {d.etaSeconds !== null && ` · ${eta(d.etaSeconds)}`}
            </span>
          )}
        </div>
        {d.error && <p className="item-error">{d.error}</p>}
      </div>
      {active && (
        <Button
          variant="ghost"
          size="icon"
          title="Cancel download"
          aria-label={`Cancel ${d.filename}`}
          onClick={() => cancel(d.id)}
        >
          <X size={16} />
        </Button>
      )}
    </article>
  );
}
