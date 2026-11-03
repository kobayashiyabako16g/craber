import { useEffect, useRef } from "react";

import { ACCESS_CLASS, KIND_ACCESS, KIND_LABEL } from "../lib/flow";
import { CloseIcon, FlowIcon } from "./icons";
import type { StepKind, Workflow } from "../types";

import "./WorkflowPicker.css";

type WorkflowPickerProps = {
  open: boolean;
  workflows: Workflow[];
  /** The workflow playing now, marked in the list. */
  current: string | null;
  onPick: (name: string) => void;
  onClose: () => void;
};

/**
 * The list of workflows, as a modal opened from the top bar.
 *
 * A native `<dialog>` opened with `showModal()`: the browser already does the
 * backdrop, Escape and keeping focus inside it, and does them right.
 */
export function WorkflowPicker({ open, workflows, current, onPick, onClose }: WorkflowPickerProps) {
  const dialog = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const element = dialog.current;
    if (!element) return;
    if (open && !element.open) element.showModal();
    if (!open && element.open) element.close();
  }, [open]);

  return (
    <dialog
      ref={dialog}
      className="craber-picker"
      // Escape closes the dialog itself; this tells the state it happened.
      onClose={onClose}
      // A click on the backdrop lands on the dialog element, outside its box.
      onClick={(event) => {
        if (event.target === event.currentTarget) onClose();
      }}
    >
      <div className="craber-picker-header">
        <FlowIcon size={14} />
        <span className="craber-picker-title">Workflows</span>
        <button type="button" className="craber-icon-button" title="Close" onClick={onClose}>
          <CloseIcon size={14} />
        </button>
      </div>

      <ul className="craber-picker-list">
        {workflows.map((workflow) => (
          <li key={workflow.name}>
            <button
              type="button"
              className={`craber-picker-item${workflow.name === current ? " is-current" : ""}`}
              onClick={() => onPick(workflow.name)}
            >
              <span className="craber-picker-name">{workflow.name}</span>
              <span className="craber-picker-kinds">
                {kinds(workflow).map((kind) => (
                  <span key={kind} className={`craber-picker-kind ${ACCESS_CLASS[KIND_ACCESS[kind]]}`}>
                    {KIND_LABEL[kind]}
                  </span>
                ))}
              </span>
              <span className="craber-picker-count">
                {workflow.steps.length} {workflow.steps.length === 1 ? "step" : "steps"}
              </span>
            </button>
          </li>
        ))}
      </ul>
    </dialog>
  );
}

/** The kinds of statement a workflow runs, each once, in the order they first appear. */
function kinds(workflow: Workflow): StepKind[] {
  return [...new Set(workflow.steps.map((step) => step.kind))];
}
