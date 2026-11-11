import { describe, expect, it } from "vitest";
import {
  collectionLabel,
  collectionSubtitle,
  formatDate,
  itemSubtitle,
  matchesQuery,
  schemaInfo,
  visibleAttributes,
} from "./format";
import type { CollectionInfo, ItemInfo } from "./keyring";

function item(over: Partial<ItemInfo> = {}): ItemInfo {
  return {
    id: "/i/1",
    collectionId: "/c",
    label: "Item",
    locked: false,
    attributes: {},
    schema: null,
    created: 0,
    modified: 0,
    ...over,
  };
}

function collection(over: Partial<CollectionInfo> = {}): CollectionInfo {
  return {
    id: "/c",
    label: "Login",
    locked: false,
    isDefault: false,
    isSession: false,
    itemCount: 0,
    created: 0,
    modified: 0,
    ...over,
  };
}

describe("schemaInfo", () => {
  it("names known schemas", () => {
    expect(schemaInfo(item({ schema: "org.gnome.keyring.NetworkPassword" }))).toEqual({
      name: "Network password",
      icon: "network-server",
    });
  });

  it("humanizes unknown schemas", () => {
    expect(schemaInfo(item({ schema: "org.example.MyApp.AccessToken" })).name).toBe("My App Access Token");
    expect(schemaInfo(item({ schema: "custom_schema" })).name).toBe("custom schema");
  });

  it("recognizes browsers by application attribute", () => {
    expect(schemaInfo(item({ attributes: { application: "chromium" } })).icon).toBe("web-browser");
  });

  it("falls back to a generic secret", () => {
    expect(schemaInfo(item())).toEqual({ name: "Secret", icon: "dialog-password" });
  });
});

describe("itemSubtitle", () => {
  it("combines account and place", () => {
    expect(itemSubtitle(item({ attributes: { user: "alice", server: "github.com" } }))).toBe("alice · github.com");
  });

  it("prefers username over user", () => {
    expect(itemSubtitle(item({ attributes: { user: "a", username: "b" } }))).toBe("b");
  });

  it("falls back to the kind of secret", () => {
    expect(itemSubtitle(item({ schema: "org.gnome.OnlineAccounts" }))).toBe("Online account");
  });
});

describe("matchesQuery", () => {
  const github = item({ label: "GitHub", attributes: { user: "alice", server: "github.com" } });

  it("matches everything on an empty query", () => {
    expect(matchesQuery(github, "  ")).toBe(true);
  });

  it("matches label, attributes and kind case-insensitively", () => {
    expect(matchesQuery(github, "GITHUB")).toBe(true);
    expect(matchesQuery(github, "alice")).toBe(true);
    expect(matchesQuery(item({ schema: "org.gnome.keyring.NetworkPassword" }), "network")).toBe(true);
    expect(matchesQuery(github, "bob")).toBe(false);
  });
});

describe("collections", () => {
  it("titles the session keyring", () => {
    expect(collectionLabel(collection({ isSession: true, label: "session" }))).toBe("Session");
    expect(collectionLabel(collection({ label: "" }))).toBe("Untitled");
  });

  it("describes default and session keyrings", () => {
    expect(collectionSubtitle(collection({ isDefault: true, itemCount: 1 }))).toBe("Default · 1 item");
    expect(collectionSubtitle(collection({ isSession: true }))).toBe("Until logout · 0 items");
  });
});

describe("misc", () => {
  it("hides the schema attribute from the attribute list", () => {
    const i = item({ attributes: { "xdg:schema": "x", user: "a" } });
    expect(visibleAttributes(i)).toEqual([["user", "a"]]);
  });

  it("formats unknown dates", () => {
    expect(formatDate(0)).toBe("Unknown");
    expect(formatDate(1_700_000_000)).not.toBe("Unknown");
  });
});
