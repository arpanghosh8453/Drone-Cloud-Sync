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
      <aside className="login-sidebar">
        <Brand />
        <p>
          Browse your flights.
          <br />
          Keep your original logs.
        </p>
      </aside>
      <main className="login-layout">
        <section className="login-card" aria-labelledby="login-heading">
          <h1 id="login-heading">Connect your DJI account</h1>
          <p className="muted">
            Sign in to browse and download your flight logs.
          </p>
          <LoginForm session={session} onConnected={onConnected} />
        </section>
        <footer className="login-footer">
          Independently developed. Not affiliated with DJI.
        </footer>
      </main>
    </div>
  );
}
