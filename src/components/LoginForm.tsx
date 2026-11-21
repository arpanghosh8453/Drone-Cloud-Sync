import { useEffect, useRef, useState } from "react";
import {
  ArrowRight,
  Eye,
  EyeOff,
  LoaderCircle,
  LockKeyhole,
} from "lucide-react";
import { command } from "../lib/api";
import { errorMessage } from "../lib/utils";
import type { Captcha, Session } from "../types";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import { Checkbox } from "./ui/checkbox";
import { CaptchaInput } from "./CaptchaInput";
export function LoginForm({
  session,
  onConnected,
}: {
  session: Session;
  onConnected: (s: Session) => void;
}) {
  const [email, setEmail] = useState(session.email ?? "");
  const [password, setPassword] = useState("");
  const [show, setShow] = useState(false);
  const [remember, setRemember] = useState(true);
  const [challenge, setChallenge] = useState<Captcha | null>(null);
  const [code, setCode] = useState("");
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const initial = useRef(false);
  async function refresh() {
    setBusy(true);
    setError(null);
    setChallenge(null);
    setCode("");
    try {
      setChallenge(await command<Captcha>("request_captcha"));
    } catch (e) {
      setError(errorMessage(e));
    } finally {
      setBusy(false);
    }
  }
  useEffect(() => {
    if (session.state === "Expired" && !initial.current) {
      initial.current = true;
      void refresh();
    }
  }, [session.state]);
  async function submit(e: React.SubmitEvent<HTMLFormElement>) {
    e.preventDefault();
    if (!challenge) {
      await refresh();
      return;
    }
    setBusy(true);
    setError(null);
    const pending = command<Session>("login", {
      email,
      password,
      challengeId: challenge.challengeId,
      captcha: code,
      remember,
    });
    setPassword("");
    setShow(false);
    try {
      onConnected(await pending);
    } catch (err) {
      setError(errorMessage(err));
      setChallenge(null);
      setCode("");
      try {
        setChallenge(await command<Captcha>("request_captcha"));
      } catch {
        /* Original login error remains visible. */
      }
    } finally {
      setBusy(false);
    }
  }
  return (
    <form onSubmit={submit} className="login-form">
      {session.state === "Expired" && (
        <div className="notice warning" role="alert">
          Your cloud session has expired. Please sign in again. Paused
          operations will resume after you connect.
        </div>
      )}
      {error && (
        <div className="notice danger" role="alert">
          {error}
        </div>
      )}
      <div className="field">
        <label htmlFor="email">Account email</label>
        <Input
          id="email"
          type="email"
          autoComplete="username"
          placeholder="you@example.com"
          required
          value={email}
          onChange={(e) => setEmail(e.target.value)}
          disabled={busy}
        />
      </div>
      <div className="field">
        <label htmlFor="password">Password</label>
        <div className="input-icon-wrap">
          <Input
            id="password"
            type={show ? "text" : "password"}
            autoComplete="current-password"
            placeholder="Enter your account password"
            required
            value={password}
            onChange={(e) => setPassword(e.target.value)}
            disabled={busy}
          />
          <button
            type="button"
            className="input-icon"
            title={show ? "Hide password" : "Show password"}
            aria-label={show ? "Hide password" : "Show password"}
            aria-pressed={show}
            onClick={() => setShow(!show)}
          >
            {show ? <EyeOff size={17} /> : <Eye size={17} />}
          </button>
        </div>
      </div>
      {challenge && (
        <CaptchaInput
          challenge={challenge}
          code={code}
          setCode={setCode}
          refresh={() => void refresh()}
          busy={busy}
        />
      )}
      <label className="check-label">
        <Checkbox
          checked={remember}
          onCheckedChange={(v) => setRemember(v === true)}
          disabled={busy}
        />
        Remember session on this device
      </label>
      <Button
        className="login-submit"
        type="submit"
        disabled={busy || (!!challenge && code.length !== 4)}
      >
        {busy ? <LoaderCircle size={17} className="spin" /> : null}
        {busy
          ? "Connecting…"
          : challenge
            ? "Verify & connect"
            : "Connect account"}
        {!busy && <ArrowRight size={17} />}
      </Button>
      <div className="secure-note">
        <LockKeyhole size={14} />
        <span>
          Your password is never saved. Remembered sessions use your system’s
          secure credential vault.
        </span>
      </div>
    </form>
  );
}
