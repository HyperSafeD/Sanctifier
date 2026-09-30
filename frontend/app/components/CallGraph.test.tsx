import { describe, it, expect } from "vitest";
import { render, screen } from "@testing-library/react";
import userEvent from "@testing-library/user-event";
import { CallGraph } from "./CallGraph";
import type { CallGraphNode, CallGraphEdge } from "../types";

/** Node count above which CallGraph gates rendering behind a confirmation. */
const RENDER_THRESHOLD = 100;

function node(overrides: Partial<CallGraphNode> & Pick<CallGraphNode, "id">): CallGraphNode {
  return {
    label: overrides.id,
    type: "function",
    ...overrides,
  };
}

function manyNodes(count: number): CallGraphNode[] {
  return Array.from({ length: count }, (_, i) => node({ id: `fn_${i}` }));
}

/** The rendered SVG canvas, or `null` when the graph is gated / empty. */
function graphSvg(): SVGElement | null {
  return screen.queryByRole("img", { name: /contract interaction graph/i }) as SVGElement | null;
}

describe("CallGraph", () => {
  describe("empty states", () => {
    it("renders the empty-state copy when there are no nodes", () => {
      render(<CallGraph nodes={[]} edges={[]} />);

      expect(
        screen.getByText(/no cross-contract call paths were reported/i),
      ).toBeInTheDocument();
      expect(graphSvg()).toBeNull();
    });

    it("renders the empty state rather than throwing when nodes is undefined", () => {
      // The dashboard passes through whatever extractCallGraph returns; a
      // malformed report can leave these undefined.
      render(
        <CallGraph
          nodes={undefined as unknown as CallGraphNode[]}
          edges={undefined as unknown as CallGraphEdge[]}
        />,
      );

      expect(
        screen.getByText(/no cross-contract call paths were reported/i),
      ).toBeInTheDocument();
    });

    it("renders a graph with nodes but no edges", () => {
      render(<CallGraph nodes={[node({ id: "transfer" })]} edges={[]} />);

      expect(graphSvg()).not.toBeNull();
      expect(screen.getByText("transfer")).toBeInTheDocument();
    });
  });

  describe("summary line", () => {
    it("counts internal and external edges separately", () => {
      const nodes = [
        node({ id: "a" }),
        node({ id: "b" }),
        node({ id: "c", type: "external" }),
      ];
      const edges: CallGraphEdge[] = [
        { source: "a", target: "b", type: "internal" },
        { source: "b", target: "c", type: "calls" },
        { source: "a", target: "c", type: "calls" },
      ];

      render(<CallGraph nodes={nodes} edges={edges} />);

      expect(screen.getByText("1 internal")).toBeInTheDocument();
      expect(screen.getByText("2 external")).toBeInTheDocument();
    });

    it("uses the singular noun for a single contract", () => {
      const { container } = render(<CallGraph nodes={[node({ id: "only" })]} edges={[]} />);

      expect(container.textContent).toContain("1 contract ");
      expect(container.textContent).not.toContain("1 contracts");
    });

    it("uses the plural noun for multiple contracts", () => {
      const { container } = render(
        <CallGraph nodes={[node({ id: "a" }), node({ id: "b" })]} edges={[]} />,
      );

      expect(container.textContent).toContain("2 contracts");
    });
  });

  describe("large-graph guard", () => {
    it("renders the graph directly at the threshold", () => {
      render(<CallGraph nodes={manyNodes(RENDER_THRESHOLD)} edges={[]} />);

      expect(graphSvg()).not.toBeNull();
      expect(screen.queryByRole("button", { name: /show graph anyway/i })).toBeNull();
    });

    it("gates rendering one node above the threshold", () => {
      render(<CallGraph nodes={manyNodes(RENDER_THRESHOLD + 1)} edges={[]} />);

      expect(graphSvg()).toBeNull();
      expect(screen.getByText(/large graph detected/i)).toBeInTheDocument();
      expect(screen.getByRole("button", { name: /show graph anyway/i })).toBeInTheDocument();
    });

    it("renders the graph after the user opts in", async () => {
      const user = userEvent.setup();
      render(<CallGraph nodes={manyNodes(RENDER_THRESHOLD + 1)} edges={[]} />);

      await user.click(screen.getByRole("button", { name: /show graph anyway/i }));

      expect(graphSvg()).not.toBeNull();
      expect(screen.queryByRole("button", { name: /show graph anyway/i })).toBeNull();
    });

    it("keeps the header summary visible while the graph is gated", () => {
      render(<CallGraph nodes={manyNodes(RENDER_THRESHOLD + 1)} edges={[]} />);

      expect(screen.getByText(/101 contracts/)).toBeInTheDocument();
    });
  });

  describe("node rendering", () => {
    it("lays out each node type in its own column", () => {
      const nodes = [
        node({ id: "fn", type: "function" }),
        node({ id: "store", type: "storage" }),
        node({ id: "ext", type: "external" }),
      ];

      const { container } = render(<CallGraph nodes={nodes} edges={[]} />);
      const rects = Array.from(container.querySelectorAll("svg > g > rect"));
      const xs = rects.map((r) => Number(r.getAttribute("x")));

      expect(new Set(xs).size).toBe(3);
      expect([...xs].sort((a, b) => a - b)).toEqual(xs.slice().sort((a, b) => a - b));
    });

    it("truncates labels longer than 16 characters", () => {
      render(
        <CallGraph
          nodes={[node({ id: "n1", label: "an_extremely_long_function_name" })]}
          edges={[]}
        />,
      );

      expect(screen.getByText("an_extremely_l…")).toBeInTheDocument();
      expect(screen.queryByText("an_extremely_long_function_name")).toBeNull();
    });

    it("renders a 16-character label untruncated", () => {
      const label = "sixteen_chars_ok";
      expect(label).toHaveLength(16);

      render(<CallGraph nodes={[node({ id: "n1", label })]} edges={[]} />);

      expect(screen.getByText(label)).toBeInTheDocument();
    });

    it("draws a severity ring only for nodes that carry a severity", () => {
      const nodes = [
        node({ id: "risky", severity: "critical" }),
        node({ id: "plain" }),
      ];

      const { container } = render(<CallGraph nodes={nodes} edges={[]} />);
      const groups = Array.from(container.querySelectorAll("svg > g"));

      expect(groups).toHaveLength(2);
      // The flagged node gets a dashed ring rect in addition to its body rect.
      expect(groups[0].querySelectorAll("rect")).toHaveLength(2);
      expect(groups[1].querySelectorAll("rect")).toHaveLength(1);
    });

    it("drops a node with an unrecognised type without throwing", () => {
      const nodes = [node({ id: "weird", type: "mystery" as CallGraphNode["type"] })];

      const { container } = render(<CallGraph nodes={nodes} edges={[]} />);

      // layoutNodes only places the three known types, so nothing is drawn —
      // but the empty layout must not blow up the SVG bounds calculation.
      const svg = container.querySelector("svg[role='img']");
      expect(svg).not.toBeNull();
      expect(svg?.getAttribute("width")).toBe("500");
      expect(container.querySelectorAll("svg[role='img'] > g")).toHaveLength(0);
    });
  });

  describe("edge rendering", () => {
    const twoNodes = [node({ id: "a" }), node({ id: "b" })];

    it("draws internal edges as curved paths", () => {
      const { container } = render(
        <CallGraph nodes={twoNodes} edges={[{ source: "a", target: "b", type: "internal" }]} />,
      );

      const path = container.querySelector("svg path");
      expect(path).not.toBeNull();
      expect(path?.getAttribute("d")).toMatch(/^M .* Q .*/);
      expect(path?.getAttribute("marker-end")).toBe("url(#arrowhead-internal)");
    });

    it("draws non-internal edges as straight lines", () => {
      const { container } = render(
        <CallGraph nodes={twoNodes} edges={[{ source: "a", target: "b", type: "calls" }]} />,
      );

      // The legend also contains <line> swatches, so scope the query to the graph.
      const lines = container.querySelectorAll("svg[role='img'] line");
      expect(lines).toHaveLength(1);
      expect(lines[0].getAttribute("marker-end")).toBe("url(#arrowhead-calls)");
    });

    it("skips edges whose endpoints are not in the node list", () => {
      const { container } = render(
        <CallGraph
          nodes={twoNodes}
          edges={[
            { source: "a", target: "ghost", type: "calls" },
            { source: "ghost", target: "b", type: "calls" },
            { source: "a", target: "b", type: "calls" },
          ]}
        />,
      );

      expect(container.querySelectorAll("svg[role='img'] line")).toHaveLength(1);
    });

    it("renders an edge with an unrecognised type without throwing", () => {
      const { container } = render(
        <CallGraph
          nodes={twoNodes}
          edges={[{ source: "a", target: "b", type: "unknown" as CallGraphEdge["type"] }]}
        />,
      );

      expect(container.querySelectorAll("svg[role='img'] line")).toHaveLength(1);
    });

    it("tolerates a self-referential edge", () => {
      const { container } = render(
        <CallGraph
          nodes={[node({ id: "recurse" })]}
          edges={[{ source: "recurse", target: "recurse", type: "internal" }]}
        />,
      );

      expect(container.querySelector("svg[role='img'] path")).not.toBeNull();
    });
  });

  describe("accessibility", () => {
    it("labels the canvas for screen readers", () => {
      render(<CallGraph nodes={[node({ id: "a" })]} edges={[]} />);

      expect(graphSvg()).toHaveAttribute(
        "aria-label",
        "Contract interaction graph visualization",
      );
    });

    it("hides decorative legend swatches from the accessibility tree", () => {
      const { container } = render(<CallGraph nodes={[node({ id: "a" })]} edges={[]} />);

      const legendSvgs = Array.from(container.querySelectorAll("svg")).filter(
        (svg) => svg.getAttribute("role") !== "img",
      );
      expect(legendSvgs.length).toBeGreaterThan(0);
      legendSvgs.forEach((svg) => expect(svg).toHaveAttribute("aria-hidden", "true"));
    });

    it("names every legend entry", () => {
      render(<CallGraph nodes={[node({ id: "a" })]} edges={[]} />);

      ["Function", "Storage", "External"].forEach((label) =>
        expect(screen.getByText(label)).toBeInTheDocument(),
      );
      ["Internal call", "External call", "Mutates", "Reads"].forEach((label) =>
        expect(screen.getByText(label)).toBeInTheDocument(),
      );
    });
  });
});
