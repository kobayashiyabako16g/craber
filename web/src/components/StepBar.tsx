import { useEffect } from "react";

import { ACCESS_CLASS, KIND_ACCESS, KIND_LABEL, stepTarget } from "../lib/flow";
import { ChevronLeftIcon, ChevronRightIcon, CloseIcon } from "./icons";
import type { Workflow } from "../types";

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
 * The statement is one line, cut to fit; the whole of it is the tooltip. The
 * arrow keys step too, unless something that takes typing has focus.
 */
export function StepBar({ workflow, step, onStep, onStop }: StepBarProps) {
  const count = workflow.steps.length;
  const current = workflow.steps[step];

  useEffect(() => {
    const onKey = (event: KeyboardEvent) => {
      const target = event.target as HTMLElement | null;
      if (target?.closest("input, textarea, select, dialog")) return;
      if (event.key === "ArrowLeft" && step > 0) onStep(step - 1);
      else if (event.key === "ArrowRight" && step < count - 1) onStep(step + 1);
      else if (event.key === "Escape") onStop();
      else return;
      event.preventDefault();
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [step, count, onStep, onStop]);

  return (
    <div className="craber-stepbar" role="toolbar" aria-label={`Workflow ${workflow.name}`}>
      <span className="craber-stepbar-name" title={workflow.name}>
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
          <code className="craber-stepbar-sql" title={current.sql}>
            {current.sql}
          </code>
        </>
      ) : (
        <span className="craber-stepbar-sql">This workflow has no steps.</span>
      )}

      <button type="button" className="craber-icon-button" title="Stop (Esc)" onClick={onStop}>
        <CloseIcon size={14} />
      </button>
    </div>
  );
}
