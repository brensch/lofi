export const APP_VERSION = "2026.07.16.3" as const;

export function versionedAssetUrl(url: string) {
  const separator = url.includes("?") ? "&" : "?";
  return `${url}${separator}v=${encodeURIComponent(APP_VERSION)}`;
}
