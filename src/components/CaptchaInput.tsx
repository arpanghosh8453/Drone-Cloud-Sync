import { RefreshCw } from "lucide-react";
import { Button } from "./ui/button";
import { Input } from "./ui/input";
import type { Captcha } from "../types";
export function CaptchaInput({
  challenge,
  code,
  setCode,
  refresh,
  busy,
}: {
  challenge: Captcha;
  code: string;
  setCode: (s: string) => void;
  refresh: () => void;
  busy: boolean;
}) {
  return (
    <div className="captcha-group">
      <div className="field-label">
        <label htmlFor="verification">Verification code</label>
        <span>4 characters</span>
      </div>
      <div className="captcha-row">
        <img
          src={challenge.imageDataUrl}
          alt="Cloud account verification challenge"
        />
        <Button
          type="button"
          variant="outline"
          size="icon"
          title="Refresh verification image"
          aria-label="Refresh verification image"
          disabled={busy}
          onClick={refresh}
        >
          <RefreshCw size={16} />
        </Button>
        <Input
          id="verification"
          autoFocus
          value={code}
          onChange={(e) =>
            setCode(e.target.value.replace(/[^a-z0-9]/gi, "").toUpperCase())
          }
          maxLength={4}
          placeholder="AB12"
          autoComplete="off"
          spellCheck={false}
          required
          aria-describedby="captcha-help"
        />
      </div>
      <p id="captcha-help" className="hint">
        Enter the characters shown in the image.
      </p>
    </div>
  );
}
