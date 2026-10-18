/**
 * Hand a file to the browser.
 *
 * A published diagram is static files on a host with no server behind it, so
 * an export is built in the page and handed over as an object URL. The URL is released a tick later rather than straight after the
 * click: revoking it in the same task can pull it out from under the download
 * that has only just been started.
 */
export function download(name: string, text: string, type = "application/sql") {
  const url = URL.createObjectURL(new Blob([text], { type }));
  const link = document.createElement("a");
  link.href = url;
  link.download = name;
  link.click();
  window.setTimeout(() => URL.revokeObjectURL(url), 0);
}
