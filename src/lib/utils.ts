import { clsx, type ClassValue } from "clsx";
import { twMerge } from "tailwind-merge";
import type { Download, SyncProgress } from "../types";
export function cn(...inputs: ClassValue[]) {
  return twMerge(clsx(inputs));
}
export function errorMessage(error: unknown): string {
  return typeof error === "object" && error && "message" in error
    ? String(error.message)
    : String(error);
}
export const terminal = (d: Download) =>
  ["Completed", "Failed", "Cancelled"].includes(d.state);
export const syncActive = (s: SyncProgress | null) =>
  !!s?.id &&
  ![
    "Completed",
    "Completed with failures",
    "Cancelled",
    "Failed",
    "Interrupted",
  ].includes(s.stage);
export function bytes(n: number): string {
  if (!Number.isFinite(n) || n <= 0) return "0 B";
  const i = Math.min(Math.floor(Math.log(n) / Math.log(1024)), 3);
  return `${(n / 1024 ** i).toFixed(i ? 1 : 0)} ${["B", "KB", "MB", "GB"][i]}`;
}
export function eta(seconds: number | null): string {
  if (seconds == null || !Number.isFinite(seconds)) return "";
  if (seconds < 60) return `${Math.ceil(seconds)}s left`;
  return `${Math.ceil(seconds / 60)}m left`;
}
export function value(v: unknown): string {
  return v === null || v === undefined || v === ""
    ? "—"
    : typeof v === "object"
      ? JSON.stringify(v)
      : String(v);
}
export function date(timestamp: string | null, fallback: number): string {
  if (fallback > 0 && fallback < 1e12) {
    const d = new Date(fallback * 1000);
    if (!Number.isNaN(d.getTime()))
      return new Intl.DateTimeFormat(undefined, {
        dateStyle: "medium",
        timeStyle: "short",
      }).format(d);
  }
  return timestamp || "Date unavailable";
}
export function aggregate(downloads: Download[]) {
  const completed = downloads.filter((d) => d.state === "Completed").length;
  const failed = downloads.filter((d) => d.state === "Failed").length;
  const cancelled = downloads.filter((d) => d.state === "Cancelled").length;
  const active = downloads.filter((d) =>
    ["Connecting", "Downloading", "Extracting", "Verifying"].includes(d.state),
  ).length;
  return {
    total: downloads.length,
    completed,
    failed,
    cancelled,
    active,
    pending: downloads.filter((d) =>
      ["Queued", "Paused", "Retrying"].includes(d.state),
    ).length,
    transferred: downloads.reduce((n, d) => n + d.downloadedBytes, 0),
    speed: downloads
      .filter((d) => d.state === "Downloading")
      .reduce((n, d) => n + d.speed, 0),
    percent: downloads.length ? (completed / downloads.length) * 100 : 0,
  };
}
