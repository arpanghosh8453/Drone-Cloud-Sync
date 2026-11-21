export type SessionState =
  | "Checking"
  | "Connected"
  | "Disconnected"
  | "Expired"
  | "Offline"
  | "Authenticating";
export interface Session {
  state: SessionState;
  email: string | null;
  accountId: string | null;
  message: string | null;
}
export interface Captcha {
  challengeId: string;
  imageDataUrl: string;
}
export interface FlightRecord {
  id: string;
  remoteRecordId: string;
  originalFilename: string;
  flightTimestamp: string | null;
  sortTimestamp: number;
  aircraftName: string | null;
  aircraftSerial: string | null;
  duration: unknown;
  distance: unknown;
  maxAltitude: unknown;
  serverMd5: string | null;
  serverHashKind: "unknown" | "content" | "payload";
  rawMetadata: unknown;
  status: string;
  localPath: string | null;
  localMd5: string | null;
  localSha256: string | null;
}
export type DownloadState =
  | "Queued"
  | "Connecting"
  | "Downloading"
  | "Extracting"
  | "Verifying"
  | "Completed"
  | "Failed"
  | "Cancelled"
  | "Retrying"
  | "Paused";
export interface Download {
  id: string;
  batchId: string;
  recordId: string;
  syncRunId: string | null;
  filename: string;
  state: DownloadState;
  downloadedBytes: number;
  totalBytes: number | null;
  percentage: number | null;
  speed: number;
  etaSeconds: number | null;
  error: string | null;
  attempt: number;
}
export interface SyncProgress {
  id: string;
  stage: string;
  totalRecords: number;
  alreadyPresent: number;
  downloaded: number;
  failed: number;
  missing: number;
  scanned: number;
  error: string | null;
}
export interface InventoryProgress {
  page: number;
  records: number;
  state: string;
  error: string | null;
}
export interface ScanSummary {
  scanned: number;
  present: number;
  missing: number;
  ambiguous: number;
  duplicateContents: number;
  errors: string[];
}
export interface FlightDetails {
  record: FlightRecord;
  history: Download[];
}
export interface SyncHistory extends SyncProgress {
  status: string;
  startedAt: string;
  finishedAt: string | null;
}
export interface AppError {
  kind: string;
  message: string;
}
export type Page = "flights" | "downloads" | "settings";
