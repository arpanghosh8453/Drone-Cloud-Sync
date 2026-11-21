import { FolderOpen, RefreshCw, RotateCw, X } from "lucide-react";
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
}) {
  const fetching = inventory?.state === "Fetching";
  const syncing = syncActive(sync);
  return (
    <>
      <div className="page-heading">
        <div>
          <span className="eyebrow">YOUR FLIGHT LIBRARY</span>
          <h1>
            Flight Logs <span className="heading-count">{records.length}</span>
          </h1>
          <p>Every flight, organized. Ready when you need it.</p>
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
      {!folder && (
        <div className="folder-banner">
          <FolderOpen size={20} />
          <div>
            <strong>Give your flight logs a home</strong>
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
      />
    </>
  );
}
