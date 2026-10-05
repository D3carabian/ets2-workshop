import { useEffect, useRef, useState } from "react";
import { getCurrentWindow } from "@tauri-apps/api/window";

// Both the titlebar and native OS close requests use this guard. Never interrupt a write.
export function useCloseGuard(
  pending: number,
  busy: boolean,
  onError: (error: unknown) => void,
) {
  const [exitPrompt, setExitPrompt] = useState(false);
  const current = useRef({ pending, busy, onError });
  current.current = { pending, busy, onError };
  useEffect(() => {
    if (
      !(window as unknown as { __TAURI_INTERNALS__?: { metadata?: unknown } })
        .__TAURI_INTERNALS__?.metadata
    )
      return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void getCurrentWindow()
      .onCloseRequested((event) => {
        if (current.current.pending || current.current.busy) {
          event.preventDefault();
          setExitPrompt(true);
        }
      })
      .then((stop) => {
        if (disposed) stop();
        else unlisten = stop;
      })
      .catch((error) => current.current.onError(error));
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);
  const requestClose = () => {
    if (current.current.pending || current.current.busy) setExitPrompt(true);
    else void getCurrentWindow().close().catch(current.current.onError);
  };
  const discardAndClose = () => {
    if (current.current.busy) return;
    // Explicit discard is the only pending-change path allowed to bypass CloseRequested.
    void getCurrentWindow().destroy().catch(current.current.onError);
  };
  return { exitPrompt, setExitPrompt, requestClose, discardAndClose };
}
