import { useEffect, useState } from "react";

import { CheckIcon, DownloadIcon, SidebarIcon } from "./icons";

type TopBarProps = {
  schemaName: string;
  sidebarOpen: boolean;
  onToggleSidebar: () => void;
  onExportSql: () => void;
};

export function TopBar({ schemaName, sidebarOpen, onToggleSidebar, onExportSql }: TopBarProps) {
  const [exported, setExported] = useState(false);

  // A download starts in the browser's chrome, not on the page, so the button
  // says so itself — and clears on unmount, or React warns about the late
  // setState.
  useEffect(() => {
    if (!exported) return;
    const timer = window.setTimeout(() => setExported(false), 1600);
    return () => window.clearTimeout(timer);
  }, [exported]);

  return (
    <header className="craber-topbar">
      <button
        type="button"
        className={`craber-icon-button${sidebarOpen ? " is-active" : ""}`}
        title={sidebarOpen ? "Hide the table list" : "Show the table list"}
        onClick={onToggleSidebar}
      >
        <SidebarIcon />
      </button>

      <span className="craber-logo">
        Crab<span>ER</span>
      </span>
      <span className="craber-schema-name">{schemaName}</span>

      <div className="craber-topbar-spacer" />

      <button
        type="button"
        className="craber-primary-button"
        title="Download the table definitions as SQL"
        onClick={() => {
          onExportSql();
          setExported(true);
        }}
      >
        {exported ? <CheckIcon size={14} /> : <DownloadIcon size={14} />}
        {exported ? "Exported" : "Export SQL"}
      </button>
    </header>
  );
}
