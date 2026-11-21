import { CloudSyncIcon } from "./CloudSyncIcon";
export function Brand({ compact = false }: { compact?: boolean }) {
  return (
    <div className="brand">
      <div className="brand-icon">
        <CloudSyncIcon size={23} />
      </div>
      {!compact && <span>drone-cloud-sync</span>}
    </div>
  );
}
