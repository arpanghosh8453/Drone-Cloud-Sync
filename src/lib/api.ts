import { invoke, isTauri } from "@tauri-apps/api/core";
export const desktop = isTauri();
export async function command<T>(
  name: string,
  args?: Record<string, unknown>,
): Promise<T> {
  if (!desktop)
    throw {
      kind: "desktop",
      message:
        "Open drone-cloud-sync as a desktop application to use this feature. Run npm run tauri dev.",
    };
  return invoke<T>(name, args);
}
