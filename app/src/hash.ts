import { useSyncExternalStore } from "react";

function subscribe(onChange: () => void) {
  window.addEventListener("hashchange", onChange);
  return () => window.removeEventListener("hashchange", onChange);
}

export function useHash(): string {
  return useSyncExternalStore(subscribe, () => window.location.hash);
}

export function useSubpath(): string[] {
  return useHash().replace(/^#/, "").split("/").slice(1).map(decodeURIComponent);
}

export function go(path: string) {
  window.location.hash = path;
}
