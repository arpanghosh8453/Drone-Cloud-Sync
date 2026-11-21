import { useEffect, useState } from "react";
import * as Dialog from "@radix-ui/react-dialog";
import {
  Copy,
  Download,
  FolderOpen,
  X,
  FileText,
  LoaderCircle,
} from "lucide-react";
import type { FlightDetails as Details, FlightRecord } from "../types";
import { command } from "../lib/api";
import { date, errorMessage, value } from "../lib/utils";
import { Button } from "./ui/button";
import { Badge } from "./ui/badge";
export function FlightDetails({
  record,
  close,
  download,
  openFolder,
  canDownload,
}: {
  record: FlightRecord | null;
  close: () => void;
  download: (id: string, force?: boolean) => void;
  openFolder: (id: string) => void;
  canDownload: boolean;
}) {
  const [details, setDetails] = useState<Details | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [copied, setCopied] = useState<string | null>(null);
  useEffect(() => {
    let current = true;
    setDetails(null);
    setError(null);
    if (record)
      void command<Details>("get_flight_details", { recordId: record.id })
        .then((d) => {
          if (current) setDetails(d);
        })
        .catch((e) => {
          if (current) setError(errorMessage(e));
        });
    return () => {
      current = false;
    };
  }, [record]);
  async function copy(text: string, label: string) {
    try {
      await navigator.clipboard.writeText(text);
      setCopied(label);
    } catch {
      setError(
        "Could not copy to the clipboard. Select and copy the hash below.",
      );
    }
  }
  const r = details?.record || record;
  return (
    <Dialog.Root
      open={!!record}
      onOpenChange={(v) => {
        if (!v) close();
      }}
    >
      <Dialog.Portal>
        <Dialog.Overlay className="dialog-overlay" />
        <Dialog.Content className="detail-panel">
          <div className="detail-header">
            <div>
              <span className="eyebrow">FLIGHT RECORD</span>
              <Dialog.Title>Flight details</Dialog.Title>
            </div>
            <Dialog.Close asChild>
              <Button
                variant="ghost"
                size="icon"
                aria-label="Close flight details"
              >
                <X size={19} />
              </Button>
            </Dialog.Close>
          </div>
          <Dialog.Description className="sr-only">
            Flight metadata, local file hashes, and download history.
          </Dialog.Description>
          {r && (
            <div className="detail-body">
              <div className="detail-file">
                <FileText size={24} />
                <strong>{r.originalFilename}</strong>
                <Badge tone={r.status === "Downloaded" ? "success" : "neutral"}>
                  {r.status}
                </Badge>
              </div>
              <dl className="metadata-grid">
                <dt>Flight date</dt>
                <dd>{date(r.flightTimestamp, r.sortTimestamp)}</dd>
                <dt>Aircraft</dt>
                <dd>{r.aircraftName || "—"}</dd>
                <dt>Serial number</dt>
                <dd>{r.aircraftSerial || "—"}</dd>
                <dt>Duration</dt>
                <dd>{value(r.duration)}</dd>
                <dt>Distance</dt>
                <dd>{value(r.distance)}</dd>
                <dt>Maximum altitude</dt>
                <dd>{value(r.maxAltitude)}</dd>
              </dl>
              <p className="hint">
                Numeric values retain the cloud’s original units.
              </p>
              <h3>File integrity</h3>
              {(
                [
                  ["Server MD5", r.serverMd5],
                  ["Local MD5", r.localMd5],
                  ["Local SHA-256", r.localSha256],
                ] as const
              ).map(([label, hash]) => (
                <div className="hash-field" key={label}>
                  <div>
                    <span>{label}</span>
                    {hash && (
                      <Button
                        size="icon"
                        variant="ghost"
                        aria-label={`Copy ${label}`}
                        title={`Copy ${label}`}
                        onClick={() => void copy(hash, label)}
                      >
                        <Copy size={13} />
                      </Button>
                    )}
                  </div>
                  <code>{hash || "Not available"}</code>
                </div>
              ))}
              <p className="hint">
                Server hash scope:{" "}
                {r.serverHashKind === "content"
                  ? "verified uncompressed TXT content"
                  : r.serverHashKind === "payload"
                    ? "compressed download payload"
                    : "unconfirmed; not used for matching"}
                .
              </p>
              {copied && (
                <p className="hint" role="status">
                  {copied} copied.
                </p>
              )}
              <h3>Local file</h3>
              <code className="local-path">
                {r.localPath || "This record has no downloaded file."}
              </code>
              <div className="detail-actions">
                <Button
                  disabled={!canDownload}
                  onClick={() => download(r.id, r.status === "Downloaded")}
                >
                  <Download size={15} />
                  {r.status === "Downloaded"
                    ? "Download another copy"
                    : "Download log"}
                </Button>
                {r.localPath && (
                  <Button variant="outline" onClick={() => openFolder(r.id)}>
                    <FolderOpen size={15} />
                    Open folder
                  </Button>
                )}
              </div>
              <h3>Download history</h3>
              {!details && !error && (
                <LoaderCircle size={18} className="spin" />
              )}
              {details?.history.length === 0 && (
                <p className="muted">No download attempts yet.</p>
              )}
              {details?.history.map((d) => (
                <div className="detail-attempt" key={d.id}>
                  <Badge
                    tone={
                      d.state === "Completed"
                        ? "success"
                        : d.state === "Failed"
                          ? "danger"
                          : "neutral"
                    }
                  >
                    {d.state}
                  </Badge>
                  <span>{d.downloadedBytes.toLocaleString()} bytes</span>
                  {d.error && <small>{d.error}</small>}
                </div>
              ))}
              {error && (
                <div className="notice danger" role="alert">
                  {error}
                </div>
              )}
              <h3>Original cloud metadata</h3>
              <pre className="raw-metadata">
                {JSON.stringify(r.rawMetadata, null, 2)}
              </pre>
            </div>
          )}
        </Dialog.Content>
      </Dialog.Portal>
    </Dialog.Root>
  );
}
