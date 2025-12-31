import htmx from "htmx.org";

console.log(`htmx ${htmx.version}`);
window.htmx = htmx;

declare global {
  interface Window {
    htmx: typeof htmx;
  }
}
