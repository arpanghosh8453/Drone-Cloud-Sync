import {
  CheckCircle2,
  CircleAlert,
  CircleMinus,
  CirclePause,
  RefreshCw,
  X,
} from "lucide-react";
import { syncActive } from "../lib/utils";
import type { SyncProgress as SyncStatus } from "../types";
import { Button } from "./ui/button";
import { ProgressBar } from "./DownloadProgress";
export function SyncProgress({
  status: s,
  cancel,
}: {
  status: SyncStatus | null;
  cancel: () => void;
}) {
  if (!s?.id) return null;
  const active = syncActive(s);
  const paused = s.stage.startsWith("Paused");
  const failed = s.stage === "Failed";
  const complete = s.stage === "Completed" && s.failed === 0;
  const Icon = paused
    ? CirclePause
    : active
      ? RefreshCw
      : complete
        ? CheckCircle2
        : failed || s.failed > 0
          ? CircleAlert
          : CircleMinus;
  const tone = failed
    ? "sync-danger"
    : paused || (!active && !complete)
      ? "sync-warning"
      : "";
  const percentage = s.missing
    ? ((s.downloaded + s.failed) / s.missing) * 100
    : 0;
  return (
    <div className={`sync-panel ${tone}`} role={failed ? "alert" : "status"}>
      <div className="sync-title">
        <Icon size={17} className={active && !paused ? "spin" : undefined} />
        <strong>
          {active
            ? s.stage
            : complete
              ? "Your flight logs are in sync"
              : `Synchronization ${s.stage.toLowerCase()}`}
        </strong>
        {active && (
          <Button variant="ghost" size="sm" onClick={cancel}>
            <X size={14} />
            Cancel sync
          </Button>
        )}
      </div>
      {active && (
        <ProgressBar
          label="Synchronization progress"
          value={percentage}
          indeterminate={
            !paused &&
            !s.stage.includes("Downloading") &&
            !s.stage.includes("Verifying")
          }
        />
      )}
      <p>
        {s.totalRecords > 0
          ? `${s.totalRecords} cloud records · ${s.alreadyPresent} already present · ${s.downloaded} downloaded · ${s.failed} failed`
          : s.scanned
            ? `${s.scanned} local files scanned`
            : "Checking your cloud records and local folder."}
      </p>
      {s.error && <p className="item-error">{s.error}</p>}
    </div>
  );
}
