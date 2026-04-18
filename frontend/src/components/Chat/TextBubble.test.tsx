/**
 * TextBubble Component Tests
 *
 * Tests for the text bubble component with:
 * - Markdown rendering for both user and assistant messages
 * - User vs assistant styling
 * - Copy button functionality
 */

import { render, screen } from "@testing-library/react";
import { describe, it, expect } from "vitest";
import { TextBubble } from "./TextBubble";

describe("TextBubble", () => {

  // ============================================================================
  // Rendering Tests
  // ============================================================================

  describe("rendering", () => {
    it("renders plain text for user messages", () => {
      render(<TextBubble text="Hello world" isUser={true} />);
      expect(screen.getByText("Hello world")).toBeInTheDocument();
    });

    it("renders plain text for assistant messages", () => {
      render(<TextBubble text="Hello world" isUser={false} />);
      expect(screen.getByText("Hello world")).toBeInTheDocument();
    });

    it("renders the copy button", () => {
      render(<TextBubble text="Hello" isUser={true} />);
      expect(screen.getByLabelText("Copy message")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Markdown Rendering Tests
  // ============================================================================

  describe("markdown rendering", () => {
    it("renders headings in user messages", () => {
      render(<TextBubble text="# Heading 1" isUser={true} />);
      expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Heading 1");
    });

    it("renders headings in assistant messages", () => {
      render(<TextBubble text="# Heading 1" isUser={false} />);
      expect(screen.getByRole("heading", { level: 1 })).toHaveTextContent("Heading 1");
    });

    it("renders lists in user messages", () => {
      const text = "- Item 1\n- Item 2\n- Item 3";
      render(<TextBubble text={text} isUser={true} />);
      expect(screen.getByText("Item 1")).toBeInTheDocument();
      expect(screen.getByText("Item 2")).toBeInTheDocument();
      expect(screen.getByText("Item 3")).toBeInTheDocument();
    });

    it("renders lists in assistant messages", () => {
      const text = "- Item 1\n- Item 2\n- Item 3";
      render(<TextBubble text={text} isUser={false} />);
      expect(screen.getByText("Item 1")).toBeInTheDocument();
      expect(screen.getByText("Item 2")).toBeInTheDocument();
      expect(screen.getByText("Item 3")).toBeInTheDocument();
    });

    it("renders inline code in user messages", () => {
      render(<TextBubble text="Use `const` for constants" isUser={true} />);
      expect(screen.getByText("const")).toBeInTheDocument();
    });

    it("renders inline code in assistant messages", () => {
      render(<TextBubble text="Use `const` for constants" isUser={false} />);
      expect(screen.getByText("const")).toBeInTheDocument();
    });

    it("renders code blocks in user messages", () => {
      const text = "```javascript\nconst x = 1;\n```";
      render(<TextBubble text={text} isUser={true} />);
      expect(screen.getByText("const x = 1;")).toBeInTheDocument();
    });

    it("renders code blocks in assistant messages", () => {
      const text = "```javascript\nconst x = 1;\n```";
      render(<TextBubble text={text} isUser={false} />);
      expect(screen.getByText("const x = 1;")).toBeInTheDocument();
    });

    it("renders bold text in user messages", () => {
      render(<TextBubble text="This is **bold** text" isUser={true} />);
      expect(screen.getByText("bold")).toBeInTheDocument();
    });

    it("renders bold text in assistant messages", () => {
      render(<TextBubble text="This is **bold** text" isUser={false} />);
      expect(screen.getByText("bold")).toBeInTheDocument();
    });
  });

  // ============================================================================
  // Styling Tests
  // ============================================================================

  describe("styling", () => {
    it("applies user styling (orange background)", () => {
      const { container } = render(<TextBubble text="Hello" isUser={true} />);
      const bubble = container.firstChild as HTMLElement;
      expect(bubble).toHaveStyle({ background: "var(--accent-primary)" });
    });

    it("applies assistant styling (dark background)", () => {
      const { container } = render(<TextBubble text="Hello" isUser={false} />);
      const bubble = container.firstChild as HTMLElement;
      expect(bubble).toHaveStyle({ background: "var(--bg-elevated)" });
    });

    it("applies rounded corners", () => {
      const { container } = render(<TextBubble text="Hello" isUser={true} />);
      const bubble = container.firstChild as HTMLElement;
      expect(bubble).toHaveClass("rounded-xl");
    });

    it("applies padding", () => {
      const { container } = render(<TextBubble text="Hello" isUser={true} />);
      const bubble = container.firstChild as HTMLElement;
      expect(bubble).toHaveClass("px-3");
      expect(bubble).toHaveClass("py-2");
    });
  });

  // ============================================================================
  // Copy Functionality Tests
  // ============================================================================

  describe("copy functionality", () => {
    it("renders copy button with correct label", () => {
      render(<TextBubble text="Hello" isUser={true} />);
      expect(screen.getByLabelText("Copy message")).toBeInTheDocument();
    });

    it("copy button is present and accessible", () => {
      render(<TextBubble text="Hello" isUser={false} />);
      const copyButton = screen.getByLabelText("Copy message");
      expect(copyButton).toBeInTheDocument();
      expect(copyButton.tagName).toBe("BUTTON");
    });
  });

  // ============================================================================
  // Accessibility Tests
  // ============================================================================

  describe("accessibility", () => {
    it("copy button has proper aria-label", () => {
      render(<TextBubble text="Hello" isUser={true} />);
      expect(screen.getByLabelText("Copy message")).toBeInTheDocument();
    });

    it("copy button is keyboard accessible", () => {
      render(<TextBubble text="Hello" isUser={false} />);
      const button = screen.getByLabelText("Copy message");
      expect(button.tagName).toBe("BUTTON");
    });
  });
});
