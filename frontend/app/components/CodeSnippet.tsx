"use client";

interface CodeSnippetProps {
  code: string;
  highlightLine?: number;
  language?: string;
}

export function CodeSnippet({ code, highlightLine }: CodeSnippetProps) {
  const lines = code.split("\n");

  return (
    <pre className="overflow-x-auto rounded-lg bg-zinc-900 dark:bg-zinc-950 p-4 text-sm font-mono text-zinc-100">
      <code>
        {lines.map((line, i) => (
          <div
            key={i}
            className={`px-2 py-0.5 -mx-2 ${
              highlightLine === i + 1
                ? "bg-amber-500/20 border-l-2 border-amber-500"
                : ""
            }`}
          >
            <span className="select-none text-zinc-500 w-8 inline-block mr-4">
              {i + 1}
            </span>
            {line || " "}
          </div>
        ))}
      </code>
    </pre>
  );
}
