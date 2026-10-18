import { FitViewIcon, MinusIcon, PlusIcon, TidyIcon } from "./icons";

type ToolbarProps = {
  zoom: number;
  onZoomIn: () => void;
  onZoomOut: () => void;
  onFitView: () => void;
  onTidyUp: () => void;
};

/**
 * The floating controls: zoom and framing.
 *
 * There is no control for how much of a table to draw. Every column is always
 * on screen, because that is what the relationship lines attach to — an edge
 * runs from `orders.user_id` to `users.id`, and a collapsed node would have
 * nowhere to put either end.
 */
export function Toolbar({ zoom, onZoomIn, onZoomOut, onFitView, onTidyUp }: ToolbarProps) {
  return (
    <div className="craber-toolbar">
      <button type="button" className="craber-icon-button" title="Zoom out" onClick={onZoomOut}>
        <MinusIcon />
      </button>
      <span className="craber-zoom">{Math.round(zoom * 100)}%</span>
      <button type="button" className="craber-icon-button" title="Zoom in" onClick={onZoomIn}>
        <PlusIcon />
      </button>

      <span className="craber-toolbar-divider" />

      <button type="button" className="craber-icon-button" title="Fit view" onClick={onFitView}>
        <FitViewIcon />
      </button>
      <button
        type="button"
        className="craber-icon-button"
        title="Tidy up — put every table back where craber-layout placed it"
        onClick={onTidyUp}
      >
        <TidyIcon />
      </button>
    </div>
  );
}
