"use client";

import { useMemo } from "react";

interface CodeSnippetProps {
  code: string;
  highlightLine?: number;
  highlightEndLine?: number;
  language?: string;
  maxHeight?: string;
}

const RUST_KEYWORDS = new Set([
  "as", "break", "const", "continue", "crate", "else", "enum",
  "extern", "false", "fn", "for", "if", "impl", "in", "let",
  "loop", "match", "mod", "move", "mut", "pub", "ref", "return",
  "self", "Self", "static", "struct", "super", "trait", "true",
  "type", "unsafe", "use", "where", "while", "async", "await",
  "dyn", "abstract", "become", "box", "do", "final", "macro",
  "override", "priv", "typeof", "unsized", "virtual", "yield",
  "try",
]);

const SOROBAN_TYPES = new Set([
  "Address", "Bytes", "BytesN", "String", "Symbol", "Vec", "Map",
  "Option", "Result", "Env", "Val", "RawVal", "Bool", "Void",
  "I128", "U128", "I256", "U256", "i128", "u128", "u32", "i32",
  "u64", "i64", "u8", "i8",
]);

function tokenizeLine(line: string): { text: string; className: string }[] {
  const tokens: { text: string; className: string }[] = [];
  const regex = /(\/\/.*)|("(?:[^"\\]|\\.)*")|('(?:[^'\\]|\\.)*')|(\b\d[\d_]*(?:\.[\d_]+)?(?:[eE][+-]?\d+)?\b)|(\b[a-zA-Z_]\w*\b)|([{}()\[\];,.:<>!=+\-*/%&|^~@#]|\s+)/g;
  let match: RegExpExecArray | null;

  while ((match = regex.exec(line)) !== null) {
    if (match[1]) {
      tokens.push({ text: match[1], className: "text-emerald-500 italic" });
    } else if (match[2]) {
      tokens.push({ text: match[2], className: "text-amber-400" });
    } else if (match[3]) {
      tokens.push({ text: match[3], className: "text-amber-400" });
    } else if (match[4]) {
      tokens.push({ text: match[4], className: "text-cyan-400" });
    } else if (match[5]) {
      const word = match[5];
      if (RUST_KEYWORDS.has(word)) {
        tokens.push({ text: word, className: "text-purple-400 font-semibold" });
      } else if (SOROBAN_TYPES.has(word)) {
        tokens.push({ text: word, className: "text-blue-400" });
      } else if (word === word.toUpperCase() && word.length > 1 && word.includes("_")) {
        tokens.push({ text: word, className: "text-orange-300" });
      } else if (word.startsWith("DataKey") || word.endsWith("Key")) {
        tokens.push({ text: word, className: "text-orange-300" });
      } else {
        tokens.push({ text: word, className: "" });
      }
    } else if (match[6]) {
      const punct = match[6];
      if (punct.trim() === "") {
        tokens.push({ text: punct, className: "" });
      } else {
        tokens.push({ text: punct, className: "text-zinc-400" });
      }
    }
  }

  if (tokens.length === 0) {
    tokens.push({ text: line, className: "" });
  }

  return tokens;
}

export function CodeSnippet({
  code,
  highlightLine,
  highlightEndLine,
  language = "rust",
  maxHeight,
}: CodeSnippetProps) {
  const lines = useMemo(() => code.split("\n"), [code]);
  const tokenizedLines = useMemo(
    () => lines.map((line) => tokenizeLine(line)),
    [lines]
  );

  const isInRange = (lineNum: number) => {
    if (highlightLine === undefined) return false;
    if (highlightEndLine !== undefined) {
      return lineNum >= highlightLine && lineNum <= highlightEndLine;
    }
    return lineNum === highlightLine;
  };

  return (
    <div
      className="overflow-x-auto rounded-lg bg-zinc-900 dark:bg-zinc-950 text-sm font-mono text-zinc-100"
      style={maxHeight ? { maxHeight, overflowY: "auto" } : undefined}
    >
      <table className="border-collapse w-full">
        <tbody>
          {tokenizedLines.map((tokens, i) => {
            const lineNum = i + 1;
            const highlighted = isInRange(lineNum);
            return (
              <tr
                key={i}
                className={
                  highlighted
                    ? "bg-amber-500/20 border-l-2 border-amber-500"
                    : ""
                }
              >
                <td className="select-none text-zinc-500 text-right w-10 pr-4 pl-2 py-0.5 align-top">
                  {lineNum}
                </td>
                <td className="py-0.5 whitespace-pre">
                  {tokens.length === 1 && tokens[0].text === "" ? (
                    <span>&nbsp;</span>
                  ) : (
                    tokens.map((t, j) =>
                      t.className ? (
                        <span key={j} className={t.className}>
                          {t.text}
                        </span>
                      ) : (
                        <span key={j}>{t.text}</span>
                      )
                    )
                  )}
                </td>
              </tr>
            );
          })}
        </tbody>
      </table>
    </div>
  );
}
