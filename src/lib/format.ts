// Display helpers for keyrings and items.

import type { CollectionInfo, ItemInfo } from "./keyring";

const dateFormat = new Intl.DateTimeFormat(undefined, { dateStyle: "medium", timeStyle: "short" });

/** Formats seconds since the epoch; 0 means unknown. */
export function formatDate(secs: number): string {
  return secs > 0 ? dateFormat.format(new Date(secs * 1000)) : "Unknown";
}

export function itemCount(n: number): string {
  return n === 1 ? "1 item" : `${n} items`;
}

/** oo7 names the session keyring "session"; show it like a title. */
export function collectionLabel(c: CollectionInfo): string {
  if (c.isSession && c.label.toLowerCase() === "session") return "Session";
  return c.label || "Untitled";
}

export function collectionSubtitle(c: CollectionInfo): string {
  const parts = [itemCount(c.itemCount)];
  if (c.isDefault) parts.unshift("Default");
  if (c.isSession) parts.unshift("Until logout");
  return parts.join(" · ");
}

interface SchemaInfo {
  name: string;
  icon: string;
}

// Schemas commonly found in GNOME keyrings.
const KNOWN_SCHEMAS: Record<string, SchemaInfo> = {
  "org.gnome.keyring.NetworkPassword": { name: "Network password", icon: "network-server" },
  "org.freedesktop.NetworkManager.Connection": { name: "Network connection", icon: "network-wireless" },
  "org.gnome.keyring.Note": { name: "Password or note", icon: "dialog-password" },
  "org.gnome.OnlineAccounts": { name: "Online account", icon: "avatar-default" },
  "org.freedesktop.Secret.Generic": { name: "Password", icon: "dialog-password" },
  "chrome_libsecret_os_crypt_password_v2": { name: "Browser encryption key", icon: "web-browser" },
  "chrome_libsecret_password_schema": { name: "Browser password", icon: "web-browser" },
};

/** Turns "org.example.MyApp.Password" into "My App Password". */
function humanizeSchema(schema: string): string {
  const segments = schema.split(".");
  const tail = segments.length > 2 ? segments.slice(2).join(" ") : schema;
  return tail
    .replace(/[_-]+/g, " ")
    .replace(/([a-z])([A-Z])/g, "$1 $2")
    .trim();
}

export function schemaInfo(item: ItemInfo): SchemaInfo {
  const schema = item.schema;
  if (schema && KNOWN_SCHEMAS[schema]) return KNOWN_SCHEMAS[schema];
  const app = (item.attributes.application ?? "").toLowerCase();
  if (app.includes("chrom") || app.includes("firefox") || app.includes("browser")) {
    return { name: schema ? humanizeSchema(schema) : "Browser secret", icon: "web-browser" };
  }
  return { name: schema ? humanizeSchema(schema) : "Secret", icon: "dialog-password" };
}

const ACCOUNT_KEYS = ["username", "user", "account", "email", "login"];
const PLACE_KEYS = ["server", "host", "domain", "service", "origin_url", "url", "application"];

function firstAttr(item: ItemInfo, keys: string[]): string | undefined {
  for (const k of keys) {
    const v = item.attributes[k];
    if (v) return v;
  }
  return undefined;
}

/** One-line summary for list rows, e.g. "alice · example.com". */
export function itemSubtitle(item: ItemInfo): string {
  const parts = [firstAttr(item, ACCOUNT_KEYS), firstAttr(item, PLACE_KEYS)].filter(Boolean);
  return parts.length ? parts.join(" · ") : schemaInfo(item).name;
}

/** Attributes shown in the details page; the schema has its own row. */
export function visibleAttributes(item: ItemInfo): [string, string][] {
  return Object.entries(item.attributes).filter(([k]) => k !== "xdg:schema");
}

/** Case-insensitive match on label, attribute values and schema name. */
export function matchesQuery(item: ItemInfo, query: string): boolean {
  const q = query.trim().toLowerCase();
  if (!q) return true;
  const haystack = [item.label, schemaInfo(item).name, ...Object.values(item.attributes)];
  return haystack.some((s) => s.toLowerCase().includes(q));
}
