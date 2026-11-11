import { useEffect, useState } from "react";

import { ACCESS_CLASS, KIND_ACCESS, KIND_LABEL, stepTarget } from "@/lib/workflow";
import {
  ChevronLeftIcon,
  ChevronRightIcon,
  ChevronUpIcon,
  CloseIcon,
} from "@/components/icons/icons";
import type { SqlToken } from "@/lib/sqlHighlight";
import type { Workflow } from "@/types";

import "./StepBar.css";

type StepBarProps = {
  workflow: Workflow;
  /** Index of the current step, from 0. */
  step: number;
  onStep: (step: number) => void;
  onStop: () => void;
};

/**
 * Playback controls for a workflow, floating over the bottom of the canvas.
 *
 * The step's description — the comment above its statement — sits over the
 * controls. The statement is one line, cut to fit; expanding the bar shows the
 * whole of it above, laid out over lines and highlighted, and stays expanded
 * from step to step. A line too long for the bar wraps, indented under its
 * start. The arrow keys step (← →) and expand (↑ ↓) too, unless something that
 * takes typing has focus.
 */
export function StepBar({ workflow, step, onStep, onStop }: StepBarProps) {
  const count = workflow.steps.length;
  const current = workflow.steps[step];
  const [expanded, setExpanded] = useState(false);
  const [highlight, setHighlight] = useState<((sql: string) => SqlToken[][]) | null>(null);

  // Shiki is fetched the first time the statement is shown, not with the page.
  // Until it arrives the statement is drawn plain, laid out the same way.
  useEffect(() => {
    if (!expanded || highlight) return;
    let live = true;
    void import("@/lib/sqlHighlight").then(({ sqlTokens }) => {
      if (live) setHighlight(() => sqlTokens);
    });
    return () => {
      live = false;
    };
  }, [expanded, highlight]);

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea, select, dialog")) return;
      if (event.key === "ArrowLeft" && step > 0) onStep(step - 1);
      else if (event.key === "ArrowRight" && step < count - 1) onStep(step + 1);
      else if (event.key === "ArrowUp") setExpanded(true);
      else if (event.key === "ArrowDown") setExpanded(false);
      else if (event.key === "Escape") onStop();
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [step, count, onStep, onStop]);

  return (
    <div className="craber-stepbar" role="toolbar" aria-label={`Workflow ${workflow.name}`}>
      {expanded && current && (
        <pre className="craber-stepbar-statement">
          <code>
            {(highlight ? highlight(current.sql) : plainLines(current.sql)).map((line, i) => (
              <span key={i} className="craber-stepbar-line">
                {line.map((token, j) => (
                  <span key={j} style={token.color ? { color: token.color } : undefined}>
                    {token.text}
                  </span>
                ))}
              </span>
            ))}
          </code>
        </pre>
      )}
      {current?.description && (
        <p className="craber-stepbar-description">{current.description}</p>
      )}
      <div className="craber-stepbar-controls">
        <span
          className="craber-stepbar-name"
          title={workflow.description ? `${workflow.name}\n\n${workflow.description}` : workflow.name}
        >
          {workflow.name}
        </span>
        <button
          type="button"
          className="craber-icon-button"
          title="Previous step (←)"
          disabled={step === 0}
          onClick={() => onStep(step - 1)}
        >
          <ChevronLeftIcon size={14} />
        </button>
        <span className="craber-stepbar-position">
          {count === 0 ? 0 : step + 1}/{count}
        </span>
        <button
          type="button"
          className="craber-icon-button"
          title="Next step (→)"
          disabled={step >= count - 1}
          onClick={() => onStep(step + 1)}
        >
          <ChevronRightIcon size={14} />
        </button>

        {current ? (
          <>
            <span className={`craber-stepbar-kind ${ACCESS_CLASS[KIND_ACCESS[current.kind]]}`}>
              {KIND_LABEL[current.kind]}
            </span>
            <span className="craber-stepbar-table">{stepTarget(current)}</span>
            <code className="craber-stepbar-sql" title={expanded ? undefined : current.sql}>
              {current.sql}
            </code>
          </>
        ) : (
          <span className="craber-stepbar-sql">This workflow has no steps.</span>
        )}

        <button
          type="button"
          className={`craber-icon-button craber-stepbar-toggle${expanded ? " is-expanded" : ""}`}
          title={expanded ? "Hide the statement (↓)" : "Show the whole statement (↑)"}
          aria-expanded={expanded}
          disabled={!current}
          onClick={() => setExpanded((open) => !open)}
        >
          <ChevronUpIcon size={14} />
        </button>
        <button type="button" className="craber-icon-button" title="Stop (Esc)" onClick={onStop}>
          <CloseIcon size={14} />
        </button>
      </div>
    </div>
  );
}

/** The statement before Shiki has loaded: the same lines, uncoloured. */
function plainLines(sql: string): SqlToken[][] {
  return sql.split("\n").map((text) => [{ text, color: "" }]);
}
