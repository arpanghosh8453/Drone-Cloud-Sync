import { aggregate, bytes } from "../lib/utils";
import type { Download } from "../types";
export function ProgressBar({
  value,
  label,
  indeterminate = false,
}: {
  value?: number | null;
  label: string;
  indeterminate?: boolean;
}) {
  return (
    <div
      className={`progress-track ${indeterminate ? "indeterminate" : ""}`}
      role="progressbar"
      aria-label={label}
      aria-valuemin={0}
      aria-valuemax={100}
      aria-valuenow={
        indeterminate || value == null ? undefined : Math.round(value)
      }
    >
      <div
        className="progress-fill"
        style={
          indeterminate
            ? undefined
            : { width: `${Math.max(0, Math.min(value ?? 0, 100))}%` }
        }
      />
    </div>
  );
}
export function DownloadProgress({ downloads }: { downloads: Download[] }) {
  const a = aggregate(downloads);
  if (!a.total) return null;
  return (
    <div className="aggregate-progress">
      <div className="aggregate-title">
        <strong>Downloading flight logs</strong>
        <span>
          {a.completed} / {a.total} completed{" "}
          <span className="muted">— {Math.round(a.percent)}%</span>
        </span>
      </div>
      <ProgressBar value={a.percent} label="Overall completed downloads" />
      <div className="aggregate-meta">
        <span>
          {a.active} active · {a.pending} pending · {a.failed} failed
          {a.cancelled > 0 && ` · ${a.cancelled} cancelled`}
        </span>
        <span>
          {bytes(a.transferred)} transferred
          {a.speed > 0 && ` · ${bytes(a.speed)}/s`}
        </span>
      </div>
    </div>
  );
}
