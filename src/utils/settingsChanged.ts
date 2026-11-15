import type { Settings } from "../types";

const sameValue = (a: unknown, b: unknown): boolean => {
  if (Array.isArray(a) && Array.isArray(b)) {
    return a.length === b.length && a.every((item, index) => item === b[index]);
  }
  return a === b;
};

/** Whether any setting differs from the saved one. Lists are compared by their items. */
export function settingsChanged(current: Settings, saved: Settings): boolean {
  return (Object.keys(current) as (keyof Settings)[]).some((key) => !sameValue(current[key], saved[key]));
}
