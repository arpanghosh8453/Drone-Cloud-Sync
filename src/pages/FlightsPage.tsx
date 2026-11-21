import { ArrowRight, FolderOpen, RefreshCw, RotateCw, X } from "lucide-react";
import type {
  FlightRecord,
  InventoryProgress,
  SyncProgress as SyncStatus,
} from "../types";
import { syncActive } from "../lib/utils";
import { Button } from "../components/ui/button";
import { FlightTable } from "../components/FlightTable";
import { SyncProgress } from "../components/SyncProgress";
export function FlightsPage({
  records,
  connected,
  folder,
  inventory,
  sync,
  refresh,
  startSync,
  cancelSync,
  cancelInventory,
  openDetails,
  download,
  chooseFolder,
  preferences,
  savePreferences,
  activeRecordId,
}: {
  records: FlightRecord[];
  connected: boolean;
  folder?: string;
  inventory: InventoryProgress | null;
  sync: SyncStatus | null;
  refresh: () => void;
  startSync: () => void;
  cancelSync: () => void;
  cancelInventory: () => void;
  openDetails: (r: FlightRecord) => void;
  download: (ids: string[]) => void;
  chooseFolder: () => void;
  preferences?: string;
  savePreferences: (p: string) => void;
  activeRecordId?: string;
}) {
  const fetching = inventory?.state === "Fetching";
  const syncing = syncActive(sync);
  const verified = records.filter(
    (record) => record.status === "Downloaded",
  ).length;
  return (
    <>
      <div className="page-heading">
        <div>
          <h1>Flight logs</h1>
          <p>Your DJI cloud records, in your own folder.</p>
        </div>
        <div className="heading-actions">
          <Button
            variant="outline"
            disabled={!connected || fetching || syncing}
            onClick={refresh}
          >
            <RefreshCw size={16} className={fetching ? "spin" : ""} />
            Refresh
          </Button>
          <Button
            disabled={!connected || syncing || fetching || !folder}
            onClick={startSync}
          >
            <RotateCw size={16} />
            Sync
          </Button>
        </div>
      </div>
      <div
        className="library-summary"
        aria-label="Cloud and local flight records"
      >
        <span>
          Cloud records <strong>{records.length}</strong>
        </span>
        <ArrowRight size={22} aria-hidden="true" />
        <span>
          Local verified <strong>{verified}</strong>
        </span>
        <span className="summary-note">
          {folder
            ? `${records.length - verified} not downloaded`
            : "Choose a folder to save your logs"}
        </span>
      </div>
      {!folder && (
        <div className="folder-banner">
          <FolderOpen size={20} />
          <div>
            <strong>Choose where to save your logs</strong>
            <p>
              Choose a local folder to enable downloads and synchronization.
            </p>
          </div>
          <Button variant="outline" size="sm" onClick={chooseFolder}>
            Choose folder
          </Button>
        </div>
      )}
      <SyncProgress status={sync} cancel={cancelSync} />
      {fetching && (
        <div className="inventory-progress" role="status">
          <RefreshCw size={14} className="spin" />
          <span>
            Fetching flight records — page {inventory.page}, {inventory.records}{" "}
            records found
          </span>
          <Button size="sm" variant="ghost" onClick={cancelInventory}>
            <X size={13} />
            Cancel
          </Button>
        </div>
      )}
      {inventory?.state === "Partial failure" && (
        <div className="notice warning" role="alert">
          Inventory is incomplete. Retrieved records have been saved.{" "}
          {inventory.error}
        </div>
      )}
      <FlightTable
        records={records}
        onOpen={openDetails}
        onDownload={download}
        canDownload={connected && !!folder && !syncing}
        preferences={preferences}
        savePreferences={savePreferences}
        activeRecordId={activeRecordId}
      />
    </>
  );
}
