import { fireEvent, render, screen, within } from "@testing-library/react";
import { describe, expect, it, vi } from "vitest";
import { Highlight } from "../src/components/Highlight";
import { NoteEditor } from "../src/features/notes/NoteEditor";
import { emptyNote } from "../src/types";

vi.mock("@tauri-apps/api/core", () => ({ invoke: vi.fn() }));
vi.mock("@tauri-apps/plugin-opener", () => ({ openUrl: vi.fn() }));

describe("core components", () => {
  it("highlights every matching keyword without injecting HTML", () => {
    render(<Highlight text="项目计划与项目复盘" terms={["项目"]} />);
    expect(screen.getAllByText("项目")).toHaveLength(2);
    expect(document.querySelectorAll("mark")).toHaveLength(2);
  });

  it("renders a new note editor with Markdown entry", () => {
    render(
      <NoteEditor
        note={emptyNote()}
        categories={[]}
        onSaved={vi.fn()}
        onRemoved={vi.fn()}
      />,
    );
    expect(screen.getByPlaceholderText("无标题")).toBeInTheDocument();
    expect(screen.getByPlaceholderText("开始记录…支持 Markdown 语法")).toBeInTheDocument();
  });

  it("keeps metadata collapsed until the user requests it", () => {
    const view = render(
      <NoteEditor
        note={emptyNote()}
        categories={[]}
        onSaved={vi.fn()}
        onRemoved={vi.fn()}
      />,
    );

    const editor = within(view.container);
    expect(editor.queryByText("分类")).not.toBeInTheDocument();
    expect(editor.queryByText("截止日期")).not.toBeInTheDocument();
    fireEvent.click(editor.getByRole("button", { name: "更多信息" }));
    expect(editor.getByText("分类")).toBeInTheDocument();
    expect(editor.getByText("截止日期")).toBeInTheDocument();
  });
});
