import { describe, expect, it } from "vitest";
import capability from "../src-tauri/capabilities/default.json";

describe("快速窗口能力配置", () => {
  it("允许无边框窗口请求原生拖动", () => {
    expect(capability.windows).toContain("quick");
    expect(capability.permissions).toContain("core:window:allow-start-dragging");
  });
});
