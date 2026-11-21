import { useEffect, useRef, useState } from "react";
import { Copy, Download, FolderOpen, X, LoaderCircle } from "lucide-react";
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
  const returnFocus = useRef<HTMLElement | null>(null);
  const closeButton = useRef<HTMLButtonElement | null>(null);
  useEffect(() => {
    if (record) {
      returnFocus.current = document.activeElement as HTMLElement | null;
      closeButton.current?.focus({ preventScroll: true });
    } else if (returnFocus.current) {
      const target = returnFocus.current.isConnected
        ? returnFocus.current
        : document.querySelector<HTMLElement>("nav [aria-current='page']");
      target?.focus({ preventScroll: true });
      returnFocus.current = null;
    }
  }, [record?.id]);
  useEffect(() => {
    let current = true;
    setDetails(null);
    setError(null);
    setCopied(null);
    if (record)
      void command<Details>("get_flight_details", { recordId: record.id })
        .then((result) => {
          if (current) setDetails(result);
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
  if (!r) return null;
  return (
    <aside
      className="detail-panel"
      aria-labelledby="flight-details-title"
      onKeyDown={(event) => {
        if (event.key === "Escape") {
          event.preventDefault();
          close();
        }
      }}
    >
      <div className="detail-header">
        <h2 id="flight-details-title">Flight details</h2>
        <Button
          ref={closeButton}
          variant="ghost"
          size="icon"
          aria-label="Close flight details"
          onClick={close}
        >
          <X size={18} />
        </Button>
      </div>
      <div className="detail-body">
        <div className="detail-file">
          <h3>{date(r.flightTimestamp, r.sortTimestamp)}</h3>
          <p>{r.aircraftName || "Aircraft unavailable"}</p>
          <Badge
            tone={
              r.status === "Downloaded"
                ? "success"
                : r.status === "Failed"
                  ? "danger"
                  : "neutral"
            }
          >
            {r.status}
          </Badge>
        </div>
        <dl className="metadata-grid">
          <dt>Original log</dt>
          <dd className="detail-filename">{r.originalFilename}</dd>
        </dl>
        <h3>Local file</h3>
        <p className="local-path">
          {r.localPath || "This log has not been downloaded."}
        </p>
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
        {error && (
          <div className="notice danger" role="alert">
            {error}
          </div>
        )}
        <h3>Download history</h3>
        {!details && !error && (
          <span className="detail-loading" role="status">
            <LoaderCircle size={16} className="spin" />
            Loading history…
          </span>
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
        <details className="technical-details">
          <summary>Flight information</summary>
          <dl className="metadata-grid">
            <dt>Serial number</dt>
            <dd>{r.aircraftSerial || "—"}</dd>
            <dt>Duration</dt>
            <dd>{value(r.duration)}</dd>
            <dt>Distance</dt>
            <dd>{value(r.distance)}</dd>
            <dt>Maximum altitude</dt>
            <dd>{value(r.maxAltitude)}</dd>
          </dl>
          <p className="hint">Values use the cloud’s original units.</p>
        </details>
        <details className="technical-details">
          <summary>File integrity</summary>
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
                    <Copy size={14} />
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
        </details>
        <details className="technical-details">
          <summary>Original cloud metadata</summary>
          <pre className="raw-metadata">
            {JSON.stringify(r.rawMetadata, null, 2)}
          </pre>
        </details>
      </div>
    </aside>
  );
}
