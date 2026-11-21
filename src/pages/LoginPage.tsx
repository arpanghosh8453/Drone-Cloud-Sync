import { CloudDownload, FolderCheck, ShieldCheck } from "lucide-react";
import { CloudSyncIcon } from "../components/CloudSyncIcon";
import { Brand } from "../components/Brand";
import { LoginForm } from "../components/LoginForm";
import type { Session } from "../types";
export function LoginPage({
  session,
  onConnected,
}: {
  session: Session;
  onConnected: (s: Session) => void;
}) {
  return (
    <div className="login-page">
      <header>
        <Brand />
      </header>
      <main className="login-layout">
        <section className="login-intro">
          <span className="eyebrow">YOUR FLIGHTS. YOUR FILES.</span>
          <h1>
            A clear path
            <br />
            from cloud to <em>local.</em>
          </h1>
          <p>
            Bring your flight records home. Browse your flights, download
            original logs, and keep your folder in sync.
          </p>
          <div className="login-benefits">
            <div>
              <CloudDownload />
              <span>
                Original flight logs
                <small>Downloaded directly to your folder.</small>
              </span>
            </div>
            <div>
              <FolderCheck />
              <span>
                Seamless synchronization
                <small>Verified files stay. Missing files download.</small>
              </span>
            </div>
            <div>
              <ShieldCheck />
              <span>
                Private by design
                <small>No analytics. No external database.</small>
              </span>
            </div>
          </div>
        </section>
        <section className="login-card" aria-labelledby="login-heading">
          <div className="small-cloud">
            <CloudSyncIcon size={27} />
          </div>
          <h2 id="login-heading">Connect your DJI cloud account</h2>
          <p className="muted">Sign in to access your flight records.</p>
          <LoginForm session={session} onConnected={onConnected} />
        </section>
      </main>
    </div>
  );
}
